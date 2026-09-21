//! Scene composition: turns game state into a frame. See docs/art/SCREEN_LAYOUT.md. Each
//! function here takes plain parameters (poses, flags, numbers), not `Cart`/`Ui` directly —
//! `lib.rs`'s `render_frame` extracts what a scene needs and dispatches to it. Every coordinate
//! comes from `layout.rs`; there are no bare screen numbers in this file.

use super::fb::Fb;
use super::layout::*;
use super::text::{draw_text, text_width};
use crate::anim::{hop_dy, is_blink, shake_dx, toggle, walk_position, AnimState};
use crate::assets::generated::{effect, heart, item, EFFECTS, HEARTS, ITEMS, TOMBSTONE};
use crate::assets::{Pose, Sprite, StageSet};
use crate::time::Sec;

/// Menu labels in cursor order (docs/art/SCREEN_LAYOUT.md "Menu"; matches
/// `assets::generated::icon::*`).
const MENU_LABELS: [&[u8]; 8] = [
    b"FEED",
    b"LIGHTS",
    b"PLAY",
    b"MEDICINE",
    b"CLEAN",
    b"STATUS",
    b"DISCIPLINE",
    b"BATTLE",
];

const HOME: i32 = PET_HOME_X as i32;

fn draw_attention_glyph(fb: &mut Fb, attention: u8, tick: u32) {
    if attention != 0 {
        fb.blit_or(&EFFECTS[effect::ATTENTION], HUD.0, HUD.1);
        let _ = tick; // static glyph for now; blinking is a Phase 6 polish item
    }
}

/// Centred text: `x` such that `text` is centred on the screen's width.
fn centred_x(text: &[u8]) -> i32 {
    (Fb::W - text_width(text)) / 2
}

/// The pet's head-right anchor for a pose drawn at `x`: just right of the bbox, at its top.
fn head_right(pose: &Pose, x: i32) -> (i32, i32) {
    let (_, y0, x1, _) = pose.bbox;
    (x + x1 as i32 + 1, y0 as i32)
}

/// Egg scene: `egg_a` at home; in the last minute before hatching, alternates with `egg_b`
/// every 8 ticks; hops in the final second (docs/art/ANIMATION.md "Egg").
pub fn render_egg(
    egg_sprites: &[Sprite; 2],
    remaining_to_hatch: Sec,
    tick: u32,
    anim: &AnimState,
) -> Fb {
    let mut fb = Fb::new();
    let elapsed = tick.wrapping_sub(anim.clip_start_tick);
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
    fb.blit_or(sprite, HOME, dy);
    fb
}

/// Main scene (Idle/Dirty): pet toggles `idle_a`/`idle_b` every 2 ticks, walks (narrower range
/// while dirty so it never overlaps the poop pile), blinks, shows the poop pile and the
/// attention glyph.
pub fn render_main(stage: &StageSet, poops: u8, attention: u8, tick: u32, anim: &AnimState) -> Fb {
    let mut fb = Fb::new();
    let elapsed = tick.wrapping_sub(anim.clip_start_tick);
    let use_b = toggle(elapsed, 2);
    let pose = if use_b { &stage.idle_b } else { &stage.idle_a };
    let hi = if poops > 0 { DIRTY_WALK_HI } else { WALK_HI };
    let x = walk_position(anim.anim_rng, elapsed, WALK_LO, hi, PET_HOME_X) as i32;
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
    const SPRITES: [usize; 3] = [item::POOP_A, item::POOP_B, item::POOP_A];
    for i in 0..(poops as usize).min(POOP_PILE.len()) {
        let (x, y) = POOP_PILE[i];
        fb.blit_or(&ITEMS[SPRITES[i]], x, y);
    }
}

/// Sick scene: `sad` (falls back to `idle_a` if the species has no `sad`), static at home,
/// skull overlay in the HUD slot blinking every 4 ticks.
pub fn render_sick(stage: &StageSet, tick: u32) -> Fb {
    let mut fb = Fb::new();
    let pose = stage.sad.as_ref().unwrap_or(&stage.idle_a);
    fb.blit_or(&pose.img, HOME, 0);
    if tick % 8 < 4 {
        fb.blit_or(&EFFECTS[effect::SKULL], HUD.0, HUD.1);
    }
    fb
}

