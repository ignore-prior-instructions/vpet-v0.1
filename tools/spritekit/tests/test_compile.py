import shutil
from pathlib import Path

import pytest

from spritekit import spec as spec_mod
from spritekit.cli import _rustfmt
from spritekit.compile import CompileError, compile_source


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


def _assets_copy(tmp_path: Path) -> Path:
    """A scratch copy of the real assets tree (minus the git-ignored preview/ dirs) so a test
    can edit species.toml without touching the repo."""
    dst = tmp_path / "assets"
    shutil.copytree(_real_assets_root(), dst, ignore=shutil.ignore_patterns("preview", "concept"))
    return dst


def _patch_species_toml(root: Path, slug: str, old: str, new: str) -> None:
    p = root / "species" / slug / "species.toml"
    text = p.read_text(encoding="utf-8")
    assert old in text, f"expected {old!r} in {p}"
    p.write_text(text.replace(old, new), encoding="utf-8")


def test_adult_alt_uses_its_own_rules_and_lifespan():
    """Regression: adult_alt used to be compiled with [stages.adult]'s rules."""
    root = _real_assets_root()
    source = compile_source(root, spec_mod.load(root))
    alt = source[source.index("adult_alt: Some(StageSet {") :]
    assert "hunger_step_secs: 300" in alt
    assert "lifespan_min_secs: 345600, lifespan_max_secs: 691200" in alt
    adult = source[source.index("adult: StageSet {") : source.index("adult_alt: Some(StageSet {")]
    assert "lifespan_min_secs: 432000, lifespan_max_secs: 864000" in adult


def test_child_branches_are_emitted_in_file_order():
    root = _real_assets_root()
    source = compile_source(root, spec_mod.load(root))
    child = source[source.index("child: StageSet {") : source.index("adult: StageSet {")]
    first = child.index("Branch { to: crate::pet::Stage::Adult, to_species: None, max_care_mistakes: Some(4), min_discipline: Some(40)")
    second = child.index("Branch { to: crate::pet::Stage::AdultAlt, to_species: None, max_care_mistakes: None")
    assert first < second
    # baby and adult have no branches
    baby = source[source.index("baby: StageSet {") : source.index("child: StageSet {")]
    assert "evolve: &[]," in baby


def test_branch_to_unknown_species_errors(tmp_path):
    root = _assets_copy(tmp_path)
    _patch_species_toml(root, "lalafu", 'to = "adult_alt"', 'to = "adult_alt"\nto_species = "nope"')
    with pytest.raises(CompileError, match="to_species 'nope' is not in registry.toml"):
        compile_source(root, spec_mod.load(root))


def test_branch_to_adult_alt_without_the_stage_errors(tmp_path):
    root = _assets_copy(tmp_path)
    _patch_species_toml(root, "lalafu", "adult_alt     = true", "adult_alt     = false")
    with pytest.raises(CompileError, match="targets adult_alt but species.toml has \\[assets\\] adult_alt = false"):
        compile_source(root, spec_mod.load(root))


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
