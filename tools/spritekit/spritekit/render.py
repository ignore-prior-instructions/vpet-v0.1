"""Preview renderer. Upscales text grids to PNG so a human can look at what a model (or a
person) drew. See docs/art/ART_PIPELINE.md "Preview renderer". Phase 0/1 scope: `--sheet` and a
single `--pose`, styles `oled`/`lcd`. `--anim`/`--gif`/`--scene` join with the animation system
in Phase 3.
"""

from __future__ import annotations

from pathlib import Path

from PIL import Image, ImageChops, ImageDraw, ImageFilter

from .grid import ON_CHARS, GridFile, parse

STYLES = {
    "oled": {"bg": (0, 0, 0), "fg": (159, 232, 255), "text": (220, 220, 220)},
    "lcd": {"bg": (184, 200, 160), "fg": (26, 36, 24), "text": (26, 36, 24)},
}


def render_sprite_image(rows: list[str], w: int, h: int, style: str = "oled", scale: int = 8, gap: int = 1) -> Image.Image:
    palette = STYLES[style]
    bg, fg = palette["bg"], palette["fg"]
    img_w = w * (scale + gap) + gap
    img_h = h * (scale + gap) + gap
    base = Image.new("RGB", (img_w, img_h), bg)
    draw = ImageDraw.Draw(base)
    for y, row in enumerate(rows):
        for x, c in enumerate(row):
            if c in ON_CHARS:
                x0 = gap + x * (scale + gap)
                y0 = gap + y * (scale + gap)
                draw.rectangle([x0, y0, x0 + scale - 1, y0 + scale - 1], fill=fg)
    if style == "oled":
        blurred = base.filter(ImageFilter.GaussianBlur(radius=max(1.0, scale / 3)))
        base = ImageChops.screen(base, blurred)
    return base


def render_sheet(gf: GridFile, style: str = "oled", scale: int = 8) -> Image.Image:
    palette = STYLES[style]
    label_h = 14
    pad = 8
    imgs = [render_sprite_image(s.rows, gf.cell_w, gf.cell_h, style, scale) for s in gf.sprites]
    if not imgs:
        return Image.new("RGB", (200, 60), palette["bg"])
    total_w = sum(im.width for im in imgs) + pad * (len(imgs) + 1)
    height = max(im.height for im in imgs) + label_h + pad * 2
    sheet = Image.new("RGB", (total_w, height), palette["bg"])
    draw = ImageDraw.Draw(sheet)
    x = pad
    for sprite, im in zip(gf.sprites, imgs):
        sheet.paste(im, (x, pad))
        draw.text((x, pad + im.height + 2), sprite.name, fill=palette["text"])
        x += im.width + pad
    return sheet


def render_file(
    path: Path,
    *,
    pose: str | None = None,
    sheet: bool = True,
    style: str = "oled",
    scale: int = 8,
) -> Path:
    if style not in STYLES:
        raise ValueError(f"unknown style {style!r}, expected one of {sorted(STYLES)}")
    gf = parse(str(path), path.read_text(encoding="utf-8"))
    preview_dir = path.parent / "preview"
    preview_dir.mkdir(parents=True, exist_ok=True)

    if pose is not None:
        try:
            sprite = next(s for s in gf.sprites if s.name == pose)
        except StopIteration:
            names = [s.name for s in gf.sprites]
            raise ValueError(f"no pose/sprite named {pose!r} in {path}; have {names}") from None
        img = render_sprite_image(sprite.rows, gf.cell_w, gf.cell_h, style, scale)
        out = preview_dir / f"{path.stem}_{pose}_{style}.png"
    else:
        img = render_sheet(gf, style, scale)
        out = preview_dir / f"{path.stem}_sheet_{style}.png"

    img.save(out)
    return out