/// Sleeping scene: `sleep` pose, static, zz overlay alternating at head-right; inverts the
/// whole frame when the lights are off (so an OLED reads as mostly dark); the attention glyph
/// still shows if `sleepy` is still pending (lights not yet off).
pub fn render_sleeping(stage: &StageSet, lights_off: bool, attention: u8, tick: u32) -> Fb {
    let mut fb = Fb::new();
    fb.blit_or(&stage.sleep.img, HOME, 0);
    let zz = if tick % 8 < 4 {
        effect::ZZ_A
    } else {
        effect::ZZ_B
    };
    let (ax, ay) = head_right(&stage.sleep, HOME);
    fb.blit_or(&EFFECTS[zz], ax, ay);
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
    let elapsed = tick.wrapping_sub(anim.clip_start_tick);
    let use_eat = toggle(elapsed, 2);
    let pose = if use_eat { &stage.eat } else { &stage.idle_a };
    fb.blit_or(&pose.img, HOME, 0);
    let bite = (elapsed / 6).min(3);
    let food_frames = if snack {
        [item::SNACK_A, item::SNACK_B, item::SNACK_C]
    } else {
        [item::FOOD_A, item::FOOD_B, item::FOOD_C]
    };
    if let Some(&idx) = food_frames.get(bite as usize) {
        fb.blit_or(&ITEMS[idx], FOOD.0, FOOD.1);
    }
    fb
}

/// Refuse scene: `sad` (or `idle_a` if none) shakes, `cross` at head-right, 4 ticks.
pub fn render_refuse(stage: &StageSet, tick: u32, anim: &AnimState) -> Fb {
    let mut fb = Fb::new();
    let elapsed = tick.wrapping_sub(anim.clip_start_tick);
    let pose = stage.sad.as_ref().unwrap_or(&stage.idle_a);
    let dx = HOME + shake_dx(elapsed);
    fb.blit_or(&pose.img, dx, 0);
    let (cx, cy) = head_right(pose, dx);
    fb.blit_or(&EFFECTS[effect::CROSS], cx, cy);
    fb
}

/// Discipline's busy animation: `sad` (or `idle_a`) shakes, 6 ticks, no overlay.
pub fn render_discipline_busy(stage: &StageSet, tick: u32, anim: &AnimState) -> Fb {
    let mut fb = Fb::new();
    let elapsed = tick.wrapping_sub(anim.clip_start_tick);
    let pose = stage.sad.as_ref().unwrap_or(&stage.idle_a);
    fb.blit_or(&pose.img, HOME + shake_dx(elapsed), 0);
    fb
}

/// Result scene (end of Play): `happy` (win, hops) or `sad` (lose/draw), heart or sweat.
pub fn render_result(stage: &StageSet, won: bool, tick: u32, anim: &AnimState) -> Fb {
    let mut fb = Fb::new();
    let elapsed = tick.wrapping_sub(anim.clip_start_tick);
    if won {
        fb.blit_or(&stage.happy.img, HOME, hop_dy(elapsed));
        fb.blit_or(&EFFECTS[effect::HEART], HEAD_EFFECT.0, HEAD_EFFECT.1);
    } else {
        let pose = stage.sad.as_ref().unwrap_or(&stage.idle_a);
        fb.blit_or(&pose.img, HOME, 0);
        fb.blit_or(&EFFECTS[effect::SWEAT], HEAD_EFFECT.0, HEAD_EFFECT.1);
    }
    fb
}

/// Playing scene: pet faces left/right for the current round (`flip_h`), an `L?R` hint at the
/// top, five round-dots along the bottom filled as rounds resolve.
pub fn render_playing(stage: &StageSet, seq: u8, round: u8, tick: u32, anim: &AnimState) -> Fb {
    let mut fb = Fb::new();
    let elapsed = tick.wrapping_sub(anim.clip_start_tick);
    let facing_right = round < 5 && (seq >> round) & 1 != 0;
    let use_happy = toggle(elapsed, 2);
    let pose = if use_happy {
        &stage.happy
    } else {
        &stage.idle_a
    };
    if facing_right {
        fb.blit_or_flipped(&pose.img, HOME, 0);
    } else {
        fb.blit_or(&pose.img, HOME, 0);
    }
    draw_text(&mut fb, PLAY_HINT.0, PLAY_HINT.1, b"L?R");
    for (i, &x) in PLAY_DOT_XS.iter().enumerate() {
        if (i as u8) < round {
            fb.invert_rect(x, PLAY_DOT_Y, DOT, DOT);
        }
    }
    fb
}

