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

/// A random stretch of life before a property is checked: gaps of up to ~11 h and button
/// masks each held across a 1.1 s window (long enough for the A+C tombstone restart to
/// complete), for up to 40 steps (~18 days). Random mashing reaches Baby -> Child at 1 h, the
/// menu and every action, death by neglect within the day, and restart; the well-cared-for
/// adult and old age are reached by the goldens and unit tests instead. Returns the `now_ms`
/// the prefix ended at.
fn lifecycle_prefix(cart: &mut Cart, start_ms: u64, steps: &[(u32, u8)]) -> u64 {
    let mut now = start_ms;
    for &(gap_secs, mask) in steps {
        now += gap_secs as u64 * 1000;
        cart.update(now, mask);
        now += 1100;
        cart.update(now, mask);
        now += 100;
        cart.update(now, 0);
    }
    now
}

fn prefix_steps() -> impl Strategy<Value = Vec<(u32, u8)>> {
    proptest::collection::vec((60u32..40_000, 0u8..8), 0..40)
}

proptest! {
    #![proptest_config(ProptestConfig::with_cases(64))]

    /// Advancing to `end_ms` in a single `update` must equal advancing through several random
    /// intermediate `update`s that land on the same `end_ms` (docs/DETERMINISM.md rule 3),
    /// from any point in a lifecycle.
    #[test]
    fn advance_invariance(
        seed in any::<u64>(),
        start_ms in START_MS_RANGE,
        steps in prefix_steps(),
        gap_secs in 1u64..400_000,
        mut fracs in proptest::collection::vec(0u64..1000, 0..8),
    ) {
        let mut a = Cart::new_uninit();
        a.reset(start_ms, seed);
        let from_ms = lifecycle_prefix(&mut a, start_ms, &steps);
        let end_ms = from_ms + gap_secs * 1000;
        a.update(end_ms, 0);

        let mut b = Cart::new_uninit();
        b.reset(start_ms, seed);
        lifecycle_prefix(&mut b, start_ms, &steps);
        fracs.sort_unstable();
        for frac in fracs {
            // Integer interpolation (no floats, matching docs/DETERMINISM.md rule 1 even though
            // this test crate isn't itself subject to vpet-core's `deny(float_arithmetic)`).
            let mid_ms = from_ms + (end_ms - from_ms) * frac / 1000;
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
        steps in prefix_steps(),
        gap_secs in 1u64..400_000,
        back_ms in 1u64..100_000,
    ) {
        let mut cart = Cart::new_uninit();
        cart.reset(start_ms, seed);
        let from_ms = lifecycle_prefix(&mut cart, start_ms, &steps);
        let reached_ms = from_ms + gap_secs * 1000;
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
        steps in prefix_steps(),
        gap_secs in 0u64..400_000,
        further_secs in 1u64..100_000,
        further_mask in 0u8..8,
    ) {
        let mut original = Cart::new_uninit();
        original.reset(start_ms, seed);
        let from_ms = lifecycle_prefix(&mut original, start_ms, &steps);
        let reached_ms = from_ms + gap_secs * 1000;
        original.update(reached_ms, 0);

        let bytes = save_bytes(&original);
        let mut reloaded = Cart::new_uninit();
        reloaded.load(&bytes).expect("a save this test just produced must load cleanly");

        prop_assert_eq!(save_bytes(&original), save_bytes(&reloaded));

        // Behaviour matches going forward from here too, input included.
        let further_ms = reached_ms + further_secs * 1000;
        original.update(further_ms, further_mask);
        reloaded.update(further_ms, further_mask);
        original.update(further_ms + 1100, further_mask);
        reloaded.update(further_ms + 1100, further_mask);
        prop_assert_eq!(save_bytes(&original), save_bytes(&reloaded));
    }

    /// The same press timestamps delivered at 5 Hz and 60 Hz must produce identical blobs
    /// (docs/DETERMINISM.md rule 7, docs/TESTING.md property 2). Each press here is held for a
    /// full second — comfortably longer than either poll interval — so both hosts are
    /// guaranteed to observe every rising edge; a press held for less than one poll interval
    /// could be missed by that particular host, which is a real, documented limitation of
    /// held-state polling (docs/HOST_ABI.md "Buttons"), not a determinism bug this property
    /// covers.
    ///
    /// Press windows are anchored to whole-second boundaries (`round_up_to_sec`). Inputs are
    /// applied at sim second `floor(now_ms / 1000)` (docs/HOST_ABI.md "Buttons"), so if a press
    /// window started at an arbitrary sub-second offset, two poll rates could observe the same
    /// held edge a few ms apart yet land on *different* floor-seconds whenever that offset sits
    /// within one poll interval of a second boundary — another real polling-granularity
    /// limitation, not a determinism bug, but one this property must route around (rather than
    /// stumble into at random) to test what it's actually meant to test.
    #[test]
    fn poll_rate_invariance(
        seed in any::<u64>(),
        start_ms in START_MS_RANGE,
        presses in proptest::collection::vec(1u8..8, 1..6),
    ) {
        fn round_up_to_sec(ms: u64) -> u64 {
            ms.div_ceil(1000) * 1000
        }

        fn run(start_ms: u64, seed: u64, presses: &[u8], poll_ms: u64) -> Vec<u8> {
            let mut cart = Cart::new_uninit();
            cart.reset(start_ms, seed);
            let hatch_ms = start_ms + 300_000;
            cart.update(hatch_ms, 0); // hatch, so buttons open the menu and do something

            // Round up to the next whole second: every offset added below (10_000, 1_000,
            // 2_000) is itself a multiple of 1000, so every press window boundary stays
            // second-aligned from here on, for every press in the sequence.
            let mut now = round_up_to_sec(hatch_ms);
            for &mask in presses {
                let press_start = now + 10_000;
                let press_end = press_start + 1_000;
                let settle = press_end + 2_000;
                let mut t = now;
                while t < settle {
                    let held = if t >= press_start && t < press_end { mask } else { 0 };
                    cart.update(t, held);
                    t += poll_ms;
                }
                cart.update(settle, 0);
                now = settle;
            }
            let mut scratch = [0u8; 512];
            let mut out = [0u8; 512];
            let len = cart.save(&mut scratch, &mut out).expect("save buffer too small");
            out[0..len].to_vec()
        }

        let slow = run(start_ms, seed, &presses, 200); // 5 Hz
        let fast = run(start_ms, seed, &presses, 17);  // ~60 Hz
        prop_assert_eq!(slow, fast);
    }
}
