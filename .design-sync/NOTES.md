# design-sync notes for vpet

The design system is `packages/vpet-ds` (`@vpet/ds`), a small React library built for this
sync: the cutesy shell around the 64x32 screen, with the game's real sprites baked in. Nothing
here redraws art; `scripts/sprites.mjs` reads `tests/fixtures/packed.json` (spritekit
`dev-fixture`) into `src/sprites.gen.ts` (gitignored, regenerated on every build).

- Build: `npm --prefix packages/vpet-ds run build` (tsup, ESM + d.ts). Run
  `uv run spritekit dev-fixture` first if `assets/` changed, so the sprites are current.
- Converter args: `--node-modules packages/vpet-ds/node_modules --entry ./packages/vpet-ds/dist/index.js`.
- Fonts: Fredoka and Nunito load from Google Fonts via `@import` in `styles.css`
  (`runtimeFontPrefixes` suppresses `[FONT_MISSING]`). Offline renders fall back to system-ui.
- Palette comes from the colour concept sketch (`docs/art/LINEAGE.md`), not the OLED/LCD
  themes the web host currently uses: ink #271e38, pink #fc7eaf, orange #ff965d, purple
  #503c70, lilac #c1afdc, bow red #f75564. The screen is ink on a warm pale panel.
- `[data-tone]` must win over component defaults: components use `var(--vp-tone, fallback)`
  rather than setting `--vp-tone` on their own class (same specificity, later rule wins).
- The `hatched` scene in `tests/golden/*.expected` is the hatch flash since PR #7; a baby idle
  frame is `feed_meal.expected` `after_eating`.
- Screen/Shell previews inline golden frames as 32-row string arrays (previews can only import
  from the package). No golden reaches lalafu's own adult; the Shell `Lalafu` card's ghost frame
  is the `lalafu/adult/idle_a` sprite blitted at `PET_HOME_X` (16). Shell captions are species
  names, so the `Baby` card has none: a baby is not a species yet.

## Known render warns

- `[RENDER_THIN]` PetSprite, PixelText: SVG-only output, no DOM text. Benign; the sheets show
  the sprites and pixel text.

## Re-sync risks

- Golden frames are copied into `.design-sync/previews/{Screen,Shell}.tsx` by hand; if the
  compositor changes, they are stale but still valid frames (not wrong, just old).
- `sprites.gen.ts` depends on `tests/fixtures/packed.json` being regenerated; a stale fixture
  ships stale art.
- Remote font: previews graded online. A `[FONT_MISSING]`-style fallback would show as
  system-ui text in the sheets.
- Uploaded 2026-09-24 to the Claude Design project "vpet" (`projectId` in config.json,
  https://claude.ai/design/p/060cf6a2-d0f8-4206-a686-16c3c713f2a1). `DesignSync` needs
  `/design-login` once per machine from an interactive session.