/// Feed submenu: `food_a`/`snack_a` large, the selected item's block inverted, `MEAL` /
/// `SNACK` labels underneath.
pub fn render_feed_sub(snack: bool) -> Fb {
    let mut fb = Fb::new();
    fb.blit_or(&ITEMS[item::FOOD_A], FEED_SUB_FOOD.0, FEED_SUB_FOOD.1);
    fb.blit_or(&ITEMS[item::SNACK_A], FEED_SUB_SNACK.0, FEED_SUB_SNACK.1);
    let sel = if snack { FEED_SUB_SNACK } else { FEED_SUB_FOOD };
    fb.invert_rect(sel.0, sel.1, ITEM, ITEM);
    for (pos, label) in [
        (FEED_SUB_FOOD, &b"MEAL"[..]),
        (FEED_SUB_SNACK, &b"SNACK"[..]),
    ] {
        let x = pos.0 + (ITEM - text_width(label)) / 2;
        draw_text(&mut fb, x, FEED_SUB_LABEL_Y, label);
    }
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
    let (lx, ly) = STATUS_LABEL;
    match page {
        0 => {
            draw_text(&mut fb, lx, ly, b"HUNGER");
            draw_hearts(&mut fb, 100u8.saturating_sub(hunger)); // inverted: full hearts = well fed
        }
        1 => {
            draw_text(&mut fb, lx, ly, b"HAPPY");
            draw_hearts(&mut fb, happiness);
        }
        2 => {
            draw_text(&mut fb, lx, ly, b"DISCPL");
            draw_discipline_bar(&mut fb, discipline);
        }
        3 => {
            let mut buf = [0u8; 8];
            let n = write_u32(&mut buf, age_days);
            draw_text(&mut fb, lx, ly, b"AGE");
            draw_text(&mut fb, STATUS_VALUE_X, ly, &buf[0..n]);
            let mut wbuf = [0u8; 8];
            let wn = write_u32(&mut wbuf, weight as u32);
            draw_text(&mut fb, lx, HEART_Y, b"WT");
            draw_text(&mut fb, STATUS_VALUE_X, HEART_Y, &wbuf[0..wn]);
        }
        _ => {
            draw_text(&mut fb, lx, ly, b"HEALTH");
            draw_hearts(&mut fb, health);
            if sick {
                fb.blit_or(&EFFECTS[effect::SKULL], STATUS_SKULL.0, STATUS_SKULL.1);
            }
        }
    }
    draw_page_dots(&mut fb, page);
    fb
}

fn draw_hearts(fb: &mut Fb, value: u8) {
    // Four hearts, each representing 25 points.
    for (i, &x) in HEART_XS.iter().enumerate() {
        let threshold = (i as u16 + 1) * 25;
        let idx = if (value as u16) >= threshold {
            heart::FULL
        } else {
            heart::EMPTY
        };
        fb.blit_or(&HEARTS[idx], x, HEART_Y);
    }
}

fn draw_discipline_bar(fb: &mut Fb, value: u8) {
    // A 4-segment bar, one segment per 25 points.
    for (i, &x) in HEART_XS.iter().enumerate() {
        let threshold = (i as u16 + 1) * 25;
        if (value as u16) >= threshold {
            fb.invert_rect(x, BAR_Y, BAR_SEG.0, BAR_SEG.1);
        }
    }
}

