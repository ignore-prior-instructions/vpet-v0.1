# 0015: Blink as an XOR mask in the sprite alphabet

Status: accepted
Date: 2026-09-20

## Context

Blinking needs an "eyes closed" variant of every idle pose. Drawing it doubles the frame count
and is the kind of near-duplicate a generator gets subtly wrong.

## Decision

The text alphabet has two extra characters: `o` (on, off during blink) and `-` (off, on during
blink). They compile to a per-pose XOR mask. The core blinks by drawing `img ^ mask` for one
tick on a randomised schedule from a separate animation RNG.

## Consequences

- Eye positions are explicit in the source, which the validator uses for symmetry, placement,
  and cross-pose eye drift checks.
- Zero extra drawn frames.
- Poses without marks simply never blink.
