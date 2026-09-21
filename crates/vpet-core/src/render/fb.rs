//! The 256-byte framebuffer and its blit operations. See docs/HOST_ABI.md "Frame format":
//! 64x32, one bit per pixel, row-major, 8 bytes per row, MSB is the leftmost pixel
//! (docs/adr/0016-screen-64x32.md).

use crate::assets::Sprite;

/// Bytes per frame: `W / 8 * H`. The one number every host's blitter and the ABI's `FRAME`
/// buffer must agree on.
pub const FRAME_LEN: usize = 256;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Fb(pub [u8; FRAME_LEN]);

impl Fb {
    pub const W: i32 = 64;
    pub const H: i32 = 32;
    const STRIDE: usize = 8;

    pub const fn new() -> Self {
        Fb([0u8; FRAME_LEN])
    }

    pub fn clear(&mut self) {
        self.0 = [0u8; FRAME_LEN];
    }

    pub const fn as_bytes(&self) -> &[u8; FRAME_LEN] {
        &self.0
    }

    fn get(&self, x: i32, y: i32) -> bool {
        if !(0..Self::W).contains(&x) || !(0..Self::H).contains(&y) {
            return false;
        }
        let byte = y as usize * Self::STRIDE + (x as usize) / 8;
        let bit = 7 - (x as usize % 8);
        (self.0[byte] >> bit) & 1 != 0
    }

    fn set(&mut self, x: i32, y: i32, v: bool) {
        if !(0..Self::W).contains(&x) || !(0..Self::H).contains(&y) {
            return;
        }
        let byte = y as usize * Self::STRIDE + (x as usize) / 8;
        let bit = 7 - (x as usize % 8);
        if v {
            self.0[byte] |= 1 << bit;
        } else {
            self.0[byte] &= !(1 << bit);
        }
    }

    fn blit_op_ex(
        &mut self,
        sprite: &Sprite,
        x: i32,
        y: i32,
        flip_h: bool,
        op: impl Fn(bool, bool) -> bool,
    ) {
        let stride = (sprite.w as usize).div_ceil(8);
        for row in 0..sprite.h as i32 {
            let py = y + row;
            if !(0..Self::H).contains(&py) {
                continue;
            }
            for col in 0..sprite.w as i32 {
                let px = x + col;
                if !(0..Self::W).contains(&px) {
                    continue;
                }
                let src_col = if flip_h {
                    sprite.w as i32 - 1 - col
                } else {
                    col
                } as usize;
                let byte_i = row as usize * stride + src_col / 8;
                let bit_i = 7 - (src_col % 8);
                let Some(&byte) = sprite.rows.get(byte_i) else {
                    continue;
                };
                let src = (byte >> bit_i) & 1 != 0;
                let cur = self.get(px, py);
                self.set(px, py, op(cur, src));
            }
        }
    }

    fn blit_op(&mut self, sprite: &Sprite, x: i32, y: i32, op: impl Fn(bool, bool) -> bool) {
        self.blit_op_ex(sprite, x, y, false, op);
    }

    /// OR a sprite onto the buffer (ordinary draw). Clips at every edge.
    pub fn blit_or(&mut self, sprite: &Sprite, x: i32, y: i32) {
        self.blit_op(sprite, x, y, |dst, src| dst || src);
    }

    /// OR a sprite onto the buffer, mirrored horizontally (docs/art/ANIMATION.md `flip_h`: the
    /// Play scene facing left/right, Battle facing the opponent).
    pub fn blit_or_flipped(&mut self, sprite: &Sprite, x: i32, y: i32) {
        self.blit_op_ex(sprite, x, y, true, |dst, src| dst || src);
    }

    /// XOR a sprite onto the buffer (blink masks, and cursor inversion via `invert_rect`).
    pub fn blit_xor(&mut self, sprite: &Sprite, x: i32, y: i32) {
        self.blit_op(sprite, x, y, |dst, src| dst ^ src);
    }

    /// XOR every pixel in the whole buffer (docs/art/SCREEN_LAYOUT.md: sleeping with lights
    /// off, evolution flash).
    pub fn invert(&mut self) {
        for b in self.0.iter_mut() {
            *b = !*b;
        }
    }

    /// XOR a rectangular region (the selected menu icon).
    pub fn invert_rect(&mut self, x: i32, y: i32, w: i32, h: i32) {
        for row in 0..h {
            let py = y + row;
            if !(0..Self::H).contains(&py) {
                continue;
            }
            for col in 0..w {
                let px = x + col;
                if !(0..Self::W).contains(&px) {
                    continue;
                }
                let cur = self.get(px, py);
                self.set(px, py, !cur);
            }
        }
    }
}

impl Default for Fb {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sprite2x2_full() -> Sprite {
        // 2x2, all four pixels on: row stride = 1 byte, top two bits of each byte set.
        static ROWS: [u8; 2] = [0b1100_0000, 0b1100_0000];
        Sprite {
            w: 2,
            h: 2,
            rows: &ROWS,
        }
    }

    #[test]
    fn blit_or_sets_pixels() {
        let mut fb = Fb::new();
        fb.blit_or(&sprite2x2_full(), 0, 0);
        assert!(fb.get(0, 0));
        assert!(fb.get(1, 0));
        assert!(fb.get(0, 1));
        assert!(fb.get(1, 1));
        assert!(!fb.get(2, 0));
    }

    #[test]
    fn blit_clips_negative_x() {
        let mut fb = Fb::new();
        // Sprite's left column at x = -1 falls off the left edge; only the right column should
        // land, at x = 0.
        fb.blit_or(&sprite2x2_full(), -1, 0);
        assert!(fb.get(0, 0));
        assert!(!fb.get(1, 0)); // would be x=1 for the left col, out of range at x=-1+1=0... see below
    }

    #[test]
    fn blit_clips_right_edge() {
        let mut fb = Fb::new();
        fb.blit_or(&sprite2x2_full(), Fb::W - 1, 0); // only the leftmost column is in range
        assert!(fb.get(Fb::W - 1, 0));
    }

    #[test]
    fn invert_flips_every_pixel() {
        let mut fb = Fb::new();
        fb.blit_or(&sprite2x2_full(), 0, 0);
        fb.invert();
        assert!(!fb.get(0, 0));
        assert!(fb.get(2, 0)); // was off, now on
    }

    #[test]
    fn invert_rect_flips_only_region() {
        let mut fb = Fb::new();
        fb.invert_rect(0, 0, 8, 8);
        assert!(fb.get(0, 0));
        assert!(fb.get(7, 7));
        assert!(!fb.get(8, 0));
    }

    #[test]
    fn xor_blit_toggles() {
        let mut fb = Fb::new();
        fb.blit_or(&sprite2x2_full(), 0, 0);
        fb.blit_xor(&sprite2x2_full(), 0, 0);
        assert!(!fb.get(0, 0)); // XORed back off
    }

    #[test]
    fn flipped_blit_mirrors_columns() {
        // An L-shape: on at (0,0) only.
        static ROWS: [u8; 2] = [0b1000_0000, 0b0000_0000];
        let sprite = Sprite {
            w: 2,
            h: 2,
            rows: &ROWS,
        };
        let mut fb = Fb::new();
        fb.blit_or_flipped(&sprite, 0, 0);
        assert!(!fb.get(0, 0)); // was column 0, now mirrored to column 1
        assert!(fb.get(1, 0));
    }
}
