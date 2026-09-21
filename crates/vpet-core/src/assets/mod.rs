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

/// Index into `generated::FONT_3X5` for a glyph, or `None` if `c` has no glyph. Order matches
/// `assets/global/font3x5.txt` exactly (A-Z, 0-9, `! ? - . :`); pure and hand-written rather
/// than compiled, since it only depends on that fixed order, not on pixel data.
pub const fn glyph_index(c: u8) -> Option<u8> {
    match c {
        b'A'..=b'Z' => Some(c - b'A'),
        b'0'..=b'9' => Some(26 + (c - b'0')),
        b'!' => Some(36),
        b'?' => Some(37),
        b'-' => Some(38),
        b'.' => Some(39),
        b':' => Some(40),
        _ => None,
    }
}
