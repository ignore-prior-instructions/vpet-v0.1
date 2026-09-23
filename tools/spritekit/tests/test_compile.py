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


def test_ninjifu_and_charamofu_adults_use_their_own_rules_and_lifespan():
    """Regression: lalafu used to compile a single species with adult/adult_alt as two stages
    of the same SpeciesDef; docs/adr/0017 replaced that with a `to_species` jump to two
    separate species (ninjifu the "good care" adult, charamofu the "bad care" one), each with
    its own [stages.adult] rules rather than sharing lalafu's."""
    root = _real_assets_root()
    source = compile_source(root, spec_mod.load(root))
    species_blocks = source.split("SpeciesDef {")[1:]  # [lalafu, charamofu, ninjifu] (registry order)
    charamofu_adult = species_blocks[1][: species_blocks[1].index("adult_alt:")]
    assert "hunger_step_secs: 300" in charamofu_adult
    assert "lifespan_min_secs: 345600, lifespan_max_secs: 691200" in charamofu_adult
    ninjifu_adult = species_blocks[2][: species_blocks[2].index("adult_alt:")]
    assert "hunger_step_secs: 360" in ninjifu_adult
    assert "lifespan_min_secs: 432000, lifespan_max_secs: 864000" in ninjifu_adult


def test_child_branches_are_emitted_in_file_order():
    root = _real_assets_root()
    source = compile_source(root, spec_mod.load(root))
    lalafu = source[source.index("SpeciesDef {") : source.index("SpeciesDef {", source.index("SpeciesDef {") + 1)]
    child = lalafu[lalafu.index("child: StageSet {") : lalafu.index("adult: StageSet {")]
    # ninjifu=3, charamofu=2 (registry.toml). Three branches in lalafu/species.toml, strictest
    # first (the core takes the first match): fed+disciplined -> ninjifu, fed -> lalafu's own
    # adult (to_species None), unconditional fallback -> charamofu.
    first = child.index(
        "Branch { to: crate::pet::Stage::Adult, to_species: Some(3), "
        "max_care_mistakes: Some(4), min_discipline: Some(40)"
    )
    middle = child.index(
        "Branch { to: crate::pet::Stage::Adult, to_species: None, max_care_mistakes: Some(4), min_discipline: None"
    )
    second = child.index("Branch { to: crate::pet::Stage::Adult, to_species: Some(2), max_care_mistakes: None")
    assert first < middle < second
    # baby has no branches.
    baby = lalafu[lalafu.index("baby: StageSet {") : lalafu.index("child: StageSet {")]
    assert "evolve: &[]," in baby


def test_branch_to_unknown_species_errors(tmp_path):
    root = _assets_copy(tmp_path)
    _patch_species_toml(root, "lalafu", 'to_species = "ninjifu"', 'to_species = "nope"')
    with pytest.raises(CompileError, match="to_species 'nope' is not in registry.toml"):
        compile_source(root, spec_mod.load(root))


def test_branch_to_adult_alt_without_the_stage_errors(tmp_path):
    root = _assets_copy(tmp_path)
    _patch_species_toml(
        root,
        "lalafu",
        'to_species = "charamofu"\n\n[stages.adult]',
        'to_species = "charamofu"\n\n[[stages.child.evolve.branch]]\nto = "adult_alt"\n\n[stages.adult]',
    )
    with pytest.raises(CompileError, match="targets adult_alt but species.toml has \\[assets\\] adult_alt = false"):
        compile_source(root, spec_mod.load(root))


def test_species_id_mismatch_with_registry_errors(tmp_path):
    root = _assets_copy(tmp_path)
    _patch_species_toml(root, "lalafu", "id = 1 ", "id = 9 ")
    with pytest.raises(CompileError, match="does not match registry.toml id"):
        compile_source(root, spec_mod.load(root))


def test_missing_shared_stage_art_errors(tmp_path):
    root = _assets_copy(tmp_path)
    (root / "species" / "ninjifu" / "child.txt").unlink()
    with pytest.raises(CompileError, match="missing child art"):
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
