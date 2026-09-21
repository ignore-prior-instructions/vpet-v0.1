//! Scene composition: turns game state into a frame. See docs/art/SCREEN_LAYOUT.md. Phase 1
//! implements the three scenes it needs: Egg, the idle Main scene, and the Menu. Dirty/Sick/
//! Sleeping/Eating/Battle/etc. join in Phase 3+ alongside their simulation handlers.

use super::fb::Fb;
use crate::anim::{hop_dy, is_blink, toggle, walk_position, AnimState};
use crate::assets::{Sprite, StageSet};
use crate::time::Sec;

/// Egg scene: `egg_a` at home (8, 0); in the last minute before hatching, alternates with
/// `egg_b` every 8 ticks; hops in the final second (docs/art/ANIMATION.md "Egg").
pub fn render_egg(
    egg_sprites: &[Sprite; 2],
    remaining_to_hatch: Sec,
    tick: u32,
    anim: &AnimState,
) -> Fb {
    let mut fb = Fb::new();
    let elapsed = tick.saturating_sub(anim.clip_start_tick);
    let use_b = remaining_to_hatch <= 60 && toggle(elapsed, 8);
    let sprite = if use_b {
        &egg_sprites[1]
    } else {
        &egg_sprites[0]
    };
    let dy = if remaining_to_hatch <= 1 {
        hop_dy(tick)
    } else {
        0
    };
    fb.blit_or(sprite, 8, dy);
    fb
}

/// Idle scene: pet toggles `idle_a`/`idle_b` every 2 ticks, walks `x` in `[1, 15]`, blinks.
pub fn render_idle(stage: &StageSet, tick: u32, anim: &AnimState) -> Fb {
    let mut fb = Fb::new();
    let elapsed = tick.saturating_sub(anim.clip_start_tick);
    let use_b = toggle(elapsed, 2);
    let pose = if use_b { &stage.idle_b } else { &stage.idle_a };
    let x = walk_position(anim.anim_rng, elapsed, 1, 15, 8) as i32;
    fb.blit_or(&pose.img, x, 0);
    if is_blink(anim.anim_rng, elapsed) {
        if let Some(mask) = &pose.blink {
            fb.blit_xor(mask, x, 0);
        }
    }
    fb
}

/// Menu scene: the eight icons tile a 4x2 grid of 8x8 cells; the selected one is XOR-inverted
/// (docs/art/SCREEN_LAYOUT.md "Menu"). `cursor` is `0..8`, matching `assets::icon::*`.
pub fn render_menu(icons: &[Sprite; 8], cursor: u8) -> Fb {
    let mut fb = Fb::new();
    for (i, icon) in icons.iter().enumerate() {
        let gx = ((i % 4) * 8) as i32;
        let gy = ((i / 4) * 8) as i32;
        fb.blit_or(icon, gx, gy);
    }
    let cursor = cursor.min(7) as usize;
    let cx = ((cursor % 4) * 8) as i32;
    let cy = ((cursor / 4) * 8) as i32;
    fb.invert_rect(cx, cy, 8, 8);
    fb
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::assets::generated;

    #[test]
    fn menu_inverts_the_selected_cell_only() {
        let plain = render_menu(&generated::ICONS, 0);
        let selected = render_menu(&generated::ICONS, 5);
        // Different cursors must produce different frames (the inverted cell differs).
        assert_ne!(plain.as_bytes(), selected.as_bytes());
    }

    #[test]
    fn egg_scene_renders_something() {
        let anim = AnimState::new();
        let fb = render_egg(&generated::EGG, 200, 0, &anim);
        assert!(fb.as_bytes().iter().any(|&b| b != 0));
    }

    #[test]
    fn idle_scene_is_deterministic_for_same_tick() {
        let anim = AnimState::new();
        let species = &generated::SPECIES[0];
        let a = render_idle(&species.baby, 100, &anim);
        let b = render_idle(&species.baby, 100, &anim);
        assert_eq!(a.as_bytes(), b.as_bytes());
    }
}
