//! Dev-only pose/global overrides plus a standalone clip renderer, gated behind the
//! `dev-overrides` cargo feature (pulls in `std`; never built into the shipped `vpet.wasm` —
//! see `just wasm-dev` and docs/CONTENT.md "Hot preview without rebuilding the core").
//!
//! Deliberately outside `Cart`: nothing here touches simulation state (the clock, the RNG, a
//! pet's meters), only asset lookup and rendering, so this module cannot affect determinism.
//! It is not linked into the default build at all, so it cannot affect the size gate either.

extern crate std;

use std::boxed::Box;
use std::collections::BTreeMap;
use std::string::String;
use std::sync::Mutex;

use crate::anim::AnimState;
use crate::assets::generated;
use crate::assets::{Pose, Sprite, StageSet};
use crate::render::{compose, Fb};

fn pose_overrides() -> &'static Mutex<BTreeMap<String, Pose>> {
    static M: std::sync::OnceLock<Mutex<BTreeMap<String, Pose>>> = std::sync::OnceLock::new();
    M.get_or_init(|| Mutex::new(BTreeMap::new()))
}

fn global_overrides() -> &'static Mutex<BTreeMap<String, Sprite>> {
    static M: std::sync::OnceLock<Mutex<BTreeMap<String, Sprite>>> = std::sync::OnceLock::new();
    M.get_or_init(|| Mutex::new(BTreeMap::new()))
}

/// Leaks the bytes so the resulting slice can live in a `'static` `Sprite`/`Pose`. Fine for a
/// dev tool that runs for a single browser tab's lifetime; never reachable from the shipped
/// build.
fn leak(bytes: &[u8]) -> &'static [u8] {
    Box::leak(bytes.to_vec().into_boxed_slice())
}

fn pose_key(slug: &str, stage: u8, name: &str) -> String {
    let mut key = String::with_capacity(slug.len() + name.len() + 2);
    key.push_str(slug);
    key.push('/');
    key.push(stage_char(stage));
    key.push('/');
    key.push_str(name);
    key
}

fn global_key(category: &str, name: &str) -> String {
    let mut key = String::with_capacity(category.len() + name.len() + 1);
    key.push_str(category);
    key.push('/');
    key.push_str(name);
    key
}

fn stage_char(stage: u8) -> char {
    match stage {
        0 => 'b',
        1 => 'c',
        2 => 'a',
        _ => 'x', // adult_alt
    }
}

/// Overwrites one pose (`species_slug/stage/pose_name`, e.g. `"lalafu/b/idle_a"`), consulted by
/// `render_clip` before the compiled-in species table. `stage`: 0=baby, 1=child, 2=adult,
/// 3=adult_alt.
#[allow(clippy::too_many_arguments)]
pub fn set_pose(
    slug: &str,
    stage: u8,
    name: &str,
    w: u8,
    h: u8,
    img: &[u8],
    mask: Option<&[u8]>,
    mouth: (i8, i8),
    bbox: (u8, u8, u8, u8),
) {
    let pose = Pose {
        img: Sprite {
            w,
            h,
            rows: leak(img),
        },
        blink: mask.map(|m| Sprite {
            w,
            h,
            rows: leak(m),
        }),
        mouth,
        bbox,
    };
    pose_overrides()
        .lock()
        .unwrap()
        .insert(pose_key(slug, stage, name), pose);
}

/// Overwrites one global sprite (`category/name`, e.g. `"icon/feed"`, `"item/food_a"`,
/// `"font/A"`, `"tombstone/tombstone"`). Stored for API completeness with docs/CONTENT.md's
/// spec (`dev_set_pose`, `dev_set_global`, `dev_clear_overrides`, `dev_render_clip`) and for a
/// caller that wants to pack+preview a global sprite directly; `render_clip` itself doesn't
/// consult this table yet, because `render/compose.rs`'s scenes reference `ITEMS`/`EFFECTS`/
/// `HEARTS`/`FONT_5X7` as compiled constants rather than through a lookup indirection. Species
/// poses (`set_pose`, consulted by every `render_clip` activity) are where the "draw a new
/// species without rebuilding" workflow (docs/CONTENT.md "Adding a species") actually lives;
/// wiring overrides into the global tables too is future work if editing icons/items/font live
/// turns out to be worth it.
pub fn set_global(category: &str, name: &str, w: u8, h: u8, img: &[u8]) {
    let sprite = Sprite {
        w,
        h,
        rows: leak(img),
    };
    global_overrides()
        .lock()
        .unwrap()
        .insert(global_key(category, name), sprite);
}

/// Drops every override, reverting to the compiled-in tables. Leaked bytes from prior overrides
/// are not reclaimed (a dev tool's tab-lifetime leak, not a long-running process concern).
pub fn clear_all() {
    pose_overrides().lock().unwrap().clear();
    global_overrides().lock().unwrap().clear();
}

fn compiled_pose(slug: &str, stage: u8, name: &str) -> Option<Pose> {
    let species = generated::SPECIES.iter().find(|s| s.slug == slug)?;
    let set = match stage {
        0 => &species.baby,
        1 => &species.child,
        2 => &species.adult,
        _ => species.adult_alt.as_ref().unwrap_or(&species.adult),
    };
    match name {
        "idle_a" => Some(set.idle_a),
        "idle_b" => Some(set.idle_b),
        "happy" => Some(set.happy),
        "eat" => Some(set.eat),
        "sleep" => Some(set.sleep),
        "sad" => set.sad,
        _ => None,
    }
}

