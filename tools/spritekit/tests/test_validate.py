from pathlib import Path

from spritekit.grid import bbox as compute_bbox
from spritekit.spec import Spec
from spritekit.validate import (
    bbox_range_errors,
    centred_errors,
    coherence_errors,
    connectivity_errors,
    density_errors,
    dither_errors,
    eyes_errors,
    ground_errors,
    icon_errors,
    infer_cell_class,
    sleep_errors,
    stray_and_hole_errors,
    symmetry_errors,
    symmetry_score,
    validate_file,
)

MINIMAL_SPEC = Spec(root=Path("/nonexistent"), data={"rules": {"icon": {"blank_col": 7, "blank_row": 7, "density": [0.30, 0.70]}}})

PET_SPEC = Spec(
    root=Path("/nonexistent"),
    data={
        "rules": {
            "pet": {
                "ground_row": 15,
                "center_tolerance": 1.0,
                "density": [0.45, 0.85],
                "max_components": 3,
                "min_component_px": 3,
                "max_checkerboards": 1,
                "eyes_required": ["idle_a", "idle_b"],
                "eyes_top_fraction": 0.45,
                "symmetry": {"idle_a": 0.85, "idle_b": 0.85, "happy": 0.70, "sad": 0.70},
                "coherence": {"idle_b": [4, 24], "eat": [1, 20], "max_eye_drift_px": 1},
                "bbox": {
                    "baby": {"w": [6, 10], "h": [6, 10]},
                    "child": {"w": [9, 14], "h": [10, 15]},
                    "adult": {"w": [11, 16], "h": [12, 16]},
                },
            }
        }
    },
)

# A 16x16 pose that passes ground/centred/density/connectivity/dither/symmetry/eyes for the
# adult bbox band: a filled diamond-ish blob, mirrored eye holes in the top 45%.
GOOD_IDLE_A = [
    "................",
    "......####......",
    ".....######.....",
    "....########....",
    "...##########...",
    "...##-####-##...",
    "..############..",
    "..############..",
    "..############..",
    "..############..",
    "..############..",
    "...##########...",
    "...##########...",
    "....########....",
    "....########....",
    "....########....",
]


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


# --- pet-pose validator rules ------------------------------------------------------------------


def test_good_idle_a_passes_the_structural_checks():
    bb = compute_bbox(GOOD_IDLE_A, 16, 16)
    assert ground_errors(GOOD_IDLE_A, 16, bb, 15) == []
    assert centred_errors(bb, 16, 1.0) == []
    assert dither_errors(GOOD_IDLE_A, 16, 16, 1) == []
    assert connectivity_errors(GOOD_IDLE_A, 16, 16, 3, 3) == []
    errs, _ = density_errors(GOOD_IDLE_A, bb, 0.45, 0.85)
    assert errs == []
    errs, _ = symmetry_errors(GOOD_IDLE_A, 16, 16, bb, 0.85, "idle_a")
    assert errs == []
    assert eyes_errors(GOOD_IDLE_A, 16, 16, bb, 0.45) == []
    assert bbox_range_errors(bb, [11, 16], [12, 16]) == []  # adult band


def test_ground_requires_bbox_bottom_at_ground_row_and_two_pixels():
    rows = ["................"] * 16
    rows[10] = "......####......"[:16] if False else rows[10]
    # A pose that doesn't reach row 15 at all.
    rows = list(GOOD_IDLE_A)
    rows[15] = "................"
    bb = compute_bbox(rows, 16, 16)
    errs = ground_errors(rows, 16, bb, 15)
    assert any("bbox bottom" in e for e in errs)


def test_ground_requires_at_least_two_on_pixels_on_ground_row():
    rows = list(GOOD_IDLE_A)
    row15 = list(rows[15])
    # Leave a single on-pixel on the ground row.
    for i, c in enumerate(row15):
        if c == "#" and i != 7:
            row15[i] = "."
    rows[15] = "".join(row15)
    bb = compute_bbox(rows, 16, 16)
    errs = ground_errors(rows, 16, bb, 15)
    assert any("on-pixel" in e for e in errs)


def test_centred_flags_an_off_centre_bbox():
    rows = ["." * 16 for _ in range(16)]
    rows[15] = "##.............."  # bbox entirely on the left edge
    bb = compute_bbox(rows, 16, 16)
    errs = centred_errors(bb, 16, 1.0)
    assert any("centre" in e for e in errs)


def test_bbox_range_errors_flags_too_small_and_too_large():
    tiny = bbox_range_errors((7, 14, 8, 15), [6, 10], [6, 10])  # 2x2 bbox, below the [6,10] band
    assert any("width" in e or "height" in e for e in tiny)
    huge = bbox_range_errors((0, 0, 15, 15), [6, 10], [6, 10])  # 16x16 bbox, above the band
    assert any("width" in e for e in huge)
    assert bbox_range_errors((3, 5, 9, 11), [6, 10], [6, 10]) == []  # 7x7, within band


def test_density_flags_a_sparse_pose():
    sparse_rows = ["." * 16 for _ in range(16)]
    sparse_rows = list(sparse_rows)
    sparse_rows[0] = "#" + "." * 15
    sparse_rows[15] = "#" + "." * 15
    bb = compute_bbox(sparse_rows, 16, 16)
    errs, metrics = density_errors(sparse_rows, bb, 0.45, 0.85)
    assert errs and metrics["density"] < 0.45


