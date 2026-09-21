// PHASE-1 HAND-WRITTEN PLACEHOLDER. From Phase 3 onward this file is produced by
// `spritekit compile` and must never be hand-edited (docs/CONTENT.md, docs/adr/0009). Until
// then, it is written by hand but its byte data is real: every array below was packed from a
// validated text-grid sprite via `uv run spritekit pack <file>` (icons/egg/effects from
// assets/global/, and one throwaway placeholder pet pose that is *not* checked in as a species
// because it never went through the art pipeline — see docs/art/ART_PIPELINE.md). Nothing here
// is invented pixel data.
//
// CONTENT_HASH is an FNV-1a-32 over the packed image (and mask, where present) bytes of every
// table below, in table order. It is informational only in Phase 1 (docs/SAVE_FORMAT.md:
// "content_hash mismatch does not fail the load"); `spritekit compile` computes the real one
// from the full asset set starting Phase 3.

use crate::assets::{GameRules, Pose, SpeciesDef, Sprite, StageRules, StageSet};

pub const CONTENT_HASH: u32 = 0x44273b96;

// --- icons (8x8), order matches assets/asset-spec.toml [global].icons -------------------------

static FEED_IMG: [u8; 8] = [0x00, 0x38, 0x7C, 0xFE, 0x7C, 0x38, 0x00, 0x00];
static LIGHTS_IMG: [u8; 8] = [0x44, 0x38, 0x7C, 0xFE, 0x7C, 0x38, 0x44, 0x00];
static PLAY_IMG: [u8; 8] = [0xC0, 0xE0, 0xF0, 0xF8, 0xF0, 0xE0, 0xC0, 0x00];
static MEDICINE_IMG: [u8; 8] = [0x38, 0x38, 0x38, 0xFE, 0x38, 0x38, 0x38, 0x00];
static CLEAN_IMG: [u8; 8] = [0x10, 0x38, 0x7C, 0xFE, 0xFE, 0x7C, 0x38, 0x00];
static STATUS_IMG: [u8; 8] = [0xFE, 0x00, 0xFE, 0x00, 0xFE, 0x00, 0xFE, 0x00];
static DISCIPLINE_IMG: [u8; 8] = [0x3C, 0x3C, 0x3C, 0x3C, 0x00, 0x3C, 0x00, 0x00];
static BATTLE_IMG: [u8; 8] = [0xC6, 0x6C, 0x38, 0x38, 0x38, 0x6C, 0xC6, 0x00];

pub static ICONS: [Sprite; 8] = [
    Sprite {
        w: 8,
        h: 8,
        rows: &FEED_IMG,
    },
    Sprite {
        w: 8,
        h: 8,
        rows: &LIGHTS_IMG,
    },
    Sprite {
        w: 8,
        h: 8,
        rows: &PLAY_IMG,
    },
    Sprite {
        w: 8,
        h: 8,
        rows: &MEDICINE_IMG,
    },
    Sprite {
        w: 8,
        h: 8,
        rows: &CLEAN_IMG,
    },
    Sprite {
        w: 8,
        h: 8,
        rows: &STATUS_IMG,
    },
    Sprite {
        w: 8,
        h: 8,
        rows: &DISCIPLINE_IMG,
    },
    Sprite {
        w: 8,
        h: 8,
        rows: &BATTLE_IMG,
    },
];

/// Index into `ICONS`, matching the menu cursor order (docs/art/SCREEN_LAYOUT.md "Menu"):
/// Feed, Lights, Play, Medicine, Clean, Status, Discipline, Battle.
pub mod icon {
    pub const FEED: usize = 0;
    pub const LIGHTS: usize = 1;
    pub const PLAY: usize = 2;
    pub const MEDICINE: usize = 3;
    pub const CLEAN: usize = 4;
    pub const STATUS: usize = 5;
    pub const DISCIPLINE: usize = 6;
    pub const BATTLE: usize = 7;
    pub const COUNT: usize = 8;
}

// --- egg (16x16) --------------------------------------------------------------------------

