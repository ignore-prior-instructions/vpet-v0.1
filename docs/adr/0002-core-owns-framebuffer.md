# 0002: The core owns the framebuffer

Status: accepted
Date: 2026-09-20

## Context

The core could emit a scene description (pet id, pose, x, effects) and let each host draw, or it
could render to a 32x16 bit buffer and let hosts copy pixels.

## Decision

The core renders. Hosts receive 64 bytes and blit them, upscaled, to whatever they have.

## Consequences

- Three hosts do not each reimplement compositing, the 3x5 font, menus, and HUD, and they cannot
  drift.
- Parity is "compare 64 bytes".
- Hosts cannot restyle the screen beyond palette and scale. The screen is fixed at 32x16, so
  there is nothing to lose.
- Sprites and the font must be compiled into the core (see 0009).
