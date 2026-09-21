//! Simulated time. See docs/DETERMINISM.md rule 2 and docs/STATE_MODEL.md.

/// Simulated time: epoch seconds. The core never reads a clock; this always comes from a host's
/// `now_ms` argument or from values already stored in the save.
pub type Sec = u32;

/// Sentinel for "no timer pending" / "no event ever".
pub const NEVER: Sec = u32::MAX;

/// Animation frames render at 4 Hz (one tick = 250 ms). Sub-second time only affects rendering,
/// never simulation state (docs/DETERMINISM.md rule 2).
pub const ANIM_HZ: u32 = 4;

/// `now_ms / 1000`, saturating. This is the simulated second a host's `now_ms` falls in.
pub fn ms_to_sec(now_ms: u64) -> Sec {
    let secs = now_ms / 1000;
    if secs > Sec::MAX as u64 {
        Sec::MAX
    } else {
        secs as Sec
    }
}

/// The animation tick a host's `now_ms` falls in: `now_ms / 250`.
pub fn tick(now_ms: u64) -> u32 {
    let t = now_ms / (1000 / ANIM_HZ as u64);
    if t > u32::MAX as u64 {
        u32::MAX
    } else {
        t as u32
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ms_to_sec_basic() {
        assert_eq!(ms_to_sec(0), 0);
        assert_eq!(ms_to_sec(999), 0);
        assert_eq!(ms_to_sec(1000), 1);
        assert_eq!(ms_to_sec(1_001), 1);
        assert_eq!(ms_to_sec(1_700_000_000_000), 1_700_000_000);
    }

    #[test]
    fn ms_to_sec_saturates_at_u32_max() {
        // (u32::MAX as u64 + 1) * 1000 overflows Sec when converted; must saturate, not panic
        // or wrap.
        let huge = (Sec::MAX as u64 + 10) * 1000;
        assert_eq!(ms_to_sec(huge), Sec::MAX);
    }

    #[test]
    fn tick_basic() {
        assert_eq!(tick(0), 0);
        assert_eq!(tick(249), 0);
        assert_eq!(tick(250), 1);
        assert_eq!(tick(1000), 4);
    }
}
