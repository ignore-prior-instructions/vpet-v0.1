"""`spritekit` command-line entry point. See docs/art/ART_PIPELINE.md for the full command set.
`validate`, `render`, `compile`, `approve`, `new-species`, `diff`, `mark-eyes`, and `gen` are
implemented (Phase 3); `concept` and `quantize` (image-model paths, not needed for a
text-grid-only species) are not yet. `pack` is a Phase 1 throwaway helper, superseded by
`compile` but harmless to keep around for one-off byte dumps.
"""

from __future__ import annotations

import json as json_mod
import shutil
import subprocess
import sys
import tempfile
from pathlib import Path

import click

from . import spec as spec_mod
from .approve import approve_species
from .compile import CompileError, compile_source
from .diff import diff_poses
from .grid import pack_image, pack_mask, parse
from .mark_eyes import propose_eyes
from .new_species import new_species as new_species_fn
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
@click.option("--scale", default=4, show_default=True)
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


@main.command(name="dev-fixture")
@click.option(
    "--out",
    "out_path",
    type=click.Path(path_type=Path),
    default=None,
    help="Output path (default: tests/fixtures/packed.json under the repo root).",
)
def dev_fixture(out_path: Path | None):
    """Dump every sprite's packed bytes as JSON, keyed by `<relpath>::<name>`.

    docs/CONTENT.md "Hot preview without rebuilding the core": the parser-parity test for
    `hosts/web/src/dev/grid.ts` (a TS port of this module's `parse`/`pack_image`/`pack_mask`)
    compares its output against this file's, so the two parsers can never silently drift. Run
    after any change to `grid.py`'s parsing or packing logic; re-run and re-commit alongside it.
    """
    root = spec_mod.find_assets_root()
    out = out_path or (root.parent / "tests" / "fixtures" / "packed.json")
    out.parent.mkdir(parents=True, exist_ok=True)

    fixture: dict[str, dict] = {}

    def dump_file(path: Path) -> None:
        gf = parse(str(path), path.read_text(encoding="utf-8"))
        rel = path.relative_to(root).as_posix()
        for s in gf.sprites:
            img = pack_image(s.rows, gf.cell_w)
            mask = pack_mask(s.rows, gf.cell_w)
            fixture[f"{rel}::{s.name}"] = {
                "w": gf.cell_w,
                "h": gf.cell_h,
                "img": img.hex(),
                "mask": mask.hex() if mask is not None else None,
            }

    global_dir = root / "global"
    if global_dir.is_dir():
        for path in sorted(global_dir.glob("*.txt")):
            dump_file(path)
    species_dir = root / "species"
    if species_dir.is_dir():
        for slug_dir in sorted(p for p in species_dir.iterdir() if p.is_dir()):
            for stage_file in sorted(slug_dir.glob("*.txt")):
                dump_file(stage_file)

    out.write_text(json_mod.dumps(fixture, indent=2, sort_keys=True) + "\n", encoding="utf-8")
    click.echo(f"{len(fixture)} sprite(s) -> {out}")


@main.command(name="new-species")
@click.argument("slug")
@click.option("--name", default=None, help="Display name (default: Slug.capitalize()).")
def new_species_cmd(slug: str, name: str | None):
    """Scaffold a new species: registry.toml entry, species.toml, stage file stubs, BRIEF.md."""
    root = spec_mod.find_assets_root()
    try:
        species_dir = new_species_fn(root, slug, name)
    except (ValueError, FileExistsError) as e:
        raise click.ClickException(str(e))
    click.echo(f"created {species_dir}")


@main.command()
@click.argument("path_a", type=click.Path(exists=True, path_type=Path))
@click.argument("pose_a")
@click.argument("path_b", type=click.Path(exists=True, path_type=Path))
@click.argument("pose_b")
def diff(path_a: Path, pose_a: str, path_b: Path, pose_b: str):
    """Compare POSE_A in PATH_A against POSE_B in PATH_B: xor count, bbox and centroid deltas."""
    gf_a = parse(str(path_a), path_a.read_text(encoding="utf-8"))
    gf_b = parse(str(path_b), path_b.read_text(encoding="utf-8"))
    sa = next((s for s in gf_a.sprites if s.name == pose_a), None)
    sb = next((s for s in gf_b.sprites if s.name == pose_b), None)
    if sa is None:
        raise click.ClickException(f"no pose {pose_a!r} in {path_a}")
    if sb is None:
        raise click.ClickException(f"no pose {pose_b!r} in {path_b}")
    if (gf_a.cell_w, gf_a.cell_h) != (gf_b.cell_w, gf_b.cell_h):
        raise click.ClickException("poses have different cell sizes")
    result = diff_poses(sa.rows, sb.rows, gf_a.cell_w, gf_a.cell_h)
    click.echo(f"xor: {result.xor_count} px")
    click.echo(f"bbox a: {result.bbox_a}  b: {result.bbox_b}")
    click.echo(f"centroid a: {result.centroid_a}  b: {result.centroid_b}")


