from pathlib import Path

from spritekit import spec as spec_mod
from spritekit.cli import _rustfmt
from spritekit.compile import compile_source


def _real_assets_root() -> Path:
    # tests/ -> tools/spritekit/ -> tools/ -> repo root -> assets/
    return Path(__file__).resolve().parents[3] / "assets"


def test_compile_is_byte_stable_across_runs():
    root = _real_assets_root()
    sp = spec_mod.load(root)
    a = compile_source(root, sp)
    b = compile_source(root, sp)
    assert a == b


def test_compile_output_is_nonempty_and_mentions_every_species():
    root = _real_assets_root()
    sp = spec_mod.load(root)
    source = compile_source(root, sp)
    assert "pub static SPECIES" in source
    assert "pub static GAME" in source
    assert "lalafu" in source


def test_compile_matches_checked_in_generated_rs():
    """Guards against someone hand-editing generated.rs or forgetting to re-run compile after
    an asset change: the same comparison `spritekit compile --check` (and `just art-check`) do,
    exercised directly here rather than via a subprocess."""
    root = _real_assets_root()
    sp = spec_mod.load(root)
    source = _rustfmt(compile_source(root, sp))
    generated = root.parent / "crates" / "vpet-core" / "src" / "assets" / "generated.rs"
    on_disk = generated.read_text(encoding="utf-8")
    assert source == on_disk
