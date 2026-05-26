# Handoff: Spill. — Applied Design (Daylight Cork)

A retro app built around a coffee-spill metaphor. Users dump messy honest thoughts onto a "pinboard" across four columns (mood / went well / went wrong / actions), then cluster, vote, and decide together.

This package transforms the **locked wireframes** into a **production-ready visual system** so your engineering agent can implement it against your existing partially-working app.

---

## 📦 What's in this folder

| File | Purpose |
|---|---|
| `README.md` | This document — the one source of truth for the redesign. |
| `Spill App.html` | Open in a browser. The full applied design — 7 screens + design system reference, all on a pannable canvas. |
| `spill-app.jsx` | All screens + components + tokens as React (Babel-transpiled). Read this for exact values. |
| `spill-app-main.jsx` | Canvas assembly. Shows screen order. |
| `design-canvas.jsx` | Just the canvas viewer wrapper. **Not needed for implementation** — it's the presentation shell. |
| `TOKENS.css` | Drop-in CSS custom properties — the entire design system as variables. |
| `AGENT_PROMPT.md` | Copy/paste prompt to give your coding agent. |

---

## ⚠️ Read this first — what these files are

The files in this bundle are **design references created in HTML/React-via-Babel.** They are prototypes showing intended look and behavior, **not production code to copy directly.**

Your task is **not** to ship `spill-app.jsx`. Your task is to **recreate these designs inside your existing app's codebase**, using its established patterns (component library, routing, state management, CSS approach). The HTML mock is the spec; your codebase is the canvas.

**Fidelity:** This is **high-fidelity.** Colors, type, spacing, shadows, and component states are all final. Match them pixel-close. Where the mock used inline styles for speed, translate to your codebase's styling system (Tailwind, CSS modules, styled-components, etc.) — the *values* are what matter.

---

## 🎨 Design system (Daylight Cork)

### Color tokens

```
/* Surfaces — paper-y, warm, low-contrast */
--paper:    #f3e8cf   /* app canvas background */
--paper-2:  #ecdcb8   /* slightly darker paper, used in chrome */
--panel:    #fbf3df   /* raised panels, top bar, tiles */
--panel-hi: #fff8e6   /* hover / input fields */
--line:     #d9c89e   /* 1px borders */
--line-2:   #c4ae7a   /* darker borders, dividers */

/* Ink */
--fg:       #1f1812   /* primary text */
--fg-2:     #4a3d2e   /* body text */
--muted:    #86755a   /* labels, meta */

/* Column accents — semantic, ONE per column */
--c-mood:    #cf8a3f   /* mood / how-you-feel */
--c-well:    #2f9469   /* went well */
--c-wrong:   #cf4f4f   /* went wrong  (also = brand primary) */
--c-act:     #8757b6   /* actions */

/* Inner-card tints (for hover / highlights, paired with above) */
--c-mood-in:  #e9a44e
--c-well-in:  #3eb486
--c-wrong-in: #e26565
--c-act-in:   #a173d0
```

**Rule:** Column accents are **semantic only**. Never use violet for a "went-well" item, never use green for "actions." Color = column membership.

### Typography

```
--font-ui:   "Inter", system-ui, -apple-system, "Segoe UI", sans-serif;
--font-hand: "Caveat", cursive;
```

Caveat is reserved for **one expressive wall-moment per screen** (column headers, board titles, the occasional sticky-note callout). All UI chrome, body, controls, labels = Inter.

**Type scale used:**
| Token | px | weight | usage |
|---|---|---|---|
| display-xl | 48–64 | 800 | hero numbers / board names in Caveat |
| display-lg | 28 | 800 | board titles |
| h1 | 22 | 700 | screen titles |
| h2 | 16 | 700 | section headers |
| body | 13.5 | 500 | card content |
| ui  | 12.5 | 600 | buttons, controls |
| meta | 11 | 600 | pills, status |
| micro | 10 | 700 | uppercase labels (letter-spacing 1.4) |

### Shadows (elevation)

```
--shadow-1: 0 1px 0 rgba(74,52,20,0.06), 0 2px 6px rgba(74,52,20,0.06);
--shadow-2: 0 1px 0 rgba(74,52,20,0.08), 0 8px 18px -4px rgba(74,52,20,0.12);
--shadow-3: 0 2px 0 rgba(74,52,20,0.08), 0 16px 32px -8px rgba(74,52,20,0.18);
--focus:    0 0 0 3px rgba(207,79,79,0.20);
```

Cards always carry `--shadow-2` + an `inset 0 1px 0 rgba(255,255,255,0.18)` highlight + `inset 0 -1px 0 rgba(0,0,0,0.08)` lip.

### Radius

