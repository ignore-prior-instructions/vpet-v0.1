//! The 64-byte framebuffer and its blit operations. See docs/HOST_ABI.md "Frame format":
//! 32x16, one bit per pixel, row-major, 4 bytes per row, MSB is the leftmost pixel.

use crate::assets::Sprite;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Fb(pub [u8; 64]);

impl Fb {
    pub const W: i32 = 32;
    pub const H: i32 = 16;
    const STRIDE: usize = 4;

    pub const fn new() -> Self {
        Fb([0u8; 64])
    }

    pub fn clear(&mut self) {
        self.0 = [0u8; 64];
    }

    pub const fn as_bytes(&self) -> &[u8; 64] {
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

    fn blit_op(&mut self, sprite: &Sprite, x: i32, y: i32, op: impl Fn(bool, bool) -> bool) {
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
                let byte_i = row as usize * stride + (col as usize) / 8;
                let bit_i = 7 - (col as usize % 8);
                let Some(&byte) = sprite.rows.get(byte_i) else {
                    continue;
                };
                let src = (byte >> bit_i) & 1 != 0;
                let cur = self.get(px, py);
                self.set(px, py, op(cur, src));
            }
        }
    }

    /// OR a sprite onto the buffer (ordinary draw). Clips at every edge.
    pub fn blit_or(&mut self, sprite: &Sprite, x: i32, y: i32) {
        self.blit_op(sprite, x, y, |dst, src| dst || src);
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
        fb.blit_or(&sprite2x2_full(), 31, 0); // only the leftmost column (x=31) is in range
        assert!(fb.get(31, 0));
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
}
