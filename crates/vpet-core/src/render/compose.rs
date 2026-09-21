//! Scene composition: turns game state into a frame. See docs/art/SCREEN_LAYOUT.md. Each
//! function here takes plain parameters (poses, flags, numbers), not `Cart`/`Ui` directly —
//! `lib.rs`'s `render_frame` extracts what a scene needs and dispatches to it.

use super::fb::Fb;
use super::text::draw_text;
use crate::anim::{hop_dy, is_blink, shake_dx, toggle, walk_position, AnimState};
use crate::assets::generated::{effect, heart, item, EFFECTS, HEARTS, ITEMS, TOMBSTONE};
use crate::assets::{Sprite, StageSet};
use crate::time::Sec;

/// Fixed HUD slot (docs/art/SCREEN_LAYOUT.md: "(24,0)... never covered by the pet"): skull when
/// sick, `!` when calling, zz-adjacent glyphs elsewhere. Only one of these is ever active at a
/// time in Phase 3 (sick takes priority in `render_frame`'s dispatch), so a single slot suffices.
const HUD_X: i32 = 24;
const HUD_Y: i32 = 0;

fn draw_attention_glyph(fb: &mut Fb, attention: u8, tick: u32) {
    if attention != 0 {
        fb.blit_or(&EFFECTS[effect::ATTENTION], HUD_X, HUD_Y);
        let _ = tick; // static glyph for now; blinking is a Phase 6 polish item
    }
}

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

/// Main scene (Idle/Dirty): pet toggles `idle_a`/`idle_b` every 2 ticks, walks (narrower range
/// while dirty so it never overlaps the poop pile), blinks, shows the poop pile and the
/// attention glyph.
pub fn render_main(stage: &StageSet, poops: u8, attention: u8, tick: u32, anim: &AnimState) -> Fb {
    let mut fb = Fb::new();
    let elapsed = tick.saturating_sub(anim.clip_start_tick);
    let use_b = toggle(elapsed, 2);
    let pose = if use_b { &stage.idle_b } else { &stage.idle_a };
    let (lo, hi) = if poops > 0 { (1, 7) } else { (1, 15) };
    let x = walk_position(anim.anim_rng, elapsed, lo, hi, 8) as i32;
    fb.blit_or(&pose.img, x, 0);
    if is_blink(anim.anim_rng, elapsed) {
        if let Some(mask) = &pose.blink {
            fb.blit_xor(mask, x, 0);
        }
    }
    draw_poop_pile(&mut fb, poops);
    draw_attention_glyph(&mut fb, attention, tick);
    fb
}

fn draw_poop_pile(fb: &mut Fb, poops: u8) {
    // docs/art/SCREEN_LAYOUT.md: first at (24,8); second at (25,7) drawn over; third at (26,6).
    const POSITIONS: [(i32, i32); 3] = [(24, 8), (25, 7), (26, 6)];
    const SPRITES: [usize; 3] = [item::POOP_A, item::POOP_B, item::POOP_A];
    for i in 0..(poops as usize).min(3) {
        let (x, y) = POSITIONS[i];
        fb.blit_or(&ITEMS[SPRITES[i]], x, y);
    }
}

/// Sick scene: `sad` (falls back to `idle_a` if the species has no `sad`), static at home,
/// skull overlay in the HUD slot.
pub fn render_sick(stage: &StageSet, tick: u32) -> Fb {
    let mut fb = Fb::new();
    let pose = stage.sad.as_ref().unwrap_or(&stage.idle_a);
    fb.blit_or(&pose.img, 8, 0);
    if tick % 8 < 4 {
        // "blinking every 4 ticks"
        fb.blit_or(&EFFECTS[effect::SKULL], HUD_X, HUD_Y);
    }
    fb
}

