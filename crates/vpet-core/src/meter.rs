//! A 0..=100 stat that moves one point every `step_secs`, in direction `dir`. See
//! docs/STATE_MODEL.md. Integer-only: no fractional arithmetic, ever
//! (docs/DETERMINISM.md rule 1).

use crate::time::Sec;

/// A meter's direction of travel: `+1` (e.g. hunger, rising toward starving) or `-1` (e.g.
/// happiness, falling).
pub type Dir = i8;

#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct Meter {
    v0: u8,
    t0: Sec,
    step_secs: u16,
    dir: Dir,
    paused: bool,
}

impl Meter {
    /// A new meter anchored at `(t0, v0)`, moving `dir` one point every `step_secs`.
    /// `step_secs == 0` is treated as "never moves" (guards a divide by zero; a species that
    /// wants a frozen meter sets 0 deliberately, distinct from `paused` which is a runtime
    /// state).
    pub const fn new(v0: u8, t0: Sec, step_secs: u16, dir: Dir) -> Self {
        Meter {
            v0: clamp100(v0),
            t0,
            step_secs,
            dir,
            paused: false,
        }
    }

    pub const fn is_paused(&self) -> bool {
        self.paused
    }

    pub const fn step_secs(&self) -> u16 {
        self.step_secs
    }

    /// The meter's value at simulated time `t`. `t` before `t0` clamps to `v0` (meters are only
    /// ever evaluated at or after their anchor time in practice, but this keeps the function
    /// total rather than requiring the caller to prove `t >= t0`).
    pub fn value_at(&self, t: Sec) -> u8 {
        if self.paused || self.step_secs == 0 {
            return self.v0;
        }
        let elapsed = t.saturating_sub(self.t0) as u64;
        let steps = elapsed / self.step_secs as u64;
        let delta = (steps.min(i32::MAX as u64) as i64) * self.dir as i64;
        clamp100_i64(self.v0 as i64 + delta)
    }

    /// The first simulated time at which this meter reaches `limit`, moving in its own
    /// direction. `NEVER` if it will not (paused, frozen, already past `limit` moving away from
    /// it, or `limit` is not between `v0` and 0/100 in the direction of travel).
    pub fn reaches(&self, limit: u8) -> Sec {
        if self.paused || self.step_secs == 0 || self.dir == 0 {
            return crate::time::NEVER;
        }
        let limit = limit.min(100) as i32;
        let v0 = self.v0 as i32;
        // Distance to travel in the meter's own direction; negative means `limit` is behind us.
        let distance = if self.dir > 0 { limit - v0 } else { v0 - limit };
        if distance <= 0 {
            // Already at or past the limit in the direction of travel: "reaches" it now.
            return self.t0;
        }
        let steps = distance as u64;
        let secs = steps.saturating_mul(self.step_secs as u64);
        self.t0.saturating_add(secs.min(u32::MAX as u64) as u32)
    }

    /// Re-anchor the meter to value `v` at time `t`, keeping `step_secs`/`dir`/`paused`
    /// unchanged. Used on stage change (with a fresh `step_secs` supplied by the caller via
    /// `reanchor_with`) and after an action that jumps the value.
    pub fn reanchor(&mut self, t: Sec, v: u8) {
        self.v0 = clamp100(v);
        self.t0 = t;
    }

    /// Re-anchor and change `step_secs` at once (used on a stage transition).
    pub fn reanchor_with_step(&mut self, t: Sec, v: u8, step_secs: u16) {
        self.reanchor(t, v);
        self.step_secs = step_secs;
    }

    pub fn pause(&mut self, t: Sec) {
        let v = self.value_at(t);
        self.reanchor(t, v);
        self.paused = true;
    }

    pub fn resume(&mut self, t: Sec) {
        self.t0 = t;
        self.paused = false;
    }
}

const fn clamp100(v: u8) -> u8 {
    if v > 100 {
        100
    } else {
        v
    }
}

fn clamp100_i64(v: i64) -> u8 {
    if v < 0 {
        0
    } else if v > 100 {
        100
    } else {
        v as u8
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn value_at_rises() {
        let m = Meter::new(0, 100, 10, 1); // +1 every 10s from t=100
        assert_eq!(m.value_at(100), 0);
        assert_eq!(m.value_at(109), 0);
        assert_eq!(m.value_at(110), 1);
        assert_eq!(m.value_at(200), 10);
    }

    #[test]
    fn value_at_falls_and_clamps_at_zero() {
        let m = Meter::new(5, 0, 10, -1);
        assert_eq!(m.value_at(0), 5);
        assert_eq!(m.value_at(50), 0);
        assert_eq!(m.value_at(1000), 0); // clamped, not negative/wrapped
    }

    #[test]
    fn value_at_clamps_at_100() {
        let m = Meter::new(95, 0, 1, 1);
        assert_eq!(m.value_at(1000), 100);
    }

    #[test]
    fn paused_freezes_value() {
        let mut m = Meter::new(50, 0, 10, 1);
        m.pause(30);
        assert_eq!(m.value_at(30), 53); // re-anchored to the value at pause time
        assert_eq!(m.value_at(1000), 53); // frozen thereafter
    }

    #[test]
    fn resume_continues_from_current_anchor() {
        let mut m = Meter::new(50, 0, 10, 1);
        m.pause(30); // v0 = 53, t0 = 30
        m.resume(100); // t0 moves to 100, value stays 53 until it steps again
        assert_eq!(m.value_at(100), 53);
        assert_eq!(m.value_at(110), 54);
    }

    #[test]
    fn step_secs_zero_is_frozen() {
        let m = Meter::new(42, 0, 0, 1);
        assert_eq!(m.value_at(1_000_000), 42);
        assert_eq!(m.reaches(80), crate::time::NEVER);
    }

    #[test]
    fn reaches_rising() {
        let m = Meter::new(0, 100, 10, 1);
        assert_eq!(m.reaches(5), 150);
        assert_eq!(m.reaches(0), 100); // already there
    }

    #[test]
    fn reaches_falling() {
        let m = Meter::new(100, 0, 10, -1);
        assert_eq!(m.reaches(20), 800);
    }

    #[test]
    fn reaches_wrong_direction_is_never() {
        let m = Meter::new(0, 100, 10, 1); // rising
        assert_eq!(m.reaches(0), 100); // at limit already, not "never"
        let m2 = Meter::new(50, 100, 10, 1); // rising, limit behind us
        assert_eq!(m2.reaches(10), 100);
    }

    #[test]
    fn reaches_never_while_paused() {
        let mut m = Meter::new(0, 0, 10, 1);
        m.pause(0);
        assert_eq!(m.reaches(80), crate::time::NEVER);
    }

    #[test]
    fn new_clamps_initial_value() {
        let m = Meter::new(255, 0, 10, 1);
        assert_eq!(m.value_at(0), 100);
    }
}