/// Overlay first, the compiled-in species table second. `None` if neither has this pose (a
/// brand-new species with this pose not drawn yet, or an unknown name).
fn resolve_pose(slug: &str, stage: u8, name: &str) -> Option<Pose> {
    if let Some(p) = pose_overrides()
        .lock()
        .unwrap()
        .get(&pose_key(slug, stage, name))
    {
        return Some(*p);
    }
    compiled_pose(slug, stage, name)
}

fn resolve_stage_set(slug: &str, stage: u8) -> Option<StageSet> {
    Some(StageSet {
        idle_a: resolve_pose(slug, stage, "idle_a")?,
        idle_b: resolve_pose(slug, stage, "idle_b")?,
        happy: resolve_pose(slug, stage, "happy")?,
        eat: resolve_pose(slug, stage, "eat")?,
        sleep: resolve_pose(slug, stage, "sleep")?,
        sad: resolve_pose(slug, stage, "sad"),
        attack: None,
        // Irrelevant to a static clip render; `render_clip` never reads timing rules.
        rules: crate::assets::StageRules {
            hunger_step_secs: 0,
            happy_step_secs: 0,
            poop_interval_min_secs: 0,
            poop_interval_max_secs: 0,
            stage_secs: 0,
            lifespan_min_secs: 0,
            lifespan_max_secs: 0,
        },
        evolve: &[],
    })
}

/// Renders one tick of one named clip for `species_slug`/`stage`, using overrides where set and
/// the compiled-in species otherwise. Returns a blank (all-zero) frame if the species/stage has
/// no pose data at all yet (nothing overridden, nothing compiled in) — this is a preview tool,
/// not simulation, so there is no error path to report through.
///
/// `activity` is one of: `egg_idle`, `egg_hatching`, `idle`, `dirty`, `sick`, `sleep_lights_on`,
/// `sleep_lights_off`, `eat_meal`, `eat_snack`, `refuse`, `discipline`, `result_win`,
/// `result_lose`, `playing`.
pub fn render_clip(slug: &str, stage: u8, activity: &str, tick: u32) -> Fb {
    let anim = AnimState::new();

    if activity == "egg_idle" || activity == "egg_hatching" {
        let egg = generated::SPECIES
            .iter()
            .find(|s| s.slug == slug)
            .and_then(|s| s.egg.as_ref())
            .unwrap_or(&generated::EGG);
        let remaining = if activity == "egg_hatching" { 0 } else { 120 };
        return compose::render_egg(egg, remaining, tick, &anim);
    }

    let Some(set) = resolve_stage_set(slug, stage) else {
        return Fb::new();
    };

    match activity {
        "idle" => compose::render_main(&set, 0, 0, tick, &anim),
        "dirty" => compose::render_main(&set, 3, 0, tick, &anim),
        "sick" => compose::render_sick(&set, tick),
        "sleep_lights_on" => compose::render_sleeping(&set, false, 0, tick),
        "sleep_lights_off" => compose::render_sleeping(&set, true, 0, tick),
        "eat_meal" => compose::render_eating(&set, false, tick, &anim),
        "eat_snack" => compose::render_eating(&set, true, tick, &anim),
        "refuse" => compose::render_refuse(&set, tick, &anim),
        "discipline" => compose::render_discipline_busy(&set, tick, &anim),
        "result_win" => compose::render_result(&set, true, tick, &anim),
        "result_lose" => compose::render_result(&set, false, tick, &anim),
        "playing" => compose::render_playing(&set, 0b10101, 2, tick, &anim),
        _ => Fb::new(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn tiny_pose(on: bool) -> (u8, u8, std::vec::Vec<u8>) {
        // 8x8, single byte per row (stride = ceil(8/8) = 1); all-on or all-off.
        let byte = if on { 0xFF } else { 0x00 };
        (8, 8, [byte; 8].to_vec())
    }

    // A single test, not three: `pose_overrides`/`global_overrides` are process-wide statics,
    // and cargo runs tests in parallel threads by default, so any test calling `clear_all()`
    // would race every other test's in-flight `set_pose`/`render_clip` calls. Keeping every
    // assertion about that shared state in one test sidesteps the race instead of chasing it.
    #[test]
    fn overrides_overlay_compiled_species_and_clear_all_drops_them() {
        clear_all();

        // Unknown slug, no compiled fallback: overriding all 5 required poses is enough to
        // render something.
        let (w, h, img) = tiny_pose(true);
        for name in ["idle_a", "idle_b", "happy", "eat", "sleep"] {
            set_pose(
                "totally-new-species",
                0,
                name,
                w,
                h,
                &img,
                None,
                (0, 0),
                (0, 0, 7, 7),
            );
        }
        let fb = render_clip("totally-new-species", 0, "idle", 0);
        assert_ne!(*fb.as_bytes(), [0u8; crate::render::fb::FRAME_LEN]);

        // An unrelated slug with nothing set at all renders blank rather than panicking.
        let blank = render_clip("does-not-exist", 0, "idle", 0);
        assert_eq!(*blank.as_bytes(), [0u8; crate::render::fb::FRAME_LEN]);

        // Clearing drops the override; the same slug/activity now renders blank again (no
        // compiled species named "totally-new-species" to fall back to).
        clear_all();
        let after_clear = render_clip("totally-new-species", 0, "idle", 0);
        assert_eq!(*after_clear.as_bytes(), [0u8; crate::render::fb::FRAME_LEN]);
    }
}