/// Sleeping scene: `sleep` pose, static, zz overlay alternating at head-right; inverts the
/// whole frame when the lights are off (so an OLED reads as mostly dark); the attention glyph
/// still shows if `sleepy` is still pending (lights not yet off).
pub fn render_sleeping(stage: &StageSet, lights_off: bool, attention: u8, tick: u32) -> Fb {
    let mut fb = Fb::new();
    fb.blit_or(&stage.sleep.img, 8, 0);
    let zz = if tick % 8 < 4 {
        effect::ZZ_A
    } else {
        effect::ZZ_B
    };
    let (bx0, by0, bx1, _) = stage.sleep.bbox;
    let anchor_x = 8 + bx1 as i32 + 1;
    let anchor_y = 8 + by0 as i32;
    let _ = bx0;
    fb.blit_or(&EFFECTS[zz], anchor_x, anchor_y);
    draw_attention_glyph(&mut fb, attention, tick);
    if lights_off {
        fb.invert();
    }
    fb
}

/// Eating scene: `eat`/`idle_a` toggle, food frame shrinks over 3 bites (18 ticks total, 6 per
/// bite), food gone on the last bite.
pub fn render_eating(stage: &StageSet, snack: bool, tick: u32, anim: &AnimState) -> Fb {
    let mut fb = Fb::new();
    let elapsed = tick.saturating_sub(anim.clip_start_tick);
    let use_eat = toggle(elapsed, 2);
    let pose = if use_eat { &stage.eat } else { &stage.idle_a };
    fb.blit_or(&pose.img, 8, 0);
    let bite = (elapsed / 6).min(3);
    let food_frames = if snack {
        [item::SNACK_A, item::SNACK_B, item::SNACK_C]
    } else {
        [item::FOOD_A, item::FOOD_B, item::FOOD_C]
    };
    if let Some(&idx) = food_frames.get(bite as usize) {
        fb.blit_or(&ITEMS[idx], 0, 8);
    }
    fb
}

/// Refuse scene: `sad` (or `idle_a` if none) shakes, `cross` at head-right, 4 ticks.
pub fn render_refuse(stage: &StageSet, tick: u32, anim: &AnimState) -> Fb {
    let mut fb = Fb::new();
    let elapsed = tick.saturating_sub(anim.clip_start_tick);
    let pose = stage.sad.as_ref().unwrap_or(&stage.idle_a);
    let dx = 8 + shake_dx(elapsed);
    fb.blit_or(&pose.img, dx, 0);
    let (_, by0, bx1, _) = pose.bbox;
    fb.blit_or(&EFFECTS[effect::CROSS], dx + bx1 as i32 + 1, by0 as i32);
    fb
}

/// Discipline's busy animation: `sad` (or `idle_a`) shakes, 6 ticks, no overlay.
pub fn render_discipline_busy(stage: &StageSet, tick: u32, anim: &AnimState) -> Fb {
    let mut fb = Fb::new();
    let elapsed = tick.saturating_sub(anim.clip_start_tick);
    let pose = stage.sad.as_ref().unwrap_or(&stage.idle_a);
    let dx = 8 + shake_dx(elapsed);
    fb.blit_or(&pose.img, dx, 0);
    fb
}

/// Result scene (end of Play): `happy` (win, hops) or `sad` (lose/draw), heart or sweat.
pub fn render_result(stage: &StageSet, won: bool, tick: u32, anim: &AnimState) -> Fb {
    let mut fb = Fb::new();
    let elapsed = tick.saturating_sub(anim.clip_start_tick);
    if won {
        let dy = hop_dy(elapsed);
        fb.blit_or(&stage.happy.img, 8, dy);
        fb.blit_or(&EFFECTS[effect::HEART], 12, 0);
    } else {
        let pose = stage.sad.as_ref().unwrap_or(&stage.idle_a);
        fb.blit_or(&pose.img, 8, 0);
        fb.blit_or(&EFFECTS[effect::SWEAT], 12, 0);
    }
    fb
}

/// Playing scene: pet faces left/right for the current round (`flip_h`), an `L?R` hint on row
/// 0, five round-dots on row 15, a heart/sweat reaction from the previous round.
pub fn render_playing(stage: &StageSet, seq: u8, round: u8, tick: u32, anim: &AnimState) -> Fb {
    let mut fb = Fb::new();
    let elapsed = tick.saturating_sub(anim.clip_start_tick);
    let facing_right = round < 5 && (seq >> round) & 1 != 0;
    let use_happy = toggle(elapsed, 2);
    let pose = if use_happy {
        &stage.happy
    } else {
        &stage.idle_a
    };
    if facing_right {
        fb.blit_or_flipped(&pose.img, 8, 0);
    } else {
        fb.blit_or(&pose.img, 8, 0);
    }
    draw_text(&mut fb, 12, 0, b"L?R");
    for i in 0..5u8 {
        let x = 12 + i as i32 * 2;
        if i < round {
            fb.invert_rect(x, 15, 1, 1);
        }
    }
    fb
}

