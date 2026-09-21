//! The fixed timer table that drives event-stepped fast-forward. See docs/STATE_MODEL.md.
//! There is at most one pending instance of each `EventKind`; a fixed array needs no allocation
//! and a linear scan over 15 entries is trivial (docs/adr, no_std budget).

use crate::time::{Sec, NEVER};

/// Timer kinds, in the order used to break ties: at equal timestamps, the lowest-numbered kind
/// fires first. Order matches docs/STATE_MODEL.md exactly: schedule before lifecycle, lifecycle
/// before environment, environment before needs, needs before consequences, consequences before
/// UI.
#[repr(u8)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum EventKind {
    Wake = 0,
    Sleep = 1,
    Hatch = 2,
    Evolve = 3,
    OldAge = 4,
    Poop = 5,
    Tantrum = 6,
    SickOnset = 7,
    HungerEmpty = 8,
    HappyEmpty = 9,
    CareMiss = 10,
    SickDamage = 11,
    Starve = 12,
    UiBusyEnd = 13,
    MenuTimeout = 14,
}

pub const EVENT_COUNT: usize = 15;

const ALL_KINDS: [EventKind; EVENT_COUNT] = [
    EventKind::Wake,
    EventKind::Sleep,
    EventKind::Hatch,
    EventKind::Evolve,
    EventKind::OldAge,
    EventKind::Poop,
    EventKind::Tantrum,
    EventKind::SickOnset,
    EventKind::HungerEmpty,
    EventKind::HappyEmpty,
    EventKind::CareMiss,
    EventKind::SickDamage,
    EventKind::Starve,
    EventKind::UiBusyEnd,
    EventKind::MenuTimeout,
];

#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct Timers(#[serde(with = "serde_arrays")] [Sec; EVENT_COUNT]);

impl Timers {
    pub const fn new() -> Self {
        Timers([NEVER; EVENT_COUNT])
    }

    pub fn set(&mut self, k: EventKind, at: Sec) {
        self.0[k as usize] = at;
    }

    pub fn clear(&mut self, k: EventKind) {
        self.0[k as usize] = NEVER;
    }

    pub fn clear_all(&mut self) {
        self.0 = [NEVER; EVENT_COUNT];
    }

    pub fn get(&self, k: EventKind) -> Sec {
        self.0[k as usize]
    }

    /// The earliest pending timer, ties broken by `EventKind` order. `None` if every timer is
    /// `NEVER`.
    pub fn earliest(&self) -> Option<(Sec, EventKind)> {
        let mut best: Option<(Sec, EventKind)> = None;
        for &k in ALL_KINDS.iter() {
            let t = self.0[k as usize];
            if t == NEVER {
                continue;
            }
            match best {
                None => best = Some((t, k)),
                Some((bt, _)) if t < bt => best = Some((t, k)),
                _ => {}
            }
        }
        best
    }

    /// Raw backing array, for the save layout (`docs/SAVE_FORMAT.md`: `timers: [u32; 15]`).
    pub const fn as_array(&self) -> [Sec; EVENT_COUNT] {
        self.0
    }

    pub const fn from_array(a: [Sec; EVENT_COUNT]) -> Self {
        Timers(a)
    }
}

impl Default for Timers {
    fn default() -> Self {
        Self::new()
    }
}

/// `serde` has no built-in support for arrays longer than 32 elements without the `serde_arrays`
/// crate; rather than pull a dependency for 15 elements, hand-roll the (de)serializer as a
/// fixed-size tuple-like sequence. Kept private to this module.
mod serde_arrays {
    use super::EVENT_COUNT;
    use crate::time::Sec;
    use serde::de::{SeqAccess, Visitor};
    use serde::ser::SerializeTuple;
    use serde::{Deserializer, Serializer};

    pub fn serialize<S: Serializer>(arr: &[Sec; EVENT_COUNT], s: S) -> Result<S::Ok, S::Error> {
        let mut tup = s.serialize_tuple(EVENT_COUNT)?;
        for v in arr {
            tup.serialize_element(v)?;
        }
        tup.end()
    }

    struct ArrVisitor;
    impl<'de> Visitor<'de> for ArrVisitor {
        type Value = [Sec; EVENT_COUNT];
        fn expecting(&self, f: &mut core::fmt::Formatter) -> core::fmt::Result {
            write!(f, "an array of {EVENT_COUNT} u32 values")
        }
        fn visit_seq<A>(self, mut seq: A) -> Result<Self::Value, A::Error>
        where
            A: SeqAccess<'de>,
        {
            let mut out = [0u32; EVENT_COUNT];
            for (i, slot) in out.iter_mut().enumerate() {
                *slot = seq
                    .next_element()?
                    .ok_or_else(|| serde::de::Error::invalid_length(i, &self))?;
            }
            Ok(out)
        }
    }

    pub fn deserialize<'de, D: Deserializer<'de>>(d: D) -> Result<[Sec; EVENT_COUNT], D::Error> {
        d.deserialize_tuple(EVENT_COUNT, ArrVisitor)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn empty_timers_have_no_earliest() {
        let t = Timers::new();
        assert_eq!(t.earliest(), None);
    }

    #[test]
    fn earliest_picks_lowest_time() {
        let mut t = Timers::new();
        t.set(EventKind::Starve, 500);
        t.set(EventKind::Hatch, 100);
        t.set(EventKind::Poop, 300);
        assert_eq!(t.earliest(), Some((100, EventKind::Hatch)));
    }

    #[test]
    fn ties_break_by_enum_order() {
        let mut t = Timers::new();
        // Starve (12) and Hatch (2) at the same instant: Hatch must win.
        t.set(EventKind::Starve, 100);
        t.set(EventKind::Hatch, 100);
        assert_eq!(t.earliest(), Some((100, EventKind::Hatch)));
    }

    #[test]
    fn clear_removes_a_timer() {
        let mut t = Timers::new();
        t.set(EventKind::Hatch, 100);
        t.clear(EventKind::Hatch);
        assert_eq!(t.get(EventKind::Hatch), NEVER);
        assert_eq!(t.earliest(), None);
    }

    #[test]
    fn clear_all_resets_everything() {
        let mut t = Timers::new();
        t.set(EventKind::Hatch, 100);
        t.set(EventKind::Poop, 200);
        t.clear_all();
        assert_eq!(t.earliest(), None);
    }

    #[test]
    fn round_trips_through_postcard() {
        let mut t = Timers::new();
        t.set(EventKind::Hatch, 100);
        t.set(EventKind::MenuTimeout, 42);
        let mut buf = [0u8; 128];
        let used = postcard::to_slice(&t, &mut buf).unwrap();
        let back: Timers = postcard::from_bytes(used).unwrap();
        assert_eq!(t, back);
    }
}
