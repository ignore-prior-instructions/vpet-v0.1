# 0012: 32x16 screen, 16x16 pets, 8x8 icons, 3x5 font

Status: superseded by [0016](0016-screen-64x32.md)
Date: 2026-09-20

## Context

v0 used 32x20. The Gen-1 devices were 32x16. The cheapest common hobby display, a 128x64
SSD1306 OLED, is exactly 32x16 at 4x. Cells that are multiples of 8 pack into whole bytes.

## Decision

The screen is 32x16, one bit per pixel. Pets are 16x16 cells, bottom-anchored. Icons, items, and
effects are 8x8. The font is 3x5 with a 4 px advance. Splash screens are 32x16.

## Consequences

- The OLED is filled exactly with 4x4 blocks; the browser scales by CSS.
- v0's 32x20 sprites are not reusable; they were being replaced anyway.
- The menu is a 4x2 grid of icons that tiles the screen with no layout code.
