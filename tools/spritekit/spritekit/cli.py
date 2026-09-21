"""`spritekit` command-line entry point. See docs/art/ART_PIPELINE.md for the full command set;
Phase 0/1 implements `validate`, `render`, and the throwaway `pack` helper used to seed the
hand-written Phase 1 `generated.rs` with real sprite bytes. `gen`, `concept`, `quantize`,
`mark-eyes`, `diff`, `approve`, `compile`, `new-species` land in Phase 3.
"""

from __future__ import annotations

import json as json_mod
import sys
from pathlib import Path

import click

from . import spec as spec_mod
from .grid import pack_image, pack_mask, parse
from .render import render_file
from .validate import validate_all, validate_file


@click.group()
def main():
    """Text-grid sprite tooling for vpet-v0.1."""


@main.command()
@click.argument("path", required=False, type=click.Path(path_type=Path))
@click.option("--all", "all_", is_flag=True, help="Validate every asset under assets/.")
@click.option("--json", "as_json", is_flag=True, help="Print one JSON report per sprite.")
def validate(path: Path | None, all_: bool, as_json: bool):
    """Validate a sprite file (or --all of them). Non-zero exit on any error."""
    if not all_ and path is None:
        raise click.UsageError("provide a PATH or pass --all")

    root = spec_mod.find_assets_root()
    sp = spec_mod.load(root)
    reports = validate_all(root, sp) if all_ else validate_file(path, sp)

    if as_json:
        click.echo(json_mod.dumps([r.to_json() for r in reports], indent=2))
    else:
        for r in reports:
            label = f"{r.file}::{r.sprite}" if r.sprite else r.file
            if r.ok and not r.warnings:
                click.echo(f"ok    {label}")
                continue
            for e in r.errors:
                click.echo(f"ERROR {label}: {e}")
            for w in r.warnings:
                click.echo(f"warn  {label}: {w}")

    n_errors = sum(len(r.errors) for r in reports)
    n_files = len({r.file for r in reports})
    click.echo(f"{n_files} file(s), {len(reports)} sprite(s), {n_errors} error(s)", err=True)
    sys.exit(1 if n_errors else 0)


@main.command(name="render")
@click.argument("path", type=click.Path(exists=True, path_type=Path))
@click.option("--pose", default=None, help="Render only this pose/sprite name.")
@click.option("--sheet", is_flag=True, default=False, help="Render every pose side by side.")
@click.option("--style", type=click.Choice(["oled", "lcd"]), default="oled")
@click.option("--scale", default=8, show_default=True)
def render_cmd(path: Path, pose: str | None, sheet: bool, style: str, scale: int):
    """Render PATH to a PNG under its preview/ directory."""
    out = render_file(path, pose=pose, sheet=sheet or pose is None, style=style, scale=scale)
    click.echo(str(out))


def _rust_byte_array(b: bytes) -> str:
    return "[" + ", ".join(f"0x{byte:02X}" for byte in b) + "]"


@main.command()
@click.argument("path", type=click.Path(exists=True, path_type=Path))
def pack(path: Path):
    """Print Rust byte arrays for every sprite in PATH.

    A throwaway helper for Phase 1 (before `compile` exists in Phase 3): copy its output into
    the hand-written `crates/vpet-core/src/assets/generated.rs` placeholder so sprite bytes come
    from real, validated text grids rather than being typed by hand.
    """
    gf = parse(str(path), path.read_text(encoding="utf-8"))
    for s in gf.sprites:
        img = pack_image(s.rows, gf.cell_w)
        mask = pack_mask(s.rows, gf.cell_w)
        click.echo(f"// {s.name} ({gf.cell_w}x{gf.cell_h})")
        click.echo(f"pub static {s.name.upper()}_IMG: [u8; {len(img)}] = {_rust_byte_array(img)};")
        if mask is not None:
            click.echo(
                f"pub static {s.name.upper()}_MASK: [u8; {len(mask)}] = {_rust_byte_array(mask)};"
            )
        click.echo()


if __name__ == "__main__":
    main()
