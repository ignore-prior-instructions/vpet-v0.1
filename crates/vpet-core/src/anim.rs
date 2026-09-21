//! Procedural animation operators: pure functions of `(tick, anim_rng)`, never of a previous
//! frame. See docs/art/ANIMATION.md. Rendering the same tick twice must give the same frame,
//! and rendering ticks out of order must be harmless — there is no per-frame mutable animation
//! state, only `AnimState`, which changes on `Ui`/activity transitions, not every tick.
//!
//! Phase 1 implements the operators the Egg/Idle/Menu scenes need: `toggle` (2-pose
//! alternation), `hop`, `blink`, and `walk`. `shake`, `flip_h`, and the clip-length reaction
//! system join in Phase 3 with the rest of the clip table.

/// Transient per-`Cart` animation bookkeeping. Not persisted (docs/STATE_MODEL.md: `AnimState`
/// is derived from `Ui`/`Pet`/`now_ms`, recomputed on load).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct AnimState {
    pub clip: ClipId,
    pub clip_start_tick: u32,
    /// Reseeded from `(sim_now, rng[0])` whenever the clip changes (docs/art/ANIMATION.md).
    pub anim_rng: u32,
}

impl AnimState {
    pub const fn new() -> Self {
        AnimState {
            clip: ClipId::Egg,
            clip_start_tick: 0,
            anim_rng: 0,
        }
    }

    pub fn on_state_change(&mut self, clip: ClipId, tick: u32, sim_now: u32, rng0: u32) {
        self.clip = clip;
        self.clip_start_tick = tick;
        self.anim_rng = mix(sim_now, rng0);
    }
}