/// Feed submenu: `food_a`/`snack_a` large, the selected item's 8x8 block inverted, `MEAL` /
/// `SNACK` labels in 3x5 underneath.
pub fn render_feed_sub(snack: bool) -> Fb {
    let mut fb = Fb::new();
    fb.blit_or(&ITEMS[item::FOOD_A], 4, 4);
    fb.blit_or(&ITEMS[item::SNACK_A], 20, 4);
    let (sel_x, label, other_label) = if snack {
        (20, &b"SNACK"[..], &b"MEAL"[..])
    } else {
        (4, &b"MEAL"[..], &b"SNACK"[..])
    };
    fb.invert_rect(sel_x, 4, 8, 8);
    draw_text(&mut fb, 4, 13, if snack { other_label } else { label });
    draw_text(&mut fb, 20, 13, if snack { label } else { other_label });
    fb
}

/// Status screen page `page` (0..5): hunger, happiness, discipline, age/weight, health.
#[allow(clippy::too_many_arguments)]
pub fn render_status(
    page: u8,
    hunger: u8,
    happiness: u8,
    discipline: u8,
    health: u8,
    sick: bool,
    age_days: u32,
    weight: u16,
) -> Fb {
    let mut fb = Fb::new();
    match page {
        0 => {
            draw_text(&mut fb, 0, 0, b"HUNGER");
            draw_hearts(&mut fb, 100u8.saturating_sub(hunger)); // inverted: full hearts = well fed
        }
        1 => {
            draw_text(&mut fb, 0, 0, b"HAPPY");
            draw_hearts(&mut fb, happiness);
        }
        2 => {
            draw_text(&mut fb, 0, 0, b"DISCPL");
            draw_discipline_bar(&mut fb, discipline);
        }
        3 => {
            let mut buf = [0u8; 8];
            let n = write_u32(&mut buf, age_days);
            draw_text(&mut fb, 0, 0, b"AGE");
            draw_text(&mut fb, 20, 0, &buf[0..n]);
            draw_text(&mut fb, 0, 8, b"WT");
            let mut wbuf = [0u8; 8];
            let wn = write_u32(&mut wbuf, weight as u32);
            draw_text(&mut fb, 20, 8, &wbuf[0..wn]);
        }
        _ => {
            draw_text(&mut fb, 0, 0, b"HEALTH");
            draw_hearts(&mut fb, health);
            if sick {
                fb.blit_or(&EFFECTS[effect::SKULL], 24, 6);
            }
        }
    }
    draw_page_dots(&mut fb, page);
    fb
}

fn draw_hearts(fb: &mut Fb, value: u8) {
    // 4 hearts across x = 0, 8, 16, 24; each represents 25 points.
    for i in 0..4u8 {
        let threshold = (i as u16 + 1) * 25;
        let idx = if (value as u16) >= threshold {
            heart::FULL
        } else {
            heart::EMPTY
        };
        fb.blit_or(&HEARTS[idx], i as i32 * 8, 6);
    }
}

fn draw_discipline_bar(fb: &mut Fb, value: u8) {
    // A 4-segment bar, one segment per 25 points, at row 6..12.
    for i in 0..4u8 {
        let threshold = (i as u16 + 1) * 25;
        if (value as u16) >= threshold {
            fb.invert_rect(i as i32 * 8, 6, 7, 6);
        }
    }
}

fn draw_page_dots(fb: &mut Fb, page: u8) {
    for i in 0..5u8 {
        let x = 12 + i as i32 * 2;
        fb.blit_or(&EFFECTS[effect::NOTE], x, 15); // placeholder dot glyph; a single on-pixel would do too
        if i == page {
            fb.invert_rect(x, 15, 1, 1);
        }
    }
}

