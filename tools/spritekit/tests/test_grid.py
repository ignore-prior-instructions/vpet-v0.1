from spritekit.grid import GridError, bbox, pack_bits, pack_image, pack_mask, parse

WORKED_EXAMPLE = """@cell 16x16

@pose idle_a
......####......
....########....
...##########...
...##-####-##...
..############..
..############..
...####..####...
...##########...
...##########...
....########....
....##....##....
....##....##....
...###....###...
...###....###...
..####....####..
..####....####..
"""


def test_parses_worked_example():
    gf = parse("worked.txt", WORKED_EXAMPLE)
    assert gf.cell_w == 16
    assert gf.cell_h == 16
    assert len(gf.sprites) == 1
    s = gf.sprites[0]
    assert s.name == "idle_a"
    assert s.kind == "pose"
    assert len(s.rows) == 16
    assert s.rows[3] == "...##-####-##..."


def test_row3_packs_per_spec_example():
    row3 = "...##-####-##..."
    img = pack_image([row3], 16)
    mask = pack_mask([row3], 16)
    assert img == bytes([0x1B, 0xD8])
    assert mask == bytes([0x04, 0x20])


def test_pack_bits_msb_first():
    # 8 bits, only bit 0 (leftmost) set -> 0b10000000
    assert pack_bits([1, 0, 0, 0, 0, 0, 0, 0], 8) == bytes([0x80])
    # only the last bit set -> 0b00000001
    assert pack_bits([0, 0, 0, 0, 0, 0, 0, 1], 8) == bytes([0x01])


def test_pack_mask_is_none_without_blink_chars():
    rows = ["........"] * 8
    assert pack_mask(rows, 8) is None


def test_multiple_sprites_and_mouth_anchor():
    text = """@cell 8x8
@species lalafu
@stage baby

@pose idle_a
@mouth 5,9
........
........
........
........
........
........
........
........

@pose idle_b
........
........
........
........
........
........
........
........
"""
    gf = parse("x.txt", text)
    assert gf.meta == {"species": "lalafu", "stage": "baby"}
    assert len(gf.sprites) == 2
    assert gf.sprites[0].mouth == (5, 9)
    assert gf.sprites[1].mouth is None


def test_wrong_row_length_raises():
    text = "@cell 4x2\n\n@sprite x\n...\n....\n"
    try:
        parse("bad.txt", text)
        assert False, "expected GridError"
    except GridError as e:
        assert "expected 4 characters" in str(e)


def test_invalid_character_raises():
    text = "@cell 4x1\n\n@sprite x\n.#x.\n"
    try:
        parse("bad.txt", text)
        assert False, "expected GridError"
    except GridError as e:
        assert "invalid character" in str(e)


def test_bbox():
    rows = ["....", "..#.", "....", "...."]
    assert bbox(rows, 4, 4) == (2, 1, 2, 1)
    assert bbox(["...."], 4, 1) is None
