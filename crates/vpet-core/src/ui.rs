//! `Ui`: what the screen is currently showing, distinct from `Pet`'s simulation state. See
//! docs/STATE_MODEL.md. Persisted so a save mid-animation reloads invisibly.
//!
//! Phase 1 only drives `Idle` and `Menu`; the other variants are declared now (matching the
//! documented shape exactly) so `Cart`'s field types don't change shape again in Phase 3 when
//! their handlers are filled in.

#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum BusyKind {
    Eating,
    Refuse,
    Playing { seq: u8, round: u8, correct: u8 },
    Discipline,
    Evolving,
    Result,
}

#[derive(Debug, Default, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum Ui {
    #[default]
    Idle,
    Menu {
        cursor: u8,
    },
    FeedSub {
        snack: bool,
    },
    Status {
        page: u8,
    },
    Busy {
        kind: BusyKind,
    },
    Battle {
        phase: u8,
        #[serde(with = "turns6")]
        turns: [u8; 6],
        hp_me: u8,
        hp_them: u8,
    },
}

/// `serde`'s derive supports const-generic arrays natively for common sizes, but a hand-rolled
/// (de)serializer keeps this module dependency-free and matches the approach already used for
/// `Timers`'s 15-element array (see `timers.rs`).
mod turns6 {
    use serde::de::{SeqAccess, Visitor};
    use serde::ser::SerializeTuple;
    use serde::{Deserializer, Serializer};

    const N: usize = 6;

    pub fn serialize<S: Serializer>(arr: &[u8; N], s: S) -> Result<S::Ok, S::Error> {
        let mut tup = s.serialize_tuple(N)?;
        for v in arr {
            tup.serialize_element(v)?;
        }
        tup.end()
    }

    struct ArrVisitor;
    impl<'de> Visitor<'de> for ArrVisitor {
        type Value = [u8; N];
        fn expecting(&self, f: &mut core::fmt::Formatter) -> core::fmt::Result {
            write!(f, "an array of {N} u8 values")
        }
        fn visit_seq<A>(self, mut seq: A) -> Result<Self::Value, A::Error>
        where
            A: SeqAccess<'de>,
        {
            let mut out = [0u8; N];
            for (i, slot) in out.iter_mut().enumerate() {
                *slot = seq
                    .next_element()?
                    .ok_or_else(|| serde::de::Error::invalid_length(i, &self))?;
            }
            Ok(out)
        }
    }

    pub fn deserialize<'de, D: Deserializer<'de>>(d: D) -> Result<[u8; N], D::Error> {
        d.deserialize_tuple(N, ArrVisitor)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn round_trips_through_postcard() {
        let cases = [
            Ui::Idle,
            Ui::Menu { cursor: 3 },
            Ui::Busy {
                kind: BusyKind::Playing {
                    seq: 1,
                    round: 2,
                    correct: 3,
                },
            },
            Ui::Battle {
                phase: 1,
                turns: [1, 2, 3, 4, 5, 6],
                hp_me: 10,
                hp_them: 8,
            },
        ];
        for ui in cases {
            let mut buf = [0u8; 64];
            let used = postcard::to_slice(&ui, &mut buf).unwrap();
            let back: Ui = postcard::from_bytes(used).unwrap();
            assert_eq!(ui, back);
        }
    }
}