def test_dither_detects_checkerboard():
    rows = ["................" for _ in range(16)]
    rows = list(rows)
    rows[4] = "....#..........."  # single isolated 2x2 checkerboard at (4,4)-(5,5)
    rows[5] = ".....#.........."
    errs = dither_errors(rows, 16, 16, 0)
    assert any("checkerboard" in e for e in errs)


def test_dither_allows_up_to_the_configured_maximum():
    rows = ["................" for _ in range(16)]
    rows = list(rows)
    rows[4] = "....#..........."
    rows[5] = ".....#.........."
    assert dither_errors(rows, 16, 16, 1) == []


def test_connectivity_flags_too_many_components():
    rows = ["................" for _ in range(16)]
    rows = list(rows)
    rows[0] = "###............."
    rows[8] = "..............##"  # a second, disconnected component
    errs = connectivity_errors(rows, 16, 16, max_components=1, min_component_px=1)
    assert any("disconnected components" in e for e in errs)


def test_connectivity_flags_a_far_component():
    rows = ["................" for _ in range(16)]
    rows = list(rows)
    rows[0] = "####............"
    rows[15] = "............####"  # opposite corner, well outside the 1px allowance
    errs = connectivity_errors(rows, 16, 16, max_components=3, min_component_px=1)
    assert any("from the main body" in e for e in errs)


def test_connectivity_allows_a_component_one_pixel_away():
    rows = ["................" for _ in range(16)]
    rows = list(rows)
    rows[0] = "###.###........."  # exactly one blank column of separation
    errs = connectivity_errors(rows, 16, 16, max_components=3, min_component_px=1)
    assert not any("from the main body" in e for e in errs)


def test_symmetry_score_is_1_for_a_mirror_symmetric_pose():
    bb = compute_bbox(GOOD_IDLE_A, 16, 16)
    assert symmetry_score(GOOD_IDLE_A, 16, 16, bb) > 0.95


def test_symmetry_errors_flags_lopsided_pose():
    rows = list(GOOD_IDLE_A)
    # Knock out one whole side to break symmetry.
    rows = ["".join(c if x < 8 else "." for x, c in enumerate(row)) for row in rows]
    bb = compute_bbox(rows, 16, 16)
    errs, metrics = symmetry_errors(rows, 16, 16, bb, 0.85, "idle_a")
    assert errs and metrics["symmetry"] < 0.85


def test_eyes_errors_requires_mask_pixels():
    rows = ["." * 16 for _ in range(16)]
    rows = list(rows)
    rows[15] = "##.............."
    bb = compute_bbox(rows, 16, 16)
    errs = eyes_errors(rows, 16, 16, bb, 0.45)
    assert any("no blink-mask" in e for e in errs)


def test_eyes_errors_requires_mirrored_pairs_in_top_band():
    rows = list(GOOD_IDLE_A)
    # Replace one eye hole with a plain '#', leaving the other mask pixel unmatched.
    rows[5] = "...##.####-##..."
    errs = eyes_errors(rows, 16, 16, compute_bbox(rows, 16, 16), 0.45)
    assert any("mirrored partner" in e for e in errs)


def test_eyes_errors_flags_eye_below_top_band():
    rows = list(GOOD_IDLE_A)
    rows[5] = "..############.."  # remove the eye holes from the top band
    rows[12] = "...##-####-##..."  # and place mirrored mask pixels well below it instead
    errs = eyes_errors(rows, 16, 16, compute_bbox(rows, 16, 16), 0.45)
    assert any("below the top" in e for e in errs)


def test_coherence_flags_pose_too_different_from_idle_a():
    idle_a = GOOD_IDLE_A
    far = ["." * 16 for _ in range(16)]
    far = list(far)
    far[15] = "##.............."
    errs, metrics = coherence_errors(far, idle_a, 16, 16, 4, 24, "idle_b", 1)
    assert errs and "differs from idle_a" in errs[0]


def test_coherence_allows_a_small_edit():
    idle_a = GOOD_IDLE_A
    idle_b = list(GOOD_IDLE_A)
    idle_b[6] = "..#" + "#" * 10 + "#.."  # a couple of pixels different
    errs, metrics = coherence_errors(idle_b, idle_a, 16, 16, 0, 24, "idle_b", 1)
    assert errs == []


def test_sleep_flags_a_taller_bbox_than_idle_a():
    idle_a = GOOD_IDLE_A  # bbox y0=1, y1=15 -> height 15
    taller_sleep = ["................" for _ in range(16)]
    taller_sleep = list(taller_sleep)
    taller_sleep[0] = "....########...."  # y0=0
    taller_sleep[15] = "....########...."  # same bottom (15), but height is now 16 > 15
    errs = sleep_errors(taller_sleep, idle_a, 16, 16)
    assert any("height" in e for e in errs)


def test_sleep_flags_a_mismatched_bottom():
    idle_a = GOOD_IDLE_A
    mismatched = list(idle_a)
    mismatched[15] = "................"
    mismatched[14] = "....########...."  # bbox bottom moves to row 14, not 15
    errs = sleep_errors(mismatched, idle_a, 16, 16)
    assert any("bbox bottom" in e for e in errs)


def test_sleep_allows_a_shorter_bbox_with_the_same_bottom():
    idle_a = GOOD_IDLE_A
    shorter_sleep = ["................" for _ in range(16)]
    shorter_sleep = list(shorter_sleep)
    shorter_sleep[10] = "....########...."
    shorter_sleep[15] = "....########...."
    errs = sleep_errors(shorter_sleep, idle_a, 16, 16)
    assert errs == []
