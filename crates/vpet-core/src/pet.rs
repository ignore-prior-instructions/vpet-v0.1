//! `Pet`: everything about the creature itself, as opposed to `Cart`'s bookkeeping (rng,
//! timers, ui). See docs/STATE_MODEL.md and docs/GAME_DESIGN.md.

use crate::meter::Meter;
use crate::time::{Sec, NEVER};

/// Lifecycle stage. Numeric values match the `Inspect` ABI layout in docs/HOST_ABI.md
/// (`0 Egg, 1 Baby, 2 Child, 3 Adult, 4 AdultAlt`).
#[repr(u8)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum Stage {
    Egg = 0,
    Baby = 1,
    Child = 2,
    Adult = 3,
    AdultAlt = 4,
}

impl Stage {
    pub const fn as_u8(self) -> u8 {
        self as u8
    }

    pub const fn from_u8(v: u8) -> Option<Self> {
        match v {
            0 => Some(Stage::Egg),
            1 => Some(Stage::Baby),
            2 => Some(Stage::Child),
            3 => Some(Stage::Adult),
            4 => Some(Stage::AdultAlt),
            _ => None,
        }
    }
}

/// Attention bits, per docs/GAME_DESIGN.md's "Attention calls and care mistakes" table.
pub mod attention {
    pub const HUNGRY: u8 = 1 << 0;
    pub const SAD: u8 = 1 << 1;
    pub const POOP: u8 = 1 << 2;
    pub const SICK: u8 = 1 << 3;
    pub const SLEEPY: u8 = 1 << 4;
    pub const TANTRUM: u8 = 1 << 5;
    /// Index into `attention_since` for each bit, matching the order the bits are declared in
    /// here (0 = hungry, 1 = sad, 2 = poop, 3 = sick, 4 = sleepy, 5 = tantrum).
    pub const COUNT: usize = 6;
    pub const ALL: [u8; COUNT] = [HUNGRY, SAD, POOP, SICK, SLEEPY, TANTRUM];

    pub const fn index_of(bit: u8) -> usize {
        match bit {
            HUNGRY => 0,
            SAD => 1,
            POOP => 2,
            SICK => 3,
            SLEEPY => 4,
            TANTRUM => 5,
            _ => 0, // unreachable for valid single-bit inputs; kept total for a const fn
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum DeathCause {
    None = 0,
    Starvation = 1,
    Sickness = 2,
    OldAge = 3,
}

#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct Pet {
    pub species: u8,
    pub stage: Stage,
    pub hatched_at: Sec,
    pub stage_since: Sec,
    pub hunger: Meter,
    pub happy: Meter,
    pub discipline: u8,
    pub health: u8,
    pub weight: u16,
    /// 0..=3, per docs/GAME_DESIGN.md `max_poops`.
    pub poops: u8,
    /// When the oldest poop appeared; `NEVER` if `poops == 0`.
    pub poop_since: Sec,
    pub sick: bool,
    pub sleeping: bool,
    pub lights_off: bool,
    /// Bitmask of `attention::*`.
    pub attention: u8,
    pub care_mistakes: u8,
    pub sick_count: u8,
    pub snacks_since_wake: u8,
    pub wins: u8,
    pub losses: u8,
    pub last_battle_at: Sec,
    /// 0 none, 1 starve, 2 sickness, 3 old age (see `DeathCause`).
    pub death_cause: u8,
}

impl Pet {
    /// A freshly-reset egg. Meters are anchored but meaningless until `Hatch` fires and
    /// re-anchors them to the baby stage's step values (docs/STATE_MODEL.md's Hatch handler).
    pub fn new_egg(species: u8, now: Sec) -> Self {
        Pet {
            species,
            stage: Stage::Egg,
            hatched_at: NEVER,
            stage_since: now,
            hunger: Meter::new(0, now, 0, 1),
            happy: Meter::new(100, now, 0, -1),
            discipline: 0,
            health: 100,
            weight: 1,
            poops: 0,
            poop_since: NEVER,
            sick: false,
            sleeping: false,
            lights_off: false,
            attention: 0,
            care_mistakes: 0,
            sick_count: 0,
            snacks_since_wake: 0,
            wins: 0,
            losses: 0,
            last_battle_at: NEVER,
            death_cause: DeathCause::None as u8,
        }
    }
}
