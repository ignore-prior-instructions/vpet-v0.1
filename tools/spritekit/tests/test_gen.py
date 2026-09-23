import json
from pathlib import Path

import pytest

from spritekit import spec as spec_mod
from spritekit.gen import GenError, _extract_pose_block, _parse_critique, gen_pose
from spritekit.model_client import AnthropicClient, MissingApiKeyError, require_api_key

# A pose that passes the real validator's default (`@style silhouette`) rules at the real 32x32
# adult cell size: the long-eared placeholder lalafu shipped with 64x32 (docs/adr/0016), before
# ADR 0017 replaced every species' art with hand-drawn `@style outline` sketches, at which point
# the asset tree stopped having any silhouette-strict pose to read this fixture from. Kept here
# as a literal so this test doesn't depend on the asset tree's current species art.
GOOD_POSE = """\
@pose idle_a
.......####..........####.......
.......####..........####.......
.......####..........####.......
.......####..........####.......
.......####..........####.......
.......####..........####.......
.......####..........####.......
.......####..........####.......
.......####..........####.......
.......####..........####.......
......####################......
.....######################.....
....######--########--######....
...#######--########--#######...
...##########################...
...##########################...
...##########################...
...##########################...
...##########################...
...##########################...
...##########################...
...##########################...
...##########################...
...##########################...
...##########################...
...##########################...
...##########################...
....########################....
.....######################.....
......####################......
.......#####........#####.......
.......#####........#####.......
"""


class FakeClient:
    """A scripted ModelClient: returns each queued `generate` response in order, and a fixed
    critique verdict."""

    def __init__(self, generate_responses: list[str], critique_verdict: dict):
        self._responses = list(generate_responses)
        self._verdict = critique_verdict
        self.generate_calls = 0
        self.critique_calls = 0

    def generate(self, prompt, *, image_bytes=None, temperature=0.8):
        self.generate_calls += 1
        return self._responses.pop(0)

    def critique(self, prompt, *, image_bytes):
        self.critique_calls += 1
        return json.dumps(self._verdict)


def _assets_root() -> Path:
    return Path(__file__).resolve().parents[3] / "assets"


def test_extract_pose_block_from_fenced_response():
    response = f"Some reasoning about symmetry.\n\n```\n{GOOD_POSE}```\n"
    block = _extract_pose_block(response)
    assert block.startswith("@pose idle_a")


def test_extract_pose_block_requires_fenced_code():
    with pytest.raises(GenError):
        _extract_pose_block("no code block here")


def test_parse_critique_extracts_json_even_with_surrounding_text():
    raw = 'Here is my verdict:\n```json\n{"readability": 5, "accept": true}\n```\n'
    verdict = _parse_critique(raw)
    assert verdict["accept"] is True
    assert verdict["readability"] == 5


def test_gen_pose_accepts_on_first_round_when_validator_and_critic_are_happy(tmp_path):
    root = _assets_root()
    sp = spec_mod.load(root)
    response = f"A round creature, symmetric about column 7.\n\n```\n{GOOD_POSE}```\n"
    verdict = {"readability": 5, "character": 5, "consistency": 5, "silhouette": 5, "issues": [], "accept": True}
    client = FakeClient([response], verdict)

    result = gen_pose(client, root, sp, "ninjifu", "adult", "idle_a", "A round creature.", "style guide text")

    assert result.accepted is True
    assert result.rounds == 1
    assert client.generate_calls == 1
    assert client.critique_calls == 1


def test_gen_pose_retries_on_validator_error_then_succeeds():
    root = _assets_root()
    sp = spec_mod.load(root)
    bad_pose = "@pose idle_a\n" + "................\n" * 16  # blank: fails ground/density
    good_response = f"Fixed it.\n\n```\n{GOOD_POSE}```\n"
    bad_response = f"First try.\n\n```\n{bad_pose}```\n"
    verdict = {"readability": 5, "character": 5, "consistency": 5, "silhouette": 5, "issues": [], "accept": True}
    client = FakeClient([bad_response, good_response], verdict)

    result = gen_pose(client, root, sp, "ninjifu", "adult", "idle_a", "A round creature.", "style guide text")

    assert result.accepted is True
    assert result.rounds == 2
    assert client.generate_calls == 2


def test_gen_pose_gives_up_after_max_rounds_when_critic_never_accepts():
    root = _assets_root()
    sp = spec_mod.load(root)
    response = f"Attempt.\n\n```\n{GOOD_POSE}```\n"
    verdict = {"readability": 2, "character": 2, "consistency": 2, "silhouette": 2, "issues": ["too blobby"], "accept": False}
    client = FakeClient([response, response, response, response], verdict)

    result = gen_pose(client, root, sp, "ninjifu", "adult", "idle_a", "A round creature.", "style guide text")

    assert result.accepted is False
    assert result.rounds == 4


def test_missing_api_key_raises_a_clear_error(monkeypatch):
    monkeypatch.delenv("ANTHROPIC_API_KEY", raising=False)
    with pytest.raises(MissingApiKeyError, match="ANTHROPIC_API_KEY"):
        require_api_key()


def test_anthropic_client_construction_fails_fast_without_a_key(monkeypatch):
    monkeypatch.delenv("ANTHROPIC_API_KEY", raising=False)
    with pytest.raises(MissingApiKeyError):
        AnthropicClient()
