"""The model client interface `gen.py`/`critique.py` talk to, and the real Anthropic-backed
implementation. Kept separate from `gen.py` so tests can substitute a fake without touching
network code or requiring `anthropic` to be installed (it's an optional dependency; see
`pyproject.toml`'s `[project.optional-dependencies].gen`).
"""

from __future__ import annotations

import os
from typing import Protocol


class ModelClient(Protocol):
    """One text call (pose generation) and one vision call (critique)."""

    def generate(self, prompt: str, *, image_bytes: bytes | None = None, temperature: float = 0.8) -> str: ...

    def critique(self, prompt: str, *, image_bytes: bytes) -> str: ...


class MissingApiKeyError(RuntimeError):
    pass


def require_api_key() -> str:
    key = os.environ.get("ANTHROPIC_API_KEY")
    if not key:
        raise MissingApiKeyError(
            "ANTHROPIC_API_KEY is not set. `spritekit gen`/`critique` call the Anthropic API "
            "for pose generation and vision critique (see docs/art/ART_PIPELINE.md); set the "
            "key or draw the pose by hand and skip straight to `spritekit validate`/`render`."
        )
    return key


class AnthropicClient:
    """Thin wrapper around the `anthropic` SDK. Constructing this without the SDK installed or
    without an API key raises immediately, before any network call."""

    def __init__(self, model: str = "claude-sonnet-5"):
        require_api_key()
        try:
            import anthropic
        except ImportError as e:
            raise MissingApiKeyError(
                "the `anthropic` package isn't installed; run `uv sync --extra gen`"
            ) from e
        self._client = anthropic.Anthropic()
        self._model = model

    def generate(self, prompt: str, *, image_bytes: bytes | None = None, temperature: float = 0.8) -> str:
        content: list[dict] = [{"type": "text", "text": prompt}]
        if image_bytes is not None:
            content.insert(0, _image_block(image_bytes))
        resp = self._client.messages.create(
            model=self._model,
            max_tokens=2048,
            temperature=temperature,
            messages=[{"role": "user", "content": content}],
        )
        return "".join(b.text for b in resp.content if getattr(b, "type", None) == "text")

    def critique(self, prompt: str, *, image_bytes: bytes) -> str:
        resp = self._client.messages.create(
            model=self._model,
            max_tokens=1024,
            temperature=0,
            messages=[{"role": "user", "content": [_image_block(image_bytes), {"type": "text", "text": prompt}]}],
        )
        return "".join(b.text for b in resp.content if getattr(b, "type", None) == "text")


def _image_block(png_bytes: bytes) -> dict:
    import base64

    return {
        "type": "image",
        "source": {"type": "base64", "media_type": "image/png", "data": base64.b64encode(png_bytes).decode("ascii")},
    }
