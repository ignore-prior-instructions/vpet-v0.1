// Recorder for the `.vlog` format (docs/TESTING.md, crates/vpet-cli/src/vlog.rs). Emits only
// `reset`/`t`/`save`/`load`/`snap` lines — never `skip`, since every `t` line already carries
// its own absolute `now_ms` (the parser treats `skip` as documentary only). A downloaded
// recording replays byte-identically through `vpet-cli replay` because the same timestamps and
// button masks are logged here that were actually passed to `core.update`.

import { Buttons } from "./core";

function maskToToken(mask: number): string {
  if (mask === 0) return "-";
  let s = "";
  if (mask & Buttons.A) s += "A";
  if (mask & Buttons.B) s += "B";
  if (mask & Buttons.C) s += "C";
  return s;
}

/** `.vlog` snap names are a single whitespace-free token (the parser does `parts.next()`). */
function sanitizeName(name: string): string {
  const cleaned = name.trim().replace(/\s+/g, "_");
  return cleaned.length > 0 ? cleaned : "snap";
}

export class Recorder {
  private lines: string[] = [];

  reset(nowMs: number, seed: bigint): void {
    this.lines.push(`reset ${Math.trunc(nowMs)} seed=${seed}`);
  }

  t(nowMs: number, mask: number): void {
    this.lines.push(`t ${Math.trunc(nowMs)} ${maskToToken(mask)}`);
  }

  save(): void {
    this.lines.push("save");
  }

  load(): void {
    this.lines.push("load");
  }

  snap(name: string): void {
    this.lines.push(`snap ${sanitizeName(name)}`);
  }

  isEmpty(): boolean {
    return this.lines.length === 0;
  }

  toText(): string {
    return this.lines.join("\n") + "\n";
  }

  download(filename = "browser.vlog"): void {
    const blob = new Blob([this.toText()], { type: "text/plain" });
    const url = URL.createObjectURL(blob);
    const a = document.createElement("a");
    a.href = url;
    a.download = filename;
    document.body.appendChild(a);
    a.click();
    a.remove();
    URL.revokeObjectURL(url);
  }
}
