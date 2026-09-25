# Building with @vpet/ds

vpet is a Tamagotchi-style virtual pet. Its 64x32 one-bit screen is drawn by a Rust core; this
library is the cutesy frame around it. Everything is `window.VpetDs.*`; styles come from
`styles.css` (tokens, fonts, and component CSS), no provider or wrapper is needed.

## Palette and type (CSS custom properties on `:root`, use `var(--vp-*)`)

- Ink for outlines and text: `--vp-ink` (#271e38), softer `--vp-ink-soft`.
- Species colours: `--vp-pink` (lalafu the ghost), `--vp-orange` (charamofu the dino),
  `--vp-purple` / `--vp-lilac` (ninjifu the ninja), bow `--vp-red`, plus `--vp-green`,
  `--vp-sky`. Pastel fills: `--vp-pink-soft`, `--vp-orange-soft`, `--vp-lilac-soft`; pale
  backgrounds `--vp-pink-pale`, `--vp-cream`, `--vp-paper` (page background).
- Screen colours: `--vp-screen-on`, `--vp-screen-off`.
- Type: `--vp-font-display` (Fredoka, headings and buttons), `--vp-font-body` (Nunito).
  Sizes `--vp-text-sm|md|lg|xl`.
- Shape: every surface has a `--vp-line` (3px) ink outline and a rounded radius
  (`--vp-radius-sm|md|lg|pill`); raised things get `--vp-shadow` (a 4px ink drop edge).
- Spacing `--vp-space-1..6` (4 to 32px).

Style your own layout glue with these tokens; do not invent colours or use browser defaults.
Put `class="vp-root"` on the page root for the paper background and body font.

## Tones

`data-tone="pink|orange|lilac"` on any wrapper recolours the `Shell`, `ShellButton`, and `Pill`
inside it. `Shell` takes it as the `tone` prop.

## Components

- `Shell` frames a `Screen` in the device body with A/B/C `ShellButton`s. `Screen` takes a
  frame as 512 hex chars or 32 rows of `.`/`#`. The screen's contents are the game's job; never
  draw UI inside it.
- `PetSprite species="lalafu|ninjifu|charamofu" stage pose` and `Sprite sprite="icon/feed"`
  render the real game art (names: `lalafu/adult/idle_a`, `icon/*`, `item/*`, `effect/*`,
  `egg/egg_a`, `heart/heart_full`, `tombstone`, `hatch/burst`). `PixelText` sets text in the
  5x7 game font. `HeartMeter` and `IconBar`/`MenuIcon` mirror the status page and menu.
- App UI: `Button` (primary/secondary/danger/ghost), `Sheet` (title, hint, fields, actions),
  `Field`, `StatusDot`, `Pill`.

## Example

```jsx
<div className="vp-root" style={{ padding: "var(--vp-space-6)", display: "grid", gap: "var(--vp-space-5)" }}>
  <Shell tone="pink" name="lalafu">
    <Screen frame={frameRows} scale={4} />
  </Shell>
  <Sheet title="Sync" hint="Share one pet between devices." actions={<Button>Save</Button>}>
    <Field label="Pet code" placeholder="lalafu-7f3k" />
    <StatusDot level="ok">Synced 2 minutes ago</StatusDot>
  </Sheet>
</div>
```
