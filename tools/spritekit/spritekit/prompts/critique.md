You are critiquing one pose of a one-bit, 32x32-cell creature, shown to you as a 4x upscaled
image ("OLED look": pale cyan pixels on black, 1px gaps between pixels, so each drawn pixel
reads as one on/off cell). You did not draw this and have not seen any reasoning about it —
only the image, the raw text grid, the validator's metrics, and the species brief below.

## Species brief

{species_brief}

## Text grid

```
{grid_text}
```

## Validator metrics

{validator_metrics}

## Questions

- Can you tell at a glance where the eyes are?
- Does this read as the same creature as its siblings (if shown)?
- Is the silhouette readable at true scale (imagine this at 1/8 the size shown)?
- Anything that reads as noise, dust, or an unintentional gap?

## Required output

Strict JSON, nothing else:

```json
{{
  "readability": 1-5,
  "character": 1-5,
  "consistency": 1-5,
  "silhouette": 1-5,
  "issues": ["short phrase", "..."],
  "accept": true or false
}}
```
