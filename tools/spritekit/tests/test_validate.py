from pathlib import Path

from spritekit.spec import Spec
from spritekit.validate import icon_errors, infer_cell_class, stray_and_hole_errors, validate_file

MINIMAL_SPEC = Spec(root=Path("/nonexistent"), data={"rules": {"icon": {"blank_col": 7, "blank_row": 7, "density": [0.30, 0.70]}}})


def test_no_stray_or_hole_on_blank_grid():
    rows = ["........"] * 8
    assert stray_and_hole_errors(rows, 8, 8) == []


def test_detects_isolated_on_pixel():
    rows = ["........"] * 8
    rows = list(rows)
    rows[4] = "...#...."  # single lit pixel with no lit neighbours
    errors = stray_and_hole_errors(rows, 8, 8)
    assert any("stray on-pixel" in e for e in errors)


def test_detects_1px_hole():
    rows = [
        "........",
        "........",
        "..###...",
        "..#.#...",  # the middle '.' is enclosed on all 4 sides by '#'
        "..###...",
        "........",
        "........",
        "........",
    ]
    errors = stray_and_hole_errors(rows, 8, 8)
    assert any("1px hole" in e for e in errors)


def test_hole_marked_with_dash_is_not_flagged_by_hole_check():
    # '-' is off-in-base but participates in the blink mask; the *hole* check only looks at '.',
    # so a '-' in the same enclosed position never triggers it.
    rows = [
        "........",
        "........",
        "..###...",
        "..#-#...",
        "..###...",
        "........",
        "........",
        "........",
    ]
    errors = stray_and_hole_errors(rows, 8, 8)
    assert not any("1px hole" in e for e in errors)


def test_icon_errors_blank_border():
    rows = ["#......."] * 8  # column 7 and row 7 are blank; density is low but let's check border first
    rows[7] = "#......."  # row 7 has a lit pixel -> should error
    errs, metrics = icon_errors(rows, 8, 8, MINIMAL_SPEC)
    assert any("row 7 must be blank" in e for e in errs)


def test_icon_errors_density_band():
    rows = ["......." + "."] * 8  # all blank -> density 0, outside [0.30, 0.70]
    errs, metrics = icon_errors(rows, 8, 8, MINIMAL_SPEC)
    assert any("density" in e for e in errs)
    assert metrics["density"] == 0.0


def test_infer_cell_class_by_filename():
    assert infer_cell_class(Path("assets/global/icons.txt")) == "icon"
    assert infer_cell_class(Path("assets/global/egg.txt")) == "pet"
    assert infer_cell_class(Path("assets/global/effects.txt")) == "effect"
    assert infer_cell_class(Path("assets/species/lalafu/baby.txt")) == "pet"
    assert infer_cell_class(Path("assets/global/unknown.txt")) is None
