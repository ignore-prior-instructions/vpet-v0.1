//! Asset types. Hand-written; the *data* (`generated.rs`) is produced by `spritekit compile`
//! from Phase 3 onward. See docs/CONTENT.md.
//!
//! Sprites are `&'static [u8]` (ADR 0011: no allocator, no runtime asset loading).

pub mod generated;

/// A packed 1-bit-per-pixel sprite. Row-major, MSB is the leftmost pixel, row stride
/// `ceil(w/8)` bytes (docs/art/SPRITE_FORMAT.md "Compiled form").
#[derive(Debug, Clone, Copy)]
pub struct Sprite {
    pub w: u8,
    pub h: u8,
    pub rows: &'static [u8],
}

/// One drawable pose: the base image, an optional blink mask (XORed in on a blink tick), the
/// mouth anchor for food/anim overlays, and a precomputed bounding box `(x0, y0, x1, y1)`.
#[derive(Debug, Clone, Copy)]
pub struct Pose {
    pub img: Sprite,
    pub blink: Option<Sprite>,
    pub mouth: (i8, i8),
    pub bbox: (u8, u8, u8, u8),
}

/// Per-stage tuning that isn't a full pose set. Phase 1 only needs enough of this to hatch a
/// baby and schedule its first poop; evolution branches, lifespan, and battle stats join this
/// struct in Phase 3/4 when `spritekit compile` replaces the hand-written `generated.rs`.
#[derive(Debug, Clone, Copy)]
pub struct StageRules {
    pub hunger_step_secs: u16,
    pub happy_step_secs: u16,
    pub poop_interval_min_secs: u32,
    pub poop_interval_max_secs: u32,
    /// Time in this stage before `Evolve` fires (`Baby`) or before the stage's evolve branches
    /// are evaluated (`Child`). Unused by `Adult`/`AdultAlt` in Phase 1 (old age is Phase 4).
    pub stage_secs: u32,
}

#[derive(Debug, Clone, Copy)]
pub struct StageSet {
    pub idle_a: Pose,
    pub idle_b: Pose,
    pub happy: Pose,
    pub eat: Pose,
    pub sleep: Pose,
    pub sad: Option<Pose>,
    pub attack: Option<Pose>,
    pub rules: StageRules,
}

#[derive(Debug, Clone, Copy)]
pub struct SpeciesDef {
    pub id: u8,
    pub slug: &'static str,
    pub name: &'static str,
    pub hatch_secs: u32,
    /// `(awake_secs, sleep_secs)`.
    pub schedule: (u32, u32),
    pub egg: Option<[Sprite; 2]>,
    pub baby: StageSet,
    pub child: StageSet,
    pub adult: StageSet,
    pub adult_alt: Option<StageSet>,
}

/// Global game constants compiled from `assets/asset-spec.toml`'s `[rules.game]` table. Phase 1
/// only draws on `menu_timeout_secs`; the rest of the table
/// (docs/GAME_DESIGN.md "Tuning table") joins in Phase 3.
#[derive(Debug, Clone, Copy)]
pub struct GameRules {
    pub menu_timeout_secs: u32,
    pub care_miss_secs: u32,
    /// Hunger value at which the `hungry` attention bit is set (docs/GAME_DESIGN.md).
    pub hungry_threshold: u8,
    /// Happiness value at which the `sad` attention bit is set.
    pub sad_threshold: u8,
}