```
--r-sm: 6px    /* chips, tiny inputs */
--r-md: 8px    /* cards, buttons, tiles — DEFAULT */
--r-lg: 12px   /* panels, modals */
--r-pill: 999px /* pills, avatars, segmented controls */
```

### Surface textures (the soul of the look)

Two layered CSS effects that make the app feel like paper, not Material:

1. **Paper grain** on the app background — overlapping radial dots + repeating diagonal hairlines at ~3–5% opacity.
2. **Card grain** — fine cross-hatched lines at ~2% opacity, overlay blend mode.

See `.sp-paper`, `.sp-panel-grain`, `.sp-card-grain` classes in `spill-app.jsx` (lines ~54–95) for the exact CSS — copy directly into your global stylesheet.

### Decorative motifs (use sparingly!)

- **Tape strips** — small yellow tape rectangles, only on the Wrapped/Summary screen and the New-Board hero.
- **Pins** — round 3D-shaded dots, only when an item is "pinned" by the user.
- **Slight rotations** — ±1°, only on the New-Board template carousel and the Wrapped recap cards. Never on the live editing surface.
- **Caveat handwriting** — board names, column headers, and the empty-state coaching only.

---

## 🧩 Components inventory

All defined in `spill-app.jsx`. Implement these as your own components in your stack.

### Atoms
- `<Logo>` — SVG mug + spill + "Spill•" wordmark. Two sizes (22, 32). Mark + wordmark are inseparable.
- `<Btn kind="primary|secondary|ghost|dashed" accent={hex} icon>` — pill-rounded? No, **8px radius**. Primary uses a linear gradient `accent → shade(accent, -8%)` with inset highlight.
- `<Pill tone="neutral|solid|soft|ghost" accent>` — for status badges and tags.
- `<Avatar k="initials" color status="ready|writing|voting|away">` — 26px gradient circle, optional status dot.
- `<Stack people>` — overlapping avatars, -8px margin.
- `<Card accent={columnColor}>` — the core post-it sleeve. Always carries column accent. **No rotation on the editing surface.**
- `<Chip>`, `<GifTile>` (procedural placeholder), `<Tape>`, `<Pin>`.

### Molecules
- **Top bar** — Logo · breadcrumb · board status pill · participants stack · live timer · action button. Always panel-grain background.
- **Column header** — Caveat label, count pill, "add" affordance.
- **Composer** — sticky-note styled textarea + accent picker dot row + post button.
- **Vote dot** — small filled circle, three states: empty / 1-vote / capped (3 votes max per person).

### Screen-level chrome
- **Floating action bar** (bottom) — for context actions per screen.
- **Stage indicator** — top center, shows ① WRITE → ② CLUSTER → ③ VOTE → ④ ACT. Active step uses `--c-wrong`.

---

## 📺 Screen-by-screen reference

Each screen is sized **1240 × 760** in the mock (desktop-first). Implement responsive breakdowns to taste — likely a single-column mobile view collapsing the columns to a tab-switcher.

### ① Overview — `window.real_Overview` (line 389 in spill-app.jsx)
**Purpose:** User lands here. Sees boards across all teams.
**Layout:**
- Left rail (240px) — workspace switcher, teams nav.
- Main grid — board cards in a 3-column grid, 16px gap.
- Each card shows: title (Caveat), status pill, "WRITING / VOTING / DONE" tag, participant stack, last-touched timestamp.
- "+ new board" card uses `Btn kind="dashed"` styling at card scale.

### ② New Board — `window.real_NewBoard` (line 537)
**Purpose:** Create a board. Pick template, name it, invite team.
**Layout:**
- Centered modal (~720px wide).
- Template carousel with 4 options (Standard / Sailboat / Start-Stop-Continue / Mad-Sad-Glad). Each is a mini board preview tilted ±1°.
- Form below: board name input (Caveat font, large), date picker, participant chips with add-by-email.
- Primary CTA bottom-right.

### ③ Writing — `window.real_Writing` (line 734) ⭐ THE CORE SCREEN
**Purpose:** Live collaborative post-it writing across 4 columns.
**Layout:**
- Top bar (board name + live timer + participants stack with status dots).
- 4 columns, equal width, vertical scroll within each.
- Each column: Caveat header + count pill + cards stack + composer at bottom.
- Composer is "always there" — a textarea that lives at the bottom of the column the user is focused on.
- "Live caret" on cards being typed by others — show a colored vertical bar with the author's color.

### ④ Cluster — `window.real_Cluster` (line 956)
**Purpose:** Drag-merge related cards. AI suggests clusters.
**Layout:**
- Same 4-column structure as Writing.
- AI suggestions appear as **dashed group outlines** around 2-3 cards with a "merge?" badge.
- User can drag a card onto another to manually cluster.
- A clustered group shows a count badge ("3 cards") and a parent title input.

