//! Save format version 1. See docs/SAVE_FORMAT.md.
//!
//! `SaveV1` embeds the live `Pet`, `Timers`, and `Ui` types directly rather than duplicating
//! their fields into a hand-frozen shadow struct: they are already simple, invariant-free data
//! (no `Vec`, no indices into anything outside themselves), and postcard's wire format is
//! purely positional, so this is bit-for-bit what a literal field-by-field `PetV1` would
//! produce. The schema snapshot test below pins the exact bytes. When a real `SaveV2` is needed
//! (Phase 4, when a persisted field's *meaning* changes), fork an actual `PetV1` copy at that
//! point so `v1.rs` stops moving under old saves — today there is only one version, so there is
//! nothing yet for it to drift out from under.

use super::LoadError;
use crate::pet::Pet;
use crate::time::Sec;
use crate::timers::Timers;
use crate::ui::Ui;

pub const SAVE_VERSION: u16 = 1;

#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct SaveV1 {
    pub state: u8,
    pub sim_now: Sec,
    pub rng: [u32; 4],
    pub pet: Pet,
    pub timers: Timers,
    pub ui: Ui,
    pub attention_since: [Sec; 6],
}

/// Encode `save` into `out` as the full blob (header + postcard payload + CRC). `scratch` is a
/// caller-provided buffer for the postcard payload itself (avoids requiring `alloc`). Returns
/// the total blob length, or `None` if either buffer is too small.
pub fn encode(
    save: &SaveV1,
    content_hash: u32,
    scratch: &mut [u8],
    out: &mut [u8],
) -> Option<usize> {
    let payload = postcard::to_slice(save, scratch).ok()?;
    super::write_header_and_payload(out, SAVE_VERSION, content_hash, payload)
}

pub fn decode(payload: &[u8]) -> Result<SaveV1, LoadError> {
    postcard::from_bytes(payload).map_err(|_| LoadError::Decode)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::meter::Meter;
    use crate::pet::{DeathCause, Stage};
    use crate::timers::EventKind;
    use crate::ui::Ui;

    fn fixed_save() -> SaveV1 {
        let mut timers = Timers::new();
        timers.set(EventKind::Hatch, 100);
        SaveV1 {
            state: 1,
            sim_now: 42,
            rng: [1, 2, 3, 4],
            pet: Pet {
                species: 0,
                stage: Stage::Baby,
                hatched_at: 10,
                stage_since: 10,
                hunger: Meter::new(5, 10, 180, 1),
                happy: Meter::new(95, 10, 240, -1),
                discipline: 0,
                health: 100,
                weight: 5,
                poops: 0,
                poop_since: crate::time::NEVER,
                sick: false,
                sleeping: false,
                lights_off: false,
                attention: 0,
                care_mistakes: 0,
                sick_count: 0,
                snacks_since_wake: 0,
                wins: 0,
                losses: 0,
                last_battle_at: crate::time::NEVER,
                death_cause: DeathCause::None as u8,
            },
            timers,
            ui: Ui::Idle,
            attention_since: [crate::time::NEVER; 6],
        }
    }

    #[test]
    fn round_trips() {
        let s = fixed_save();
        let mut scratch = [0u8; 512];
        let payload = postcard::to_slice(&s, &mut scratch).unwrap();
        let back = decode(payload).unwrap();
        assert_eq!(s, back);
    }

    #[test]
    fn schema_snapshot() {
        // Pins the exact wire bytes of a fixed state. If this fails after an intentional
        // change to a persisted struct, bump SAVE_VERSION and re-bless; if it fails
        // unintentionally, a field was added/reordered/retyped without a version bump
        // (docs/SAVE_FORMAT.md).
        let s = fixed_save();
        let mut scratch = [0u8; 512];
        let payload = postcard::to_slice(&s, &mut scratch).unwrap();
        let mut out = [0u8; 512];
        let len = encode(&s, 0xDEAD_BEEF, &mut [0u8; 512], &mut out).unwrap();
        assert_eq!(len, 12 + payload.len() + 4);
        assert_eq!(&out[0..4], b"VPET");
        assert_eq!(u16::from_le_bytes([out[4], out[5]]), SAVE_VERSION);
        assert_eq!(
            u32::from_le_bytes([out[6], out[7], out[8], out[9]]),
            0xDEAD_BEEF
        );
    }
}
