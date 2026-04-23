# Modern Stylized UI Redesign — Contemporary Editorial

## Overview

The current Roary Mic UI is a functional settings panel with zero visual personality: system fonts, soft-pink accent, hairline borders, no typographic hierarchy beyond font weight. It looks like every other Tauri app.

This plan replaces the visual system with a **contemporary editorial** direction — dark-first, warm neutral palette, distinctive serif display type paired with a modern sans and a mono for technical elements. Keeps the app functional and dense (it's still a productivity tool, not a landing page) but gives it a confident point of view that matches the 🦁 "Roary" brand.

**What changes:** design tokens (CSS vars), typography pipeline, sidebar, section headers, UI primitives (Button/Toggle/SettingContainer/Group), and the recording overlay. Behavior, IPC, and all backend code are untouched.

**What stays:** sidebar-plus-content layout, Lucide icons, Tailwind usage, dark/light as OS-driven, all Rust/Tauri plumbing.

## Context (from discovery)

Files/components involved:

- `tailwind.config.js` — Tailwind 4 config; named colors reference CSS vars.
- `src/App.css:3-63` — `:root` and `prefers-color-scheme: dark` CSS vars: `--color-text`, `--color-background`, `--color-logo-primary`, `--color-background-ui`, `--color-mid-gray`, `--color-logo-stroke`.
- `index.html` — currently no `<link>` for fonts; will add Google Fonts preconnect + link.
- `src/components/Sidebar.tsx:47-127` — `SECTIONS_CONFIG`, 160px sidebar with Lucide icons + label pairs.
- `src/components/ui/Button.tsx`, `ToggleSwitch.tsx`, `SettingContainer.tsx`, `SettingsGroup.tsx`, `Input.tsx`, `Textarea.tsx`, `Select.tsx`, `Slider.tsx`, `Dropdown.tsx`, `Badge.tsx`, `Alert.tsx`, `Tooltip.tsx`, `ResetButton.tsx`, `PathDisplay.tsx`.
- `src/components/settings/*/...Settings.tsx` — individual section pages (General, Models, Corrections, References, AiMode, PostProcess, Advanced, Debug, About).
- `src/overlay/RecordingOverlay.tsx` — 172×36 HUD: `#000000cc` bg, `#ffe5ee` bars, white text, 18px radius.
- `src/components/icons/HandyTextLogo.tsx` — rendered wordmark; retains brand typography.
- `src/App.tsx` — root wrapper; contains `<Toaster />` and the top-level layout.

Design-token audit today (from `src/App.css`):

```css
:root {
  --color-text: #0f0f0f;
  --color-background: #fbfbfb;
  --color-logo-primary: #faa2ca;
  --color-background-ui: #da5893;
  --color-mid-gray: #808080;
  --color-logo-stroke: #f17ab3;
}
@media (prefers-color-scheme: dark) {
  --color-text: #fbfbfb;
  --color-background: #2c2b29;
  --color-logo-primary: #f28cbb;
  --color-logo-stroke: #fad1ed;
}
```

## Development Approach

- **Testing approach**: Regular (pragmatic) + manual visual verification. Playwright smoke tests should still pass; `bun run build` and `bun run lint` must stay clean after every task. No new Rust code, so `cargo test` is orthogonal — run once at the end to confirm nothing regressed.
- Complete each task fully before moving to the next.
- Every task ends with a lint + build run before proceeding.
- Dark mode and light mode must both be deliberately tuned — never ship a design that only looks right in one.
- Don't break i18n — all user-facing copy changes must go through `t()` and backfill all 20 locales.
- Preserve keyboard accessibility: visible focus rings, tab order, ARIA unchanged.

## Testing Strategy

- **Unit tests**: not adding new ones — this is a visual refactor, not logic. Existing tests must continue to pass.
- **Playwright**: the two smoke tests in `tests/app.spec.ts` must still pass (they verify dev server + HTML structure, not visual).
- **Manual visual verification** (per Post-Completion): walk every section in both dark and light mode, confirm the recording overlay still displays correctly, launch and onboarding flows render.

## Progress Tracking

- Mark completed items `[x]` immediately.
- Add discovered tasks with `➕`.
- Flag blockers with `⚠️`.

## What Goes Where

- **Implementation Steps**: design tokens, font pipeline, component restyle, page-level cleanup.
- **Post-Completion**: eyeballing the app, rebuilding the DMG, cutting a new release.

## Implementation Steps

### Task 1: Design tokens + font pipeline

- [ ] update `src/App.css`: replace the CSS var palette with the contemporary editorial values (see Technical Details). Add light and dark variants. Keep the `--color-logo-primary` name for compatibility but point it at the new accent; deprecate `--color-background-ui` by aliasing it to the accent for now.
- [ ] add `--font-display`, `--font-sans`, `--font-mono`, `--radius-sm`, `--radius-md`, `--radius-lg`, `--grain-opacity`, `--hairline` tokens.
- [ ] add `<link rel="preconnect" href="https://fonts.googleapis.com">` and `<link rel="preconnect" href="https://fonts.gstatic.com" crossorigin>` plus the three `<link>` tags for Instrument Serif, Geist, and JetBrains Mono to `index.html` (with `display=swap`).
- [ ] set `body { font-family: var(--font-sans); }` globally in `App.css`, plus base antialiasing (`-webkit-font-smoothing: antialiased`, `text-rendering: optimizeLegibility`).
- [ ] add a `.display` utility class mapped to the serif; a `.mono` utility mapped to the mono.
- [ ] add a subtle SVG noise texture overlay as a fixed-position pseudo-element on `body::before` at ~3% opacity (dark mode only — grain on light mode reads as dirt).
- [ ] `bun run lint` + `bun run build` — both must be clean.

### Task 2: Tailwind config extensions

- [ ] in `tailwind.config.js`, extend `fontFamily` with `display`, `sans`, `mono` mapped to the new CSS vars.
- [ ] extend `colors` with semantic names: `ink` (main text), `paper` (background), `accent` (terracotta), `rule` (hairline divider), `muted` (secondary text). Keep `logo-primary`, `mid-gray`, etc. as aliases so nothing breaks mid-migration.
- [ ] extend `borderRadius` with `xs: 3px`, `sm: 5px`, `md: 8px`, `lg: 12px`, `xl: 18px` (current 8/12 feels bouncy — we want a tighter hierarchy).
- [ ] extend `spacing` with a `hairline: 0.5px` utility that uses scale transforms to render sub-pixel under Retina.
- [ ] `bun run lint` + `bun run build`.

### Task 3: Sidebar redesign

- [ ] widen sidebar from 160px to 200px in `src/components/Sidebar.tsx`.
- [ ] replace Lucide-icon-plus-label rows with small-caps mono labels (text-xs, letter-spacing-wider, opacity 60% inactive, 100% active).
- [ ] active section gets an accent-colored left bar (2px wide, full height of the row) — not a background fill. Hover state: 100% opacity, no bar.
- [ ] section numbers `01 02 03...` in JetBrains Mono at 50% opacity to the left of each label (editorial section markers).
- [ ] brand area at the top: "Roary" in Instrument Serif at 22px italic, "Mic" in Geist Sans at 22px regular, combined like a magazine nameplate. Drop the existing logo SVG here.
- [ ] footer of sidebar gets a small embossed lion sigil (the 🦁 emoji rendered at 14px opacity 30%) + version number `0.8.2-roary.N` in JetBrains Mono.
- [ ] `bun run lint` + `bun run build`.

### Task 4: Section header pattern

- [ ] create `src/components/settings/SectionHeader.tsx` that renders: section-number in mono (e.g., "03"), title in Instrument Serif at a display size (32-40px), a thin accent rule beneath, and a description paragraph in Geist at 14px / 1.6 line-height / 70% opacity.
- [ ] replace the existing inline `<h2 className="text-xs font-medium text-mid-gray uppercase...">` pattern in every `*Settings.tsx` with the new `SectionHeader`.
- [ ] header has a generous top margin (48px) and bottom margin (32px) — editorial whitespace rhythm.
- [ ] `bun run lint` + `bun run build`.

### Task 5: Primitive refresh — Button + ToggleSwitch

- [ ] `src/components/ui/Button.tsx`: redefine variants. `primary` = accent bg, ink text, `rounded-sm`; `ghost` = transparent with accent underline on hover; `quiet` = just text with color change on hover. Drop `primary-soft`, `danger-ghost` as redundant. Buttons use `--font-sans` at `font-medium`, tracking `-0.01em`.
- [ ] `src/components/ui/ToggleSwitch.tsx`: tighter proportions (36x20 → 32x18), off-state is a hairline-outlined pill with ink at 40% opacity, on-state is a solid accent pill with paper-colored knob. Smooth spring transition (200ms `cubic-bezier(0.34, 1.56, 0.64, 1)` — a subtle overshoot).
- [ ] preserve all existing prop shapes — this is a visual-only refresh.
- [ ] `bun run lint` + `bun run build`.

### Task 6: Primitive refresh — SettingContainer, SettingsGroup, Input, Select, Textarea

- [ ] `SettingContainer`: remove the rounded card border entirely. Each setting becomes a row with a hairline bottom rule. Title in sans medium at 14px, description in sans regular at 13px at 70% opacity below the title (stacked). Control floats right.
- [ ] `SettingsGroup`: group title becomes small-caps mono (12px, tracking-widest, 50% opacity), section gets no outer border — just the internal rule stack from SettingContainer. One `<hr class="hairline">` separates groups.
- [ ] `Input` / `Textarea` / `Select`: kill the `rounded-md` + border. Replace with bottom-hairline-only underlined inputs — focus state shifts the hairline to the accent color (200ms). Monospace for path-like inputs, sans for prose.
- [ ] `Slider`: already minimal; just restyle the track and thumb with the accent color.
- [ ] `Dropdown`, `Badge`, `Alert`, `Tooltip`: restyle to use the new palette. Tooltip gets a small serif italic treatment for emphasis.
- [ ] `bun run lint` + `bun run build`.

### Task 7: Recording overlay + AI-reply window refresh

- [ ] `src/overlay/RecordingOverlay.tsx`: update bg to `rgba(14, 14, 16, 0.88)` with 24px backdrop-blur, accent-colored bars (was pink), white text in mono at 12px tracking-wider. Radius 12px.
- [ ] `src/ai-reply/AiReplyWindow.tsx`: apply the new palette + typography to the reply preview. Preserve all interactive behavior.
- [ ] `bun run lint` + `bun run build`.

### Task 8: Apply to every settings section

- [ ] go through `src/components/settings/{general,models,corrections,references,ai-mode,history,post-process,advanced,debug,about}/*.tsx` — adopt the new `SectionHeader`, remove redundant inline borders / rounded cards that now clash with the hairline-first system, clean up any spacing that assumed the old density.
- [ ] AboutSettings (or the equivalent) gets the hero treatment: display-size "Roary Mic" in Instrument Serif italic, mono version line, one lion sigil decoration, three credits lines in sans. Aspirational but not precious.
- [ ] `bun run lint` + `bun run build`.

### Task 9: Onboarding flow refresh

- [ ] `src/components/onboarding/*.tsx`: adopt new tokens. The welcome screen gets the same magazine-nameplate brand treatment as the sidebar. Progress bars become hairline-rule fills in the accent color.
- [ ] Accessibility onboarding step: the permission prompt card gets the new hairline + display heading treatment.
- [ ] `bun run lint` + `bun run build`.

### Task 10: Motion polish

- [ ] add a 200ms fade-in on section switch (the content area's root key changes on route — easy `animate-in` via Tailwind Motion plugin or CSS keyframes).
- [ ] sidebar section numbers get a subtle `tabular-nums` + slight opacity bump (50% → 80%) on hover.
- [ ] toast enter/exit softened (sonner has its own API — tweak via the `<Toaster />` options in `App.tsx`).
- [ ] no motion should exceed 300ms; no bouncy overshoot on any non-toggle component (it would read as AI slop).
- [ ] `bun run lint` + `bun run build`.

### Task 11: Verify

- [ ] `cargo test --lib` — all 129 tests still pass (nothing touched).
- [ ] `bun run lint`, `bun run build`, `bun run check:translations` (note: pre-existing missing keys are unrelated; verify no new missing keys).
- [ ] manual sweep of every settings section in **both** dark and light mode.
- [ ] launch the actual app, trigger a dictation, confirm the recording overlay and paste flow still look and work right.
- [ ] check that focus rings are visible and keyboard navigation works.

### Task 12: Update documentation

- [ ] update `AGENTS.md` — add a short "Visual System" section covering the three fonts, the semantic palette tokens, and the typography rhythm rule (serif for display, sans for body, mono for technical/code).
- [ ] update `docs/research/` if any new conventions emerged (e.g., the section-header component pattern, the hairline-first layout philosophy).

## Technical Details

**Proposed CSS variable palette.** Tuned to be warm, contemporary, and readable on a mid-brightness laptop screen.

Light mode:
```css
--color-paper: #FAF8F3;         /* cream */
--color-ink: #14110E;           /* deep cocoa */
--color-accent: #C85A3A;        /* terracotta */
--color-accent-hover: #B34E2E;
--color-muted: #6B6560;         /* secondary text */
--color-rule: rgba(20, 17, 14, 0.08);
--color-focus-ring: #C85A3A;
```

Dark mode:
```css
--color-paper: #0E0E10;         /* warm charcoal, not pure black */
--color-ink: #F5F3EE;           /* warm cream */
--color-accent: #E8906A;        /* warmer terracotta for dark */
--color-accent-hover: #F4A680;
--color-muted: rgba(245, 243, 238, 0.55);
--color-rule: rgba(245, 243, 238, 0.08);
--color-focus-ring: #E8906A;
```

**Typography stack:**

```css
--font-display: 'Instrument Serif', Georgia, serif;  /* titles only; use italic for brand */
--font-sans: 'Geist', system-ui, -apple-system, sans-serif;
--font-mono: 'JetBrains Mono', ui-monospace, 'SF Mono', monospace;
```

All three are free via Google Fonts. Instrument Serif is a display-optimized modern serif; Geist is Vercel's geometric sans; JetBrains Mono is widely-shipped and has good tabular numerals.

**Typography rhythm rule.** Serif is *only* for display sizes (28px+). Never use Instrument Serif for body or buttons — it's a display face and falls apart at small sizes. Sans for everything from 11px to 22px. Mono for: keyboard shortcuts, version numbers, section numerals (01, 02…), and path-like inputs.

**Hairline divider implementation.** Tailwind's `border` is 1px which becomes visually thick at Retina scale. We want a 0.5px divider for editorial elegance. Implement as:

```css
.hairline {
  height: 1px;
  background: var(--color-rule);
  transform: scaleY(0.5);
  transform-origin: top;
}
```

Or as a `::after` pseudo-element on the container.

**Grain texture (dark mode only).** A small SVG noise tile applied via `background-image` at very low opacity, scaled and tiled. Not animated — just adds imperceptible texture that makes the dark background feel like paper rather than a monitor. Skip on light mode.

**Motion philosophy.** Duration: 200ms for state changes, 280ms for section transitions. Easing: `cubic-bezier(0.32, 0.72, 0, 1)` (snappy out, gentle in). One exception — the toggle switch gets a mild spring (`cubic-bezier(0.34, 1.56, 0.64, 1)`) for tactile feel. Everything else is calm.

**Why not keep the pink.** The original pink (#faa2ca / #f28cbb) is a cotton-candy shade that reads as juvenile in a professional tool. Terracotta retains warmth (matches the lion) while signaling seriousness. Pink could be retained as a tertiary-emphasis color in one specific spot (e.g., recording indicator), but the primary accent should change.

**Why not keep generic system fonts.** System fonts (SF / Segoe / Roboto depending on OS) make every app look the same. Ship your own fonts, own the aesthetic.

## Post-Completion

**Manual verification** (on the dev machine after all tasks complete):

- Launch the app in dark mode, walk every sidebar section, confirm headers render in Instrument Serif, body renders in Geist, shortcuts in JetBrains Mono. No layout shift when fonts load (Google Fonts `display=swap` should be instant enough; if it flashes, add `font-display: optional`).
- Switch system to light mode, confirm palette inverts cleanly and the accent stays recognizable.
- Trigger a recording — overlay should have the new dark/accent palette and the 12-px mono text.
- Trigger an AI-mode reply — reply window should match the settings aesthetic.
- Test keyboard navigation: Tab through a settings page, confirm focus rings are visible (they should be the accent color at 2px).
- Confirm Roary's lion sigil appears in the sidebar footer + About but doesn't feel cluttered.

**Release cycle:**

- Build the DMG: `bun run tauri build --bundles dmg`.
- Bump the tag to `v0.8.2-roary.2` (the UI redesign is a meaningful visual change; the underlying feature set is unchanged, so we stay at `0.8.2` semver).
- Publish a new GitHub release with the updated DMG, changelog entry: "Modern stylized UI redesign — new typography, terracotta accent, hairline-first layout."

**Out of scope (do NOT attempt in this plan):**

- Accessibility mode (high-contrast, reduced-motion, large-text). A proper a11y pass is its own follow-up plan.
- Animations that depend on Framer Motion or a motion library — keep it CSS for now.
- A mobile / narrow-viewport pass. Roary Mic is a desktop app; the window has a minimum width.
- Light/dark manual toggle. Remains OS-driven per existing behavior.
- Changing any product copy or translations. Voice-and-tone is a separate concern.
