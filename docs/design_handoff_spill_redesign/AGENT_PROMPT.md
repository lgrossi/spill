# Agent prompt — paste this to your Claude Code / coding agent

> Copy everything below the line and paste it into your coding agent's chat as the FIRST message of a new session. Attach `/docs/design/design_handoff_spill_redesign/` (this folder) to the project root or to the prompt.

---

We're redesigning the visual layer of our app (codename **Spill.** — a retrospective tool with columns: mood / went well / went wrong / actions). I have a partially working implementation built against earlier wireframes. I'm now upgrading to a high-fidelity applied design called **Daylight Cork**.

**Your job:** Migrate the existing app to the new design. Do NOT ship the HTML reference files directly — they are mockups. Rebuild in our existing codebase using its existing patterns (components, state, routing, styling).

## Read these first, in order

1. `design_handoff_spill_redesign/README.md` — full design spec.
2. `design_handoff_spill_redesign/TOKENS.css` — every color, font, shadow, radius as CSS custom properties.
3. `design_handoff_spill_redesign/Spill App.html` — open this in a browser to SEE the design. Pan/zoom the canvas to inspect all 7 screens.
4. `design_handoff_spill_redesign/spill-app.jsx` — the React source of the mock. Read this when you need an exact value for any component. Each screen is a function on `window` (e.g. `window.real_Writing` starts ~line 734).

## How to proceed

1. **Survey the existing code.** Identify our current theme file / token system, our base components (Button, Card, Avatar, Pill, etc.), and our 7 screens that match the mock screens. Report what you find before changing anything.
2. **Plan the migration.** Propose an order. Recommended order: tokens → primitive components → screen-by-screen. Wait for my approval.
3. **Token swap first.** Replace existing theme values with values from `TOKENS.css`. Get the app rendering with the new palette/type/shadows globally before refactoring layouts.
4. **Then primitives.** Refactor our Button / Card / Avatar / Pill / Chip / Input components to match the spec (radius 8px default, dual-shadow card with inset highlight, etc.).
5. **Then screens, in this order:**
   - Overview (lobby) — ① Overview
   - New Board modal — ② New Board
   - **Writing screen — ③ Writing** ← this is the core; spend the most time here
   - Cluster — ④
   - Voting — ⑤
   - Action discussion — ⑥
   - Wrapped recap — ⑦
6. After each screen, show me a screenshot/preview and wait for sign-off before moving on.

## Hard rules

- **Column accents are semantic.** Mood = amber, well = green, wrong = red, actions = violet. Never mix.
- **Caveat font is for wall-moments only**: column headers, board name, empty-state coaching. Everything else = Inter.
- **Cards always carry the column accent color as background**, with white text, `--shadow-2` + `--card-inset-hi` + `--card-inset-lo`.
- **Paper texture is non-negotiable.** Apply `.sp-paper` to the app body. Apply `.sp-card-grain` to every Card. (See TOKENS.css.)
- **Slight rotations & tape are reserved** for the New-Board template carousel and the Wrapped recap. Never on the live editing surface.
- **No emoji as iconography.** If you need icons, use our existing icon set (or Lucide / Heroicons) in monoline style at 16–20px, color = `--fg-2` or column accent.
- **Don't invent screens.** If a screen exists in the mock but our app doesn't have it yet, flag it and ask. If a screen exists in our app but not the mock, ask if it should be removed.

## State / behavior

The README has a `State model` section with TypeScript types and an `Interactions & behavior` section with details on realtime presence, vote casting, cluster drag, and stage transitions. Use those as the spec. Where our existing app already handles a behavior differently, propose the diff and ask before refactoring.

## Reporting back

After each milestone (token swap, each primitive, each screen): summarize what changed, list files touched, paste a screenshot or describe the visual state, and wait for go-ahead.

## Open questions to resolve with me before/during work

- Mobile breakpoint: collapse columns to tabs, or horizontal swipe?
- Dark mode: are we doing one? (Not in current spec.)
- Accessibility: column membership is currently color-only. We need a redundant signal (icon or pattern in column header). Propose options.
- Motion: confirm durations/easings in README — or substitute with our motion lib's defaults if we have one.

Start with step 1 (survey). Don't change any files yet.