impl Default for AnimState {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ClipId {
    Egg,
    Main, // idle/dirty
    Sick,
    Sleeping,
    Eating,
    Refuse,
    DisciplineBusy,
    Result,
    Playing,
    Menu,
    FeedSub,
    Status,
    Dead,
}

/// A cheap, well-mixed integer hash (Murmur3's 32-bit finalizer), used to turn `(seed, index)`
/// pairs into pseudo-random values without keeping sequential RNG state between frames.
pub fn mix(seed: u32, x: u32) -> u32 {
    let mut z = seed.wrapping_add(x.wrapping_mul(0x9E37_79B9));
    z ^= z >> 16;
    z = z.wrapping_mul(0x85EB_CA6B);
    z ^= z >> 13;
    z = z.wrapping_mul(0xC2B2_AE35);
    z ^ (z >> 16)
}

/// `toggle`: alternate two poses every `period` ticks, counting from `elapsed` ticks into the
/// clip. Returns `false` for the first pose, `true` for the second.
pub fn toggle(elapsed_ticks: u32, period: u32) -> bool {
    (elapsed_ticks / period.max(1)) % 2 == 1
}

/// `hop`: vertical offset for a 4-tick hop cycle, `[0, -1, -2, -1]`.
pub fn hop_dy(elapsed_ticks: u32) -> i32 {
    const TABLE: [i32; 4] = [0, -1, -2, -1];
    TABLE[(elapsed_ticks % 4) as usize]
}

/// `shake`: horizontal offset for a 4-tick shake cycle, `[-1, 1, -1, 0]` (Refuse, Discipline).
pub fn shake_dx(elapsed_ticks: u32) -> i32 {
    const TABLE: [i32; 4] = [-1, 1, -1, 0];
    TABLE[(elapsed_ticks % 4) as usize]
}

/// `blink`: does tick `elapsed_ticks` (since the clip started) fall on a blink? Approximates
/// docs/art/ANIMATION.md's sequential `next_blink = tick + 12 + (rng % 16)` schedule with a
/// stateless one: each non-overlapping 16-tick window has exactly one hashed blink tick,
/// spacing blinks 1 to 31 ticks apart depending on adjacent windows' offsets — close enough for
/// a placeholder pose, and it costs O(1) instead of replaying history.
pub fn is_blink(anim_rng: u32, elapsed_ticks: u32) -> bool {
    const WINDOW: u32 = 16;
    let window_index = elapsed_ticks / WINDOW;
    let offset = mix(anim_rng, window_index) % WINDOW;
    elapsed_ticks % WINDOW == offset
}

/// `walk`: deterministic position at `elapsed_ticks` into the clip, within `[lo, hi]`, starting
/// at `home` and stepping every 2 ticks with a 15% chance per step (hashed, not drawn from the
/// simulation RNG) to flip direction or pause 4 to 12 ticks. Recomputed by replaying from the
/// clip start each call (never stored between frames), which is what makes rendering the same
/// tick twice, or ticks out of order, give identical results. `elapsed_ticks` is folded into a
/// bounded window first so a multi-year gap still costs O(1) rather than replaying years of
/// steps (docs/DETERMINISM.md rule 8: no unbounded work under an adversarial gap); the walk
/// pattern simply repeats every `REPLAY_PERIOD` ticks of clip time, which is inaudible/invisible
/// for an idle animation.
pub fn walk_position(anim_rng: u32, elapsed_ticks: u32, lo: i8, hi: i8, home: i8) -> i8 {
    const REPLAY_PERIOD: u32 = 20_000; // ~83 minutes of idle-clip time at ANIM_HZ=4
    let bounded = elapsed_ticks % REPLAY_PERIOD;

    let mut x = home;
    let mut dir: i8 = 1;
    let mut pause_until: u32 = 0;
    let mut t: u32 = 0;
    while t < bounded {
        if t >= pause_until && t.is_multiple_of(2) {
            let roll = mix(anim_rng, t) % 100;
            if roll < 15 {
                if mix(anim_rng, t ^ 0x9E37_79B9).is_multiple_of(2) {
                    dir = -dir;
                } else {
                    let pause_len = 4 + (mix(anim_rng, t ^ 0xDEAD_BEEF) % 9);
                    pause_until = t + pause_len;
                }
            } else {
                x = (x + dir).clamp(lo, hi);
                if x <= lo {
                    dir = 1;
                }
                if x >= hi {
                    dir = -1;
                }
            }
        }
        t += 1;
    }
    x
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn toggle_alternates() {
        assert!(!toggle(0, 2));
        assert!(!toggle(1, 2));
        assert!(toggle(2, 2));
        assert!(toggle(3, 2));
        assert!(!toggle(4, 2));
    }

    #[test]
    fn hop_matches_table() {
        assert_eq!(hop_dy(0), 0);
        assert_eq!(hop_dy(1), -1);
        assert_eq!(hop_dy(2), -2);
        assert_eq!(hop_dy(3), -1);
        assert_eq!(hop_dy(4), 0);
    }

    #[test]
    fn shake_matches_table() {
        assert_eq!(shake_dx(0), -1);
        assert_eq!(shake_dx(1), 1);
        assert_eq!(shake_dx(2), -1);
        assert_eq!(shake_dx(3), 0);
        assert_eq!(shake_dx(4), -1);
    }

    #[test]
    fn blink_is_rare_and_deterministic() {
        let count = (0..1000).filter(|&t| is_blink(42, t)).count();
        // One blink per 16-tick window: exactly 1000/16 windows worth, +/- boundary effects.
        assert!((50..=70).contains(&count), "count={count}");
        // Same tick, same answer, called twice.
        assert_eq!(is_blink(42, 500), is_blink(42, 500));
    }

    #[test]
    fn walk_stays_in_bounds() {
        for seed in 0..20u32 {
            for t in (0..5000u32).step_by(37) {
                let x = walk_position(seed, t, 1, 15, 8);
                assert!((1..=15).contains(&x), "seed={seed} t={t} x={x}");
            }
        }
    }

    #[test]
    fn walk_is_pure_and_order_independent() {
        let a = walk_position(7, 1234, 1, 15, 8);
        let b = walk_position(7, 1234, 1, 15, 8);
        assert_eq!(a, b);
        // Order independence: computing t=500 then t=200 gives the same t=200 result as
        // computing t=200 directly (no hidden state carried from the t=500 call).
        let _ = walk_position(7, 500, 1, 15, 8);
        let c = walk_position(7, 200, 1, 15, 8);
        let d = walk_position(7, 200, 1, 15, 8);
        assert_eq!(c, d);
    }

    #[test]
    fn walk_bounded_under_huge_gaps() {
        // A ten-year gap in ticks must not hang or overflow: ~10 years * 365d * 86400s * 4 =
        // well within u32, and the function must return promptly.
        let ten_years_ticks: u32 = 10 * 365 * 86_400 * 4;
        let x = walk_position(1, ten_years_ticks, 1, 15, 8);
        assert!((1..=15).contains(&x));
    }
}