static EGG_A_IMG: [u8; 32] = [
    0x00, 0x00, 0x03, 0xC0, 0x07, 0xE0, 0x0F, 0xF0, 0x1F, 0xF8, 0x1F, 0xF8, 0x3F, 0xFC, 0x3F, 0xFC,
    0x3F, 0xFC, 0x3F, 0xFC, 0x3F, 0xFC, 0x1F, 0xF8, 0x1F, 0xF8, 0x0F, 0xF0, 0x07, 0xE0, 0x03, 0xC0,
];
static EGG_B_IMG: [u8; 32] = [
    0x00, 0x00, 0x03, 0xC0, 0x07, 0xE0, 0x0F, 0xF0, 0x1F, 0xF8, 0x1F, 0xF8, 0x3F, 0xFC, 0x00, 0x00,
    0x00, 0x00, 0x3F, 0xFC, 0x3F, 0xFC, 0x1F, 0xF8, 0x1F, 0xF8, 0x0F, 0xF0, 0x07, 0xE0, 0x03, 0xC0,
];

pub static EGG: [Sprite; 2] = [
    Sprite {
        w: 16,
        h: 16,
        rows: &EGG_A_IMG,
    },
    Sprite {
        w: 16,
        h: 16,
        rows: &EGG_B_IMG,
    },
];

// --- effects (8x8), order matches assets/asset-spec.toml [global].effects ---------------------

static HEART_IMG: [u8; 8] = [0x66, 0xFF, 0xFF, 0x7E, 0x7E, 0x3C, 0x18, 0x00];
static HEART_SMALL_IMG: [u8; 8] = [0x00, 0x6C, 0xFF, 0x7E, 0x3C, 0x18, 0x00, 0x00];
static ZZ_A_IMG: [u8; 8] = [0xFF, 0x01, 0x02, 0x04, 0x08, 0x10, 0x20, 0xFF];
static ZZ_B_IMG: [u8; 8] = [0x00, 0x0F, 0x01, 0x02, 0x04, 0x0F, 0x00, 0x00];
static SKULL_IMG: [u8; 8] = [0x3C, 0x7E, 0xFF, 0xDB, 0xFF, 0x7E, 0x3C, 0x00];
static SWEAT_IMG: [u8; 8] = [0x00, 0x10, 0x38, 0x7C, 0x7C, 0x38, 0x10, 0x00];
static SPARKLE_A_IMG: [u8; 8] = [0x18, 0x00, 0x00, 0xC3, 0x00, 0x00, 0x18, 0x00];
static SPARKLE_B_IMG: [u8; 8] = [0x00, 0x66, 0x24, 0x00, 0x00, 0x24, 0x66, 0x00];
static HIT_STAR_IMG: [u8; 8] = [0x18, 0x3C, 0x7E, 0xFF, 0xFF, 0x7E, 0x3C, 0x18];
static ATTENTION_IMG: [u8; 8] = [0x30, 0x30, 0x30, 0x30, 0x30, 0x00, 0x30, 0x00];
static NOTE_IMG: [u8; 8] = [0x03, 0x01, 0x01, 0x01, 0x01, 0x3D, 0x7E, 0x3C];
static CROSS_IMG: [u8; 8] = [0x81, 0x42, 0x24, 0x18, 0x18, 0x24, 0x42, 0x81];

pub static EFFECTS: [Sprite; 12] = [
    Sprite {
        w: 8,
        h: 8,
        rows: &HEART_IMG,
    },
    Sprite {
        w: 8,
        h: 8,
        rows: &HEART_SMALL_IMG,
    },
    Sprite {
        w: 8,
        h: 8,
        rows: &ZZ_A_IMG,
    },
    Sprite {
        w: 8,
        h: 8,
        rows: &ZZ_B_IMG,
    },
    Sprite {
        w: 8,
        h: 8,
        rows: &SKULL_IMG,
    },
    Sprite {
        w: 8,
        h: 8,
        rows: &SWEAT_IMG,
    },
    Sprite {
        w: 8,
        h: 8,
        rows: &SPARKLE_A_IMG,
    },
    Sprite {
        w: 8,
        h: 8,
        rows: &SPARKLE_B_IMG,
    },
    Sprite {
        w: 8,
        h: 8,
        rows: &HIT_STAR_IMG,
    },
    Sprite {
        w: 8,
        h: 8,
        rows: &ATTENTION_IMG,
    },
    Sprite {
        w: 8,
        h: 8,
        rows: &NOTE_IMG,
    },
    Sprite {
        w: 8,
        h: 8,
        rows: &CROSS_IMG,
    },
];