@main.command(name="mark-eyes")
@click.argument("path", type=click.Path(exists=True, path_type=Path))
@click.argument("pose")
def mark_eyes_cmd(path: Path, pose: str):
    """Propose mirrored eye-hole positions for POSE in PATH (never edits the file)."""
    gf = parse(str(path), path.read_text(encoding="utf-8"))
    sprite = next((s for s in gf.sprites if s.name == pose), None)
    if sprite is None:
        raise click.ClickException(f"no pose {pose!r} in {path}")
    proposals = propose_eyes(sprite.rows, gf.cell_w, gf.cell_h)
    if not proposals:
        click.echo("no candidates found")
        return
    for p in proposals:
        click.echo(f"({p.x},{p.y}) <-> ({p.mirror_x},{p.y})")


@main.command()
@click.argument("slug")
@click.argument("stage")
@click.argument("pose")
@click.option("--model", default="claude-sonnet-5")
def gen(slug: str, stage: str, pose: str, model: str):
    """Generate one pose via the model API (docs/art/ART_PIPELINE.md). Requires
    ANTHROPIC_API_KEY."""
    from .gen import append_gen_log, gen_pose
    from .model_client import AnthropicClient, MissingApiKeyError

    root = spec_mod.find_assets_root()
    sp = spec_mod.load(root)
    species_dir = root / "species" / slug
    brief_path = species_dir / "BRIEF.md"
    brief = brief_path.read_text(encoding="utf-8") if brief_path.exists() else slug
    style_guide_path = root.parent / "docs" / "art" / "STYLE_GUIDE.md"
    style_guide = style_guide_path.read_text(encoding="utf-8") if style_guide_path.exists() else ""

    try:
        client = AnthropicClient(model=model)
    except MissingApiKeyError as e:
        raise click.ClickException(str(e))

    result = gen_pose(client, root, sp, slug, stage, pose, brief, style_guide)
    append_gen_log(species_dir, result)
    if result.accepted:
        stage_file = species_dir / f"{stage}.txt"
        click.echo(f"accepted after {result.rounds} round(s); merge into {stage_file} by hand and re-validate")
        click.echo(result.grid_text)
    else:
        click.echo(f"not accepted after {result.rounds} round(s); see {species_dir / 'gen-log.jsonl'}", err=True)
        click.echo(result.grid_text)
        sys.exit(1)


@main.command()
@click.argument("slug_stage")
def approve(slug_stage: str):
    """Approve every pose of SLUG (or SLUG/STAGE), hashing them into assets/approved.lock."""
    root = spec_mod.find_assets_root()
    if "/" in slug_stage:
        slug, stage = slug_stage.split("/", 1)
    else:
        slug, stage = slug_stage, None
    changed = approve_species(root, slug, stage)
    if changed:
        click.echo(f"approved/updated {len(changed)} pose(s):")
        for key in changed:
            click.echo(f"  {key}")
    else:
        click.echo("nothing changed")


def _rustfmt(source: str) -> str:
    """Runs `rustfmt` over generated source so the checked-in file matches what `cargo fmt
    --check` expects. Falls back to the unformatted source (with a warning) if rustfmt isn't on
    PATH — compile.py's output is already valid Rust either way, just not pretty."""
    rustfmt = shutil.which("rustfmt")
    if rustfmt is None:
        click.echo("warning: rustfmt not found on PATH; writing unformatted output", err=True)
        return source
    with tempfile.NamedTemporaryFile("w", suffix=".rs", delete=False) as f:
        f.write(source)
        tmp_path = Path(f.name)
    try:
        subprocess.run([rustfmt, "--edition", "2021", str(tmp_path)], check=True, capture_output=True, text=True)
        return tmp_path.read_text(encoding="utf-8")
    except subprocess.CalledProcessError as e:
        raise CompileError(f"rustfmt failed on generated output:\n{e.stderr}") from e
    finally:
        tmp_path.unlink(missing_ok=True)


@main.command()
@click.option("--check", is_flag=True, help="Don't write; fail if the output would differ from the existing file.")
@click.option("--allow-unapproved", is_flag=True, help="Skip the approved.lock check (dev builds only).")
@click.option("--out", "out_path", type=click.Path(path_type=Path), default=None)
def compile(check: bool, allow_unapproved: bool, out_path: Path | None):
    """Compile assets/ into crates/vpet-core/src/assets/generated.rs."""
    root = spec_mod.find_assets_root()
    sp = spec_mod.load(root)
    if out_path is None:
        out_path = root.parent / "crates" / "vpet-core" / "src" / "assets" / "generated.rs"

    try:
        source = compile_source(root, sp, allow_unapproved=allow_unapproved)
        source = _rustfmt(source)
    except CompileError as e:
        click.echo(f"compile failed: {e}", err=True)
        sys.exit(1)

    if check:
        existing = out_path.read_text(encoding="utf-8") if out_path.exists() else None
        if existing != source:
            click.echo(f"{out_path} is stale; run `spritekit compile` to regenerate", err=True)
            sys.exit(1)
        click.echo(f"{out_path} is up to date")
        return

    out_path.write_text(source, encoding="utf-8")
    click.echo(f"wrote {out_path}")


if __name__ == "__main__":
    main()
