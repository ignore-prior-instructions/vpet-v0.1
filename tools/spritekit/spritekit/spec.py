"""Loads `assets/asset-spec.toml`. See docs/CONTENT.md."""

from __future__ import annotations

import tomllib
from dataclasses import dataclass
from pathlib import Path


@dataclass
class Spec:
    root: Path  # the `assets/` directory this spec was loaded from
    data: dict

    @property
    def cells(self) -> dict:
        return self.data.get("cells", {})

    @property
    def rules_pet(self) -> dict:
        return self.data.get("rules", {}).get("pet", {})

    @property
    def rules_icon(self) -> dict:
        return self.data.get("rules", {}).get("icon", {})

    @property
    def global_names(self) -> dict:
        return self.data.get("global", {})


def load(assets_root: Path) -> Spec:
    path = assets_root / "asset-spec.toml"
    with open(path, "rb") as f:
        data = tomllib.load(f)
    return Spec(root=assets_root, data=data)


def find_assets_root(start: Path | None = None) -> Path:
    """Walk upward from `start` (default: cwd) looking for an `assets/asset-spec.toml`."""
    p = (start or Path.cwd()).resolve()
    for candidate in [p, *p.parents]:
        if (candidate / "assets" / "asset-spec.toml").is_file():
            return candidate / "assets"
    raise FileNotFoundError("could not find assets/asset-spec.toml above " + str(p))