pub mod effect {
    pub const HEART: usize = 0;
    pub const HEART_SMALL: usize = 1;
    pub const ZZ_A: usize = 2;
    pub const ZZ_B: usize = 3;
    pub const SKULL: usize = 4;
    pub const SWEAT: usize = 5;
    pub const SPARKLE_A: usize = 6;
    pub const SPARKLE_B: usize = 7;
    pub const HIT_STAR: usize = 8;
    pub const ATTENTION: usize = 9;
    pub const NOTE: usize = 10;
    pub const CROSS: usize = 11;
}

// --- placeholder species (id 0) -----------------------------------------------------------
//
// One crude pose pair (idle_a/idle_b), reused for every required StageSet pose. Real species
// (lalafu and friends) arrive in Phase 3 through `spritekit new-species` + `gen`, each with its
// own 15-to-28 hand-critiqued poses; this placeholder only exists so Phase 1's egg-to-baby
// golden has *something* to render.

static IDLE_A_IMG: [u8; 32] = [
    0x00, 0x00, 0x03, 0xC0, 0x07, 0xE0, 0x0F, 0xF0, 0x1F, 0xF8, 0x1D, 0xB8, 0x1F, 0xF8, 0x3F, 0xFC,
    0x3F, 0xFC, 0x3F, 0xFC, 0x3F, 0xFC, 0x1F, 0xF8, 0x1F, 0xF8, 0x0E, 0x70, 0x0E, 0x70, 0x0E, 0x70,
];
static IDLE_A_MASK: [u8; 32] = [
    0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x02, 0x40, 0x00, 0x00, 0x00, 0x00,
    0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
];
static IDLE_B_IMG: [u8; 32] = [
    0x00, 0x00, 0x00, 0x00, 0x03, 0xC0, 0x07, 0xE0, 0x0F, 0xF0, 0x1F, 0xF8, 0x1D, 0xB8, 0x1F, 0xF8,
    0x3F, 0xFC, 0x3F, 0xFC, 0x3F, 0xFC, 0x3F, 0xFC, 0x1F, 0xF8, 0x1F, 0xF8, 0x0E, 0x70, 0x0E, 0x70,
];
static IDLE_B_MASK: [u8; 32] = [
    0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x02, 0x40, 0x00, 0x00,
    0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
];

const PLACEHOLDER_IDLE_A: Pose = Pose {
    img: Sprite {
        w: 16,
        h: 16,
        rows: &IDLE_A_IMG,
    },
    blink: Some(Sprite {
        w: 16,
        h: 16,
        rows: &IDLE_A_MASK,
    }),
    mouth: (7, 10),
    bbox: (2, 1, 13, 15),
};
const PLACEHOLDER_IDLE_B: Pose = Pose {
    img: Sprite {
        w: 16,
        h: 16,
        rows: &IDLE_B_IMG,
    },
    blink: Some(Sprite {
        w: 16,
        h: 16,
        rows: &IDLE_B_MASK,
    }),
    mouth: (7, 10),
    bbox: (2, 2, 13, 15),
};

const PLACEHOLDER_RULES: StageRules = StageRules {
    hunger_step_secs: 180,
    happy_step_secs: 240,
    poop_interval_min_secs: 7200,
    poop_interval_max_secs: 10800,
    stage_secs: 3600,
};

const PLACEHOLDER_STAGE: StageSet = StageSet {
    idle_a: PLACEHOLDER_IDLE_A,
    idle_b: PLACEHOLDER_IDLE_B,
    happy: PLACEHOLDER_IDLE_A,
    eat: PLACEHOLDER_IDLE_A,
    sleep: PLACEHOLDER_IDLE_A,
    sad: None,
    attack: None,
    rules: PLACEHOLDER_RULES,
};

pub static SPECIES: [SpeciesDef; 1] = [SpeciesDef {
    id: 0,
    slug: "placeholder",
    name: "Placeholder",
    hatch_secs: 300,
    schedule: (50400, 36000),
    egg: None, // None: renderer falls back to the global `EGG` table.
    baby: PLACEHOLDER_STAGE,
    child: PLACEHOLDER_STAGE,
    adult: PLACEHOLDER_STAGE,
    adult_alt: None,
}];

pub static GAME: GameRules = GameRules {
    menu_timeout_secs: 10,
    care_miss_secs: 900,
    hungry_threshold: 80,
    sad_threshold: 20,
};