fn draw_page_dots(fb: &mut Fb, page: u8) {
    for (i, &x) in PAGE_DOT_XS.iter().enumerate() {
        if i as u8 == page {
            fb.invert_rect(x, PAGE_DOT_Y - DOT, DOT, DOT * 2); // the current page's dot, doubled
        } else {
            fb.invert_rect(x, PAGE_DOT_Y, DOT, DOT);
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
/// old stage's `idle_a` at home with the whole buffer inverted every other tick for 8 ticks
/// and `sparkle_a`/`sparkle_b` alternating in opposite corners; then the new stage's `idle_a`
/// shaking for 4 ticks. 12 ticks total, mirrored by the `UiBusyEnd` timer `enter_stage` sets.
pub fn render_evolving(old: &StageSet, new: &StageSet, tick: u32, anim: &AnimState) -> Fb {
    let mut fb = Fb::new();
    let elapsed = tick.wrapping_sub(anim.clip_start_tick);
    if elapsed < 8 {
        fb.blit_or(&old.idle_a.img, HOME, 0);
        let sparkle = if toggle(elapsed, 1) {
            effect::SPARKLE_B
        } else {
            effect::SPARKLE_A
        };
        for (x, y) in SPARKLES {
            fb.blit_or(&EFFECTS[sparkle], x, y);
        }
        if elapsed % 2 == 1 {
            fb.invert();
        }
    } else {
        fb.blit_or(&new.idle_a.img, HOME + shake_dx(elapsed - 8), 0);
    }
    fb
}

/// Dead scene: the tombstone, static, a cross overlay.
pub fn render_dead() -> Fb {
    let mut fb = Fb::new();
    fb.blit_or(&TOMBSTONE, HOME, 0);
    fb.blit_or(&EFFECTS[effect::CROSS], HEAD_EFFECT.0, HEAD_EFFECT.1);
    fb
}

/// Menu scene, Tamagotchi Connection style (docs/art/SCREEN_LAYOUT.md "Menu"): four icons
/// along the top edge, four along the bottom, the selected one XOR-inverted in its block, and
/// its name centred in the band between. `cursor` is `0..8`, matching `assets::icon::*`.
pub fn render_menu(icons: &[Sprite; 8], cursor: u8) -> Fb {
    let mut fb = Fb::new();
    for (i, icon) in icons.iter().enumerate() {
        let (x, y) = MENU_SLOTS[i];
        fb.blit_or(icon, x, y);
    }
    if let Some(&(x, y)) = MENU_SLOTS.get(cursor as usize) {
        fb.invert_rect(x, y, MENU_ICON, MENU_ICON);
    }
    if let Some(label) = MENU_LABELS.get(cursor as usize) {
        draw_text(&mut fb, centred_x(label), LABEL_Y, label);
    }
    fb
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::assets::generated::{EGG, ICONS, SPECIES};

    fn set() -> &'static StageSet {
        &SPECIES[0].baby
    }

    fn lit(fb: &Fb) -> usize {
        fb.as_bytes().iter().map(|b| b.count_ones() as usize).sum()
    }

    /// Lit pixels inside a rectangle.
    fn lit_in(fb: &Fb, x0: i32, y0: i32, w: i32, h: i32) -> usize {
        let mut n = 0;
        for y in y0..y0 + h {
            for x in x0..x0 + w {
                let byte = fb.as_bytes()[(y as usize) * (Fb::W as usize / 8) + (x as usize) / 8];
                if (byte >> (7 - (x % 8))) & 1 != 0 {
                    n += 1;
                }
            }
        }
        n
    }

    #[test]
    fn egg_scene_renders_something() {
        let fb = render_egg(&EGG, 200, 0, &AnimState::new());
        assert!(lit(&fb) > 0);
    }

    #[test]
    fn idle_scene_is_deterministic_for_same_tick() {
        let anim = AnimState::new();
        let a = render_main(set(), 0, 0, 7, &anim);
        let b = render_main(set(), 0, 0, 7, &anim);
        assert_eq!(a, b);
    }

    #[test]
    fn dirty_pet_shows_the_poop_pile() {
        let anim = AnimState::new();
        let clean = render_main(set(), 0, 0, 0, &anim);
        let dirty = render_main(set(), 1, 0, 0, &anim);
        let (px, py) = POOP_PILE[0];
        assert_eq!(lit_in(&clean, px, py, ITEM, ITEM), 0);
        assert!(lit_in(&dirty, px, py, ITEM, ITEM) > 0);
    }

    #[test]
    fn sleeping_inverts_when_lights_are_off() {
        let on = render_sleeping(set(), false, 0, 0);
        let off = render_sleeping(set(), true, 0, 0);
        for (a, b) in on.as_bytes().iter().zip(off.as_bytes()) {
            assert_eq!(*a, !*b);
        }
    }

    #[test]
    fn menu_inverts_the_selected_icon_and_names_it() {
        let a = render_menu(&ICONS, 0);
        let b = render_menu(&ICONS, 1);
        assert_ne!(a, b);
        // The label band holds text for every cursor position.
        for cursor in 0..8u8 {
            let fb = render_menu(&ICONS, cursor);
            assert!(
                lit_in(&fb, 0, LABEL_Y, Fb::W, GLYPH_H) > 0,
                "no label at cursor {cursor}"
            );
        }
        // Only the selected slot's block differs between cursor 0 and cursor 1, plus the label.
        let (x0, y0) = MENU_SLOTS[0];
        let (x1, y1) = MENU_SLOTS[1];
        assert_ne!(
            lit_in(&a, x0, y0, MENU_ICON, MENU_ICON),
            lit_in(&b, x0, y0, MENU_ICON, MENU_ICON)
        );
        assert_ne!(
            lit_in(&a, x1, y1, MENU_ICON, MENU_ICON),
            lit_in(&b, x1, y1, MENU_ICON, MENU_ICON)
        );
        let (x2, y2) = MENU_SLOTS[2];
        assert_eq!(
            lit_in(&a, x2, y2, MENU_ICON, MENU_ICON),
            lit_in(&b, x2, y2, MENU_ICON, MENU_ICON)
        );
    }

    #[test]
    fn status_pages_differ() {
        let pages: [Fb; 5] =
            core::array::from_fn(|p| render_status(p as u8, 50, 50, 50, 100, false, 3, 25));
        for i in 0..5 {
            for j in (i + 1)..5 {
                assert_ne!(pages[i], pages[j], "pages {i} and {j} identical");
            }
        }
    }

    #[test]
    fn dead_scene_renders_the_tombstone() {
        assert!(lit(&render_dead()) > 0);
    }

    #[test]
    fn write_u32_formats_correctly() {
        let mut buf = [0u8; 8];
        let n = write_u32(&mut buf, 0);
        assert_eq!(&buf[0..n], b"0");
        let n = write_u32(&mut buf, 12);
        assert_eq!(&buf[0..n], b"12");
        let n = write_u32(&mut buf, 999);
        assert_eq!(&buf[0..n], b"999");
    }
}