/// Writes `n` in decimal into `buf`, returns the number of bytes written. No `alloc`, so this
/// is a small hand-rolled formatter rather than `format!`.
fn write_u32(buf: &mut [u8; 8], mut n: u32) -> usize {
    if n == 0 {
        buf[0] = b'0';
        return 1;
    }
    let mut tmp = [0u8; 8];
    let mut i = 0;
    while n > 0 && i < tmp.len() {
        tmp[i] = b'0' + (n % 10) as u8;
        n /= 10;
        i += 1;
    }
    for j in 0..i {
        buf[j] = tmp[i - 1 - j];
    }
    i
}

/// Evolution (docs/art/SCREEN_LAYOUT.md "Evolution", docs/art/ANIMATION.md "Evolving"): the
/// old stage's `idle_a` at x = 8 with the whole buffer inverted every other tick for 8 ticks
/// and `sparkle_a`/`sparkle_b` alternating at (0, 0) and (24, 8); then the new stage's `idle_a`
/// shaking for 4 ticks. 12 ticks total, mirrored by the `UiBusyEnd` timer `enter_stage` sets.
pub fn render_evolving(old: &StageSet, new: &StageSet, tick: u32, anim: &AnimState) -> Fb {
    let mut fb = Fb::new();
    let elapsed = tick.saturating_sub(anim.clip_start_tick);
    if elapsed < 8 {
        fb.blit_or(&old.idle_a.img, 8, 0);
        let sparkle = if toggle(elapsed, 1) {
            effect::SPARKLE_B
        } else {
            effect::SPARKLE_A
        };
        fb.blit_or(&EFFECTS[sparkle], 0, 0);
        fb.blit_or(&EFFECTS[sparkle], 24, 8);
        if elapsed % 2 == 1 {
            fb.invert();
        }
    } else {
        let dx = 8 + shake_dx(elapsed - 8);
        fb.blit_or(&new.idle_a.img, dx, 0);
    }
    fb
}

/// Dead scene: the tombstone, static, a cross overlay.
pub fn render_dead() -> Fb {
    let mut fb = Fb::new();
    fb.blit_or(&TOMBSTONE, 8, 0);
    fb.blit_or(&EFFECTS[effect::CROSS], 12, 0);
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
        let a = render_main(&species.baby, 0, 0, 100, &anim);
        let b = render_main(&species.baby, 0, 0, 100, &anim);
        assert_eq!(a.as_bytes(), b.as_bytes());
    }

    #[test]
    fn dirty_pet_shows_the_poop_pile() {
        let anim = AnimState::new();
        let species = &generated::SPECIES[0];
        let clean = render_main(&species.baby, 0, 0, 100, &anim);
        let dirty = render_main(&species.baby, 2, 0, 100, &anim);
        assert_ne!(clean.as_bytes(), dirty.as_bytes());
    }

    #[test]
    fn sleeping_inverts_when_lights_are_off() {
        let species = &generated::SPECIES[0];
        let on = render_sleeping(&species.baby, false, 0, 0);
        let off = render_sleeping(&species.baby, true, 0, 0);
        // Every byte should be the bitwise complement (the invert happens after everything
        // else is drawn identically in both calls at tick 0).
        for (a, b) in on.as_bytes().iter().zip(off.as_bytes().iter()) {
            assert_eq!(*a, !*b);
        }
    }

    #[test]
    fn dead_scene_renders_the_tombstone() {
        let fb = render_dead();
        assert!(fb.as_bytes().iter().any(|&b| b != 0));
    }

    #[test]
    fn status_pages_differ() {
        let p0 = render_status(0, 50, 50, 50, 100, false, 1, 10);
        let p1 = render_status(1, 50, 50, 50, 100, false, 1, 10);
        assert_ne!(p0.as_bytes(), p1.as_bytes());
    }

    #[test]
    fn write_u32_formats_correctly() {
        let mut buf = [0u8; 8];
        let n = write_u32(&mut buf, 0);
        assert_eq!(&buf[0..n], b"0");
        let n = write_u32(&mut buf, 42);
        assert_eq!(&buf[0..n], b"42");
        let n = write_u32(&mut buf, 12345);
        assert_eq!(&buf[0..n], b"12345");
    }
}