### ⑤ Voting — `window.real_Voting` (line 1046)
**Purpose:** Each person gets 3 dots per column. Vote on what to discuss.
**Layout:**
- Same 4 columns.
- Each card now shows a row of vote dots below content.
- Top right: "your votes: 8 / 12 remaining" indicator.
- Cards sort live by vote count.

### ⑥ Action Discussion — `window.real_Action` (line 1163)
**Purpose:** Top-voted items become discussion focus. Decide on actions.
**Layout:**
- Split: left = focused card (large, centered), right = comments thread + action items list.
- "Next" button advances through top items in order.
- Action item creation form at bottom — owner picker, due date, copy.

### ⑦ Wrapped (Summary) — `window.real_Summary` (line 1336)
**Purpose:** Spotify-Wrapped-style recap. Shareable.
**Layout:**
- Centered "card-deck" of recap moments — top quote, biggest theme, action count, MVP shout-out.
- Each recap card uses tape, slight rotation, big Caveat numbers.
- Share / export CTAs at bottom.

---

## 🔄 Interactions & behavior

### Realtime presence
- Avatars in the top bar show status: `ready` (green dot), `writing` (amber, pulsing), `voting` (violet), `away` (gray).
- When user A is typing on a card, card shows colored caret with A's color.
- Use Y.js / Liveblocks / your existing realtime layer.

### Stage transitions (① → ② → ③ → ④)
- Facilitator advances the stage via top-bar button. Animation: column contents shift up 8px and back, accent line under stage indicator slides.
- Other participants get an in-app toast: "Sarah moved you to Voting."

### Vote casting
- Click card → dot fills with your color. Click again → unfill.
- Max 3 votes per column per person. When at cap, all empty dots in that column go to 30% opacity.
- Vote totals update live across clients.

### Cluster drag
- Card hover at >150ms = lifts (shadow-3, no rotation).
- Dropping on another card = merge animation: both rotate 1°, snap together, group outline draws in over 200ms.
- AI suggestion clusters appear with a 400ms outline-draw animation when generated.

### Composer
- Empty state shows Caveat-font placeholder ("what's on your mind, na?" with the user's initials).
- ⌘+Enter posts. Each post slides up from the composer position with a 220ms ease-out.
- Optional GIF picker (button in composer) — opens search modal; selected GIF embeds in card.

---

## 🗂️ State model

```ts
type Board = {
  id: string
  name: string
  template: 'standard' | 'sailboat' | 'ssc' | 'msg'
  stage: 'lobby' | 'writing' | 'cluster' | 'voting' | 'action' | 'wrapped'
  createdAt, scheduledFor, endedAt
  facilitatorId: string
  participantIds: string[]
}

type Card = {
  id: string
  boardId: string
  column: 'mood' | 'well' | 'wrong' | 'act'
  authorId: string
  text: string
  gifUrl?: string
  clusterId?: string  // if grouped
  pinned?: boolean
  createdAt
}

type Vote = { boardId, cardId, voterId, createdAt }

type Action = {
  id, boardId, cardId, ownerId, dueDate, text, completed: boolean
}

type Presence = {
  userId, boardId, status: 'ready'|'writing'|'voting'|'away',
  typingOnCardId?: string, focusedColumn?: string
}
```

Each card belongs to exactly one column (the column accent = `Card.column`). Votes are scoped per-board-per-voter; enforce max 3 per column server-side.

---

## ✏️ Migration from your current wireframe-based UI

Since you have a partially working app on the old wireframe look:

1. **Don't rebuild from scratch.** Find your current `theme.ts` / `tokens.css` / Tailwind config and **replace the values** with those in `TOKENS.css`.
2. **Add the paper-grain CSS** to your global stylesheet (see `.sp-paper`, `.sp-card-grain` in `spill-app.jsx` lines 54–95).
3. **Swap Caveat → Inter** as the default body font. Reserve Caveat for the wall-moments listed above.
4. **Refactor card component** — your existing cards probably look like generic divs. Wrap them in the `<Card accent>` pattern with the new shadow + inset-highlight stack.
5. **Then go screen by screen.** Compare your current screen to the `real_X` mock side-by-side and reconcile.

---

## 🤖 For your coding agent

Use the prompt in `AGENT_PROMPT.md` — it's pre-written. Drop this entire folder into your repo (e.g. `/docs/design/`) and reference it from the prompt.

## ❓ Open questions (decide before implementing)

- **Mobile breakpoint behavior** — collapse columns to tabs, or horizontal swipe? (Mock is desktop-first.)
- **Dark mode** — not designed. Are we doing it? If yes, needs a follow-up pass.
- **Accessibility** — color-only column membership won't pass WCAG. Need to add column **icons** or **patterns** as a redundant signal. Recommend small monoline icons in column headers.
- **Animations spec** — durations/easings above are recommendations. Confirm with motion stakeholder.
