// Held-state button mask. The core does edge detection and long-press itself
// (docs/HOST_ABI.md "Buttons"); this module only ever reports what is currently down.

import { Buttons } from "./core";

const KEY_TO_BUTTON: Record<string, number> = {
  z: Buttons.A,
  "1": Buttons.A,
  x: Buttons.B,
  "2": Buttons.B,
  c: Buttons.C,
  "3": Buttons.C,
};

export class Input {
  private heldMask = 0;

  constructor(buttonEls: { a: HTMLElement; b: HTMLElement; c: HTMLElement }) {
    window.addEventListener("keydown", (e) => this.setFromKey(e, true));
    window.addEventListener("keyup", (e) => this.setFromKey(e, false));
    window.addEventListener("blur", () => {
      this.heldMask = 0;
    });

    this.wireButton(buttonEls.a, Buttons.A);
    this.wireButton(buttonEls.b, Buttons.B);
    this.wireButton(buttonEls.c, Buttons.C);
  }

  private setFromKey(e: KeyboardEvent, down: boolean): void {
    const bit = KEY_TO_BUTTON[e.key.toLowerCase()];
    if (bit === undefined) return;
    e.preventDefault();
    if (down) this.heldMask |= bit;
    else this.heldMask &= ~bit;
  }

  private wireButton(el: HTMLElement, bit: number): void {
    const down = (e: Event) => {
      e.preventDefault();
      this.heldMask |= bit;
    };
    const up = () => {
      this.heldMask &= ~bit;
    };
    el.addEventListener("pointerdown", down);
    el.addEventListener("pointerup", up);
    el.addEventListener("pointercancel", up);
    el.addEventListener("pointerleave", up);
  }

  mask(): number {
    return this.heldMask;
  }
}
