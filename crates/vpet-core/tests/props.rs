//! Property tests for the determinism invariants in docs/DETERMINISM.md. Phase 1 covers rules
//! 3 (advance invariance), 4 (time reversal is a no-op), and 8 (no panics, including under
//! multi-year gaps); poll-rate invariance and save/load transparency round out
//! docs/TESTING.md's list once actions exist to make poll-rate timing actually matter (Phase 3).

use proptest::prelude::*;
use vpet_core::Cart;

const START_MS_RANGE: core::ops::Range<u64> = 1_600_000_000_000..1_800_000_000_000;
const TEN_YEARS_SECS: u64 = 10 * 365 * 86_400;

fn save_bytes(cart: &Cart) -> Vec<u8> {
    let mut scratch = [0u8; 512];
    let mut out = [0u8; 512];
    let len = cart
        .save(&mut scratch, &mut out)
        .expect("save buffer too small");
    out[0..len].to_vec()
}

proptest! {
    #![proptest_config(ProptestConfig::with_cases(64))]

    /// Advancing to `end_ms` in a single `update` must equal advancing through several random
    /// intermediate `update`s that land on the same `end_ms` (docs/DETERMINISM.md rule 3).
    #[test]
    fn advance_invariance(
        seed in any::<u64>(),
        start_ms in START_MS_RANGE,
        gap_secs in 1u64..400_000,
        mut fracs in proptest::collection::vec(0u64..1000, 0..8),
    ) {
        let end_ms = start_ms + gap_secs * 1000;

        let mut a = Cart::new_uninit();
        a.reset(start_ms, seed);
        a.update(end_ms, 0);

        let mut b = Cart::new_uninit();
        b.reset(start_ms, seed);
        fracs.sort_unstable();
        for frac in fracs {
            // Integer interpolation (no floats, matching docs/DETERMINISM.md rule 1 even though
            // this test crate isn't itself subject to vpet-core's `deny(float_arithmetic)`).
            let mid_ms = start_ms + (end_ms - start_ms) * frac / 1000;
            b.update(mid_ms, 0);
        }
        b.update(end_ms, 0);

        prop_assert_eq!(save_bytes(&a), save_bytes(&b));
    }

    /// An `update` with `now_ms` earlier than the current `sim_now` must change nothing but the
    /// animation phase (docs/DETERMINISM.md rule 4). `save()` never includes animation state, so
    /// comparing saved bytes before and after is a direct check.
    #[test]
    fn time_reversal_is_a_noop(
        seed in any::<u64>(),
        start_ms in START_MS_RANGE,
        gap_secs in 1u64..400_000,
        back_ms in 1u64..100_000,
    ) {
        let mut cart = Cart::new_uninit();
        cart.reset(start_ms, seed);
        let reached_ms = start_ms + gap_secs * 1000;
        cart.update(reached_ms, 0);
        let before = save_bytes(&cart);

        let earlier = reached_ms.saturating_sub(back_ms);
        cart.update(earlier, 0);
        let after = save_bytes(&cart);

        prop_assert_eq!(before, after);
    }

    /// No panics under random button mashing and gaps up to ten years (docs/DETERMINISM.md rule
    /// 8). Success is simply not panicking; proptest shrinks any failing case automatically.
    #[test]
    fn no_panics_under_random_input_and_huge_gaps(
        seed in any::<u64>(),
        start_ms in START_MS_RANGE,
        gap_secs in 0u64..TEN_YEARS_SECS,
        button_presses in proptest::collection::vec(0u8..8, 0..20),
    ) {
        let mut cart = Cart::new_uninit();
        cart.reset(start_ms, seed);
        let mut now_ms = start_ms;
        let step_ms = (gap_secs * 1000 / (button_presses.len() as u64 + 1)).max(1);
        for buttons in button_presses {
            now_ms = now_ms.saturating_add(step_ms);
            cart.update(now_ms, buttons);
        }
        cart.update(start_ms.saturating_add(gap_secs.saturating_mul(1000)), 0);
        let _ = cart.inspect();
        let _ = save_bytes(&cart);
    }

    /// Save/load transparency (docs/TESTING.md property 3): `load(save(x))` behaves the same as
    /// `x` going forward, for a variety of reached states.
    #[test]
    fn save_load_transparency(
        seed in any::<u64>(),
        start_ms in START_MS_RANGE,
        gap_secs in 0u64..400_000,
        further_secs in 1u64..100_000,
    ) {
        let mut original = Cart::new_uninit();
        original.reset(start_ms, seed);
        original.update(start_ms + gap_secs * 1000, 0);

        let bytes = save_bytes(&original);
        let mut reloaded = Cart::new_uninit();
        reloaded.load(&bytes).expect("a save this test just produced must load cleanly");

        prop_assert_eq!(save_bytes(&original), save_bytes(&reloaded));

        // Behaviour matches going forward from here too.
        let further_ms = start_ms + gap_secs * 1000 + further_secs * 1000;
        original.update(further_ms, 0);
        reloaded.update(further_ms, 0);
        prop_assert_eq!(save_bytes(&original), save_bytes(&reloaded));
    }
}
