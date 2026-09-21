//! Draws 3x5 glyphs from `assets::generated::FONT_3X5`. See docs/art/SPRITE_FORMAT.md's "Font
//! glyph" row: 3x5, advance 4 (1px gap between glyphs).

use super::fb::Fb;
use crate::assets::generated::FONT_3X5;
use crate::assets::glyph_index;

const ADVANCE: i32 = 4;

/// Draws `text` (ASCII, only characters `glyph_index` knows map to a glyph; anything else is
/// skipped, still advancing the cursor so alignment isn't thrown off by an unsupported byte).
pub fn draw_text(fb: &mut Fb, x: i32, y: i32, text: &[u8]) {
    let mut cx = x;
    for &c in text {
        if let Some(i) = glyph_index(c) {
            fb.blit_or(&FONT_3X5[i as usize], cx, y);
        }
        cx += ADVANCE;
    }
}

/// Total pixel width `draw_text` would use for `text` (for right-aligning or centring).
pub fn text_width(text: &[u8]) -> i32 {
    if text.is_empty() {
        0
    } else {
        text.len() as i32 * ADVANCE - 1 // no trailing gap after the last glyph
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn draws_something_for_known_letters() {
        let mut fb = Fb::new();
        draw_text(&mut fb, 0, 0, b"HI");
        assert!(fb.as_bytes().iter().any(|&b| b != 0));
    }

    #[test]
    fn unsupported_bytes_are_skipped_but_still_advance() {
        let mut fb = Fb::new();
        draw_text(&mut fb, 0, 0, b"H"); // baseline
        let with_gap = {
            let mut fb2 = Fb::new();
            draw_text(&mut fb2, 0, 0, b"H_H"); // '_' has no glyph
            fb2
        };
        assert_ne!(fb.as_bytes(), with_gap.as_bytes()); // the second H moved over
    }

    #[test]
    fn text_width_matches_advance() {
        assert_eq!(text_width(b""), 0);
        assert_eq!(text_width(b"A"), 3);
        assert_eq!(text_width(b"AB"), 7);
    }
}
