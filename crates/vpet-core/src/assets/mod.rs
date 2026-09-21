//! Asset types. Hand-written; the *data* (`generated.rs`) is produced by `spritekit compile`
//! from Phase 3 onward. See docs/CONTENT.md.
//!
//! Sprites are `&'static [u8]` (ADR 0011: no allocator, no runtime asset loading).

pub mod generated;

use crate::pet::Stage;

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

/// Per-stage tuning that isn't a full pose set (`species.toml`'s `[stages.<stage>]` tables;
/// docs/CONTENT.md). Battle stats join in Phase 6.
#[derive(Debug, Clone, Copy)]
pub struct StageRules {
    pub hunger_step_secs: u16,
    pub happy_step_secs: u16,
    pub poop_interval_min_secs: u32,
    pub poop_interval_max_secs: u32,
    /// Time in this stage before `Evolve` fires (`Baby`) or before the stage's evolve branches
    /// are evaluated (`Child`). 0 for the adult stages, which end by `OldAge` instead.
    pub stage_secs: u32,
    /// `lifespan_secs = [min, max]` for the adult stages (docs/GAME_DESIGN.md "Death": the
    /// `OldAge` timer is drawn uniformly from this window at adult evolution, then shortened
    /// by care mistakes). Both 0 for stages that don't die of old age.
    pub lifespan_min_secs: u32,
    pub lifespan_max_secs: u32,
}

/// One `[[stages.child.evolve.branch]]` entry (docs/CONTENT.md, docs/GAME_DESIGN.md
/// "Evolution"). Branches are evaluated in file order; a branch matches when every condition
/// it carries holds. Among the first run of matching branches with equal `weight`, the RNG
/// picks one (no draw when there is only one).
#[derive(Debug, Clone, Copy)]
pub struct Branch {
    /// `Stage::Adult` or `Stage::AdultAlt`.
    pub to: Stage,
    /// Species id to jump lineages to (`to_species` in the toml), resolved against
    /// `registry.toml` by `spritekit compile`; `None` stays in this species.
    pub to_species: Option<u8>,
    pub max_care_mistakes: Option<u8>,
    pub min_discipline: Option<u8>,
    pub max_weight: Option<u16>,
    /// RNG priority among simultaneously-matching branches; default 0.
    pub weight: u8,
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
    /// Empty for every stage but `child` today (baby evolves unconditionally; adults die).
    pub evolve: &'static [Branch],
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

/// Global game constants compiled from `assets/asset-spec.toml`'s `[rules.game]` table
/// (docs/GAME_DESIGN.md "Tuning table"). Every key of that table has a field here, even ones a
/// given phase doesn't read yet (battle_*, lifespan_*), so this struct doesn't need to grow
/// again when Phase 4/6 wire them up.
#[derive(Debug, Clone, Copy)]
pub struct GameRules {
    pub care_miss_secs: u32,
    pub sick_delay_min_secs: u32,
    pub sick_delay_max_secs: u32,
    pub sick_damage_secs: u32,
    pub sick_damage_hp: u8,
    pub starve_secs: u32,
    pub lifespan_penalty_per_mistake_secs: u32,
    pub lifespan_floor_secs: u32,
    pub tantrum_interval_min_secs: u32,
    pub tantrum_interval_max_secs: u32,
    pub battle_cooldown_secs: u32,
    pub menu_timeout_secs: u32,
    /// Hunger value at which the `hungry` attention bit is set.
    pub hungry_threshold: u8,
    /// Happiness value at which the `sad` attention bit is set.
    pub sad_threshold: u8,
    /// Hunger value below which a meal is refused (already full).
    pub full_threshold: u8,
    pub overweight_threshold: u16,
    pub max_poops: u8,
    pub poop_sick_after_secs: u32,
    pub snacks_before_sick: u8,
    pub meal_hunger: u8,
    pub meal_weight: u16,
    pub snack_happy: u8,
    pub snack_weight: u16,
    /// Signed: play costs weight.
    pub play_weight: i16,
    pub play_happy_win: u8,
    pub play_happy_lose: u8,
    pub medicine_health: u8,
    pub medicine_discipline_penalty: u8,
    pub discipline_gain: u8,
    pub discipline_wrong_happy_penalty: u8,
    pub wake_health_bonus: u8,
    pub battle_hit_damage: u8,
    pub battle_crit_damage: u8,
    pub battle_happy_win: u8,
    pub battle_happy_lose: u8,
    pub battle_lose_sick_pct: u8,
}

/// Index into `generated::FONT_5X7` for a glyph, or `None` if `c` has no glyph. Order matches
/// `assets/global/font5x7.txt` exactly (A-Z, 0-9, `! ? - . :`, space); pure and hand-written
/// rather than compiled, since it only depends on that fixed order, not on pixel data.
pub const fn glyph_index(c: u8) -> Option<u8> {
    match c {
        b'A'..=b'Z' => Some(c - b'A'),
        b'0'..=b'9' => Some(26 + (c - b'0')),
        b'!' => Some(36),
        b'?' => Some(37),
        b'-' => Some(38),
        b'.' => Some(39),
        b':' => Some(40),
        b' ' => Some(41),
        _ => None,
    }
}
