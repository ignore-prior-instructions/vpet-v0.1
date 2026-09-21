"""`spritekit gen`: the generate -> validate -> render -> critique -> revise loop from
docs/art/ART_PIPELINE.md. Needs `ANTHROPIC_API_KEY` (see `model_client.require_api_key`); not
exercised in this environment (no key), but written to spec and unit-tested against a fake
`ModelClient` so the loop logic itself is verified offline.
"""

from __future__ import annotations

import json
import re
import tempfile
from dataclasses import dataclass, field
from pathlib import Path

from .grid import GridError, parse as parse_grid
from .model_client import ModelClient
from .render import render_sprite_image
from .spec import Spec
from .validate import Report, validate_file

PROMPTS_DIR = Path(__file__).parent / "prompts"
MAX_VALIDATE_ROUNDS = 3
MAX_TOTAL_ROUNDS = 4

POSE_INSTRUCTIONS = {
    "idle_a": "Neutral, front-facing. This is the anchor pose: every other pose is judged "
    "against it. Mark eyes.",
    "idle_b": "Same silhouette as idle_a, 4 to 24 pixels changed (a squash, or ears/limbs "
    "moved). Keep the bottom row identical. Mark eyes.",
    "happy": "Arms, ears, or mouth express excitement. At most 40 pixels different from idle_a.",
    "eat": "Identical to idle_a except the mouth is open, 1 to 20 pixels different.",
    "sleep": "Eyes closed (no mask), body may be lower and rounder, but keep the same bottom "
    "row and the same or a smaller height than idle_a.",
    "sad": "Eyes or ears droop. At most 40 pixels different from idle_a.",
}


@dataclass
class GenResult:
    pose_name: str
    accepted: bool
    grid_text: str
    rounds: int
    log_entries: list[dict] = field(default_factory=list)


class GenError(RuntimeError):
    pass


def _extract_pose_block(response_text: str) -> str:
    m = re.search(r"```(?:\w*)\n(.*?)```", response_text, re.S)
    if not m:
        raise GenError("model response has no fenced code block")
    block = m.group(1).strip("\n")
    if not block.lstrip().startswith("@pose"):
        raise GenError("fenced block does not start with '@pose'")
    return block


def _validate_candidate(assets_root: Path, spec: Spec, slug: str, stage: str, pose_name: str, block_text: str) -> Report:
    stage_file = assets_root / "species" / slug / f"{stage}.txt"
    if stage_file.exists():
        existing = parse_grid(str(stage_file), stage_file.read_text(encoding="utf-8"))
        cell_w, cell_h = existing.cell_w, existing.cell_h
        siblings = [s for s in existing.sprites if s.name != pose_name]
    else:
        cell_w, cell_h = 16, 16
        siblings = []

    header = f"@cell {cell_w}x{cell_h}\n@species {slug}\n@stage {stage}\n\n"
    parts = [f"@pose {s.name}\n" + "\n".join(s.rows) for s in siblings]
    parts.append(block_text)
    synthetic = header + "\n\n".join(parts) + "\n"

    with tempfile.NamedTemporaryFile("w", suffix=".txt", delete=False, dir=stage_file.parent if stage_file.exists() else None) as f:
        f.write(synthetic)
        tmp_path = Path(f.name)
    try:
        try:
            reports = validate_file(tmp_path, spec)
        except GridError as e:
            return Report(file=str(tmp_path), sprite=pose_name, errors=[str(e)])
        for r in reports:
            if r.sprite == pose_name:
                return r
        return Report(file=str(tmp_path), sprite=pose_name, errors=["pose not found after parsing candidate"])
    finally:
        tmp_path.unlink(missing_ok=True)


def _build_prompt(species_brief: str, style_guide: str, stage_context: str, pose_name: str, stage: str, feedback: str | None) -> str:
    template = (PROMPTS_DIR / "pose.md").read_text(encoding="utf-8")
    prompt = template.format(
        species_brief=species_brief,
        style_guide=style_guide,
        stage_context=stage_context,
        pose_name=pose_name,
        stage=stage,
        pose_instruction=POSE_INSTRUCTIONS.get(pose_name, ""),
    )
    if feedback:
        prompt += f"\n\n## Feedback from the previous attempt\n\n{feedback}\n"
    return prompt


def _parse_critique(raw: str) -> dict:
    m = re.search(r"\{.*\}", raw, re.S)
    if not m:
        raise GenError(f"critique response has no JSON object: {raw!r}")
    return json.loads(m.group(0))


def gen_pose(
    client: ModelClient,
    assets_root: Path,
    spec: Spec,
    slug: str,
    stage: str,
    pose_name: str,
    species_brief: str,
    style_guide: str,
    stage_context: str = "(first pose of this stage)",
) -> GenResult:
    feedback: str | None = None
    log: list[dict] = []

    for round_no in range(MAX_TOTAL_ROUNDS):
        prompt = _build_prompt(species_brief, style_guide, stage_context, pose_name, stage, feedback)
        response = client.generate(prompt, temperature=0.8)
        try:
            block = _extract_pose_block(response)
        except GenError as e:
            log.append({"round": round_no, "error": str(e)})
            feedback = str(e)
            continue

        report = _validate_candidate(assets_root, spec, slug, stage, pose_name, block)
        log.append({"round": round_no, "errors": report.errors, "warnings": report.warnings, "metrics": report.metrics})

        if report.errors:
            if round_no < MAX_VALIDATE_ROUNDS:
                feedback = "Validator errors:\n" + "\n".join(report.errors)
                continue
            return GenResult(pose_name, accepted=False, grid_text=block, rounds=round_no + 1, log_entries=log)

        rows = block.split("\n")[1:]  # drop the @pose header line
        png = render_sprite_image(rows, spec.cells.get("pet", [16, 16])[0], spec.cells.get("pet", [16, 16])[1])
        import io

        buf = io.BytesIO()
        png.save(buf, format="PNG")

        critique_template = (PROMPTS_DIR / "critique.md").read_text(encoding="utf-8")
        critique_prompt = critique_template.format(
            species_brief=species_brief, grid_text=block, validator_metrics=json.dumps(report.metrics)
        )
        verdict_raw = client.critique(critique_prompt, image_bytes=buf.getvalue())
        verdict = _parse_critique(verdict_raw)
        log.append({"round": round_no, "verdict": verdict})

        scores = [verdict.get(k, 0) for k in ("readability", "character", "consistency", "silhouette")]
        if verdict.get("accept") and min(scores) >= 4:
            return GenResult(pose_name, accepted=True, grid_text=block, rounds=round_no + 1, log_entries=log)

        feedback = "Critic issues:\n" + "\n".join(verdict.get("issues", []))

    return GenResult(pose_name, accepted=False, grid_text=block, rounds=MAX_TOTAL_ROUNDS, log_entries=log)


def append_gen_log(species_dir: Path, result: GenResult) -> None:
    log_path = species_dir / "gen-log.jsonl"
    with open(log_path, "a", encoding="utf-8") as f:
        f.write(json.dumps({"pose": result.pose_name, "accepted": result.accepted, "rounds": result.rounds, "entries": result.log_entries}) + "\n")
