# Spill. - Designer HTML Mock Brief

## Goal

Produce a detailed HTML mock package that shows how Spill. works across its main phases, major states, and AI-assisted behaviors.

The mock should be realistic enough to review product flow, information architecture, and UX behavior before implementation planning.

## Core layout principle

Except for the retro index / overview / history entry points, the whole retro experience should happen inside the board view.

That means:

- writing
- private draft review
- user deck placement
- reveal
- discussion
- clustering
- voting
- action discussion
- summary output

should all feel like states, panels, trays, overlays, or focused regions around the same main board surface.

Do not design the core retro flow as a sequence of disconnected pages. The product should feel frictionless and spatially consistent end to end.

## Product tone

- calm but expressive
- safe place to share wins and pains
- playful, not silly
- AI is helpful and backgrounded, not loud
- keep the UX simple
- prefer fewer surfaces, fewer clicks, and fewer competing controls
- avoid cleverness that makes the retro harder to run

## Primary flows to cover

### Flow A: Standard retro without connectors
- create retro
- writing in draft mode
- reveal
- discussion
- voting
- action discussion
- summary/end

### Flow B: Standard retro with personal AI-fed drafts
- user receives private suggestions
- user reviews/edits/rejects
- user sends approved items to board
- cards land in private drafts

### Flow C: Custom retro with user deck
- non-standard board template
- personal suggestions land in user-only deck
- user drags deck items into chosen columns

## Screens and views required

### 1. Retro index / dashboard
Show:

- upcoming retros
- recent retros
- quick status of active retro
- access to previous boards
- filters or search for past retros

Include states:

- empty state
- list with active retro
- list with historical retros

### 2. Create retro / template picker
Show:

- standard template
- custom template
- settings for:
  - votes per person
  - top N voted items for action discussion
  - clustering mode:
    - manual one-off
    - auto once on vote start
  - direct map vs user deck ingestion mode

### 3. Board - writing phase, standard template
This is a critical screen.

Must show:

- columns: Mood / Went well / Went wrong / Actions
- cards in private draft mode
- your own cards readable
- others' cards hidden or blurred
- ready progress indicator
- clear phase indicator: Writing
- optional access point for personal AI suggestions
- GIF attach/search affordance

Behavior notes to reflect:

- only the current user sees their draft card content
- others are represented as blurred placeholders or counts
- ready action is available
- board cannot reveal early by accident

### 4. Personal AI review panel / modal
Show the private suggestion review experience.

Must show:

- grouped suggestions:
  - mood
  - went well
  - went wrong
- ability to edit text
- ability to reject suggestions
- ability to reorder suggestions
- button to send approved items to board

Need two variants:

#### A. Direct map mode
Approved suggestions go directly into private draft cards in mapped columns.

#### B. User deck mode
Approved suggestions go into a private "My deck" area.

### 5. Board - writing phase with user deck
Show:

- custom template with more or different columns
- private "My deck" tray/backlog
- drag from deck into columns
- cards still hidden from others

Important:

- this is still the same board view
- the deck should appear as an adjacent tray, side panel, drawer, or embedded region
- the user should be able to move from deck to columns without navigating away

### 6. Board - reveal state / discussion state
Show what the board looks like once everyone is ready and cards are visible.

Must show:

- cards revealed to all
- ready state resolved
- phase indicator: Discussing
- no forced interaction required
- optional note that clustering can be triggered on demand

Need a version with:

- unclustered cards
- a visible "Cluster-fy" action

### 7. Cluster suggestion / clustered board state
Show the one-off clustering result.

Must show:

- clustered cards or grouped themes
- clear indication clustering happened once
- affordance for human adjustment if intended
- no suggestion that clustering will keep changing in the background

Need two cases:

#### A. Manual cluster-fy
User clicked button during or after discussion.

#### B. Auto on vote start
Voting opened and the board clustered once automatically.

### 8. Voting phase
This is another critical screen.

Must show:

- phase indicator: Voting
- default 3 votes per person
- remaining votes visible
- ready progress indicator
- vote affordances on cards or clusters
- whether multiple votes on a card are allowed should be visually legible if configured

Behavior notes:

- voting is async
- users must mark ready
- no clustering changes after voting has started
- voting should happen in-place on the board, not in a separate voting screen

### 9. Transition into action discussion
Show how top voted issues become the agenda for action discussion.

Must show:

- ranked voted cards/clusters
- top N selected for discussion
- AI-proposed actions attached to those items
- a clear distinction between:
  - voted problem
  - proposed action
  - confirmed action

Important:

- this should still be anchored in the board view
- action discussion can use a focused side panel, bottom sheet, spotlight region, or similar in-place treatment
- avoid switching to a detached workflow page

### 10. Action discussion view
Show the sync action review flow.

Must show:

- one voted issue in focus
- proposed actions
- ability to accept, edit, reject, or add action
- owner / follow-up fields if included
- navigation through top voted items one by one

Behavior note:

- AI can propose; humans decide
- action discussion should preserve visibility of the board context while focusing the current issue

### 11. End state / summary screen
Show:

- concise retro summary
- agreed actions
- team mood
- optional outbound destinations:
  - Slack
  - Jira / Linear
  - copy/share/export

The summary should not look like raw meeting notes. It should look like a compact digest.

Important:

- end state should preferably feel like the board entering a completed mode, not the user being taken to an unrelated summary page

### 12. Past retro detail view
Show:

- completed board
- summary
- actions
- votes
- mood result
- connection to previous/following retros
- recurring theme references if available

### 13. History / trends view
Designer should explore a simple but useful history screen.

Potential elements:

- list of past retros
- recurring themes
- unresolved actions
- repeated pain areas

No heavy analytics dashboard. Keep it practical.

### 14. Settings / integrations view
Show:

- basic retro defaults
- vote defaults
- clustering defaults
- connector configuration entry points
- summary delivery configuration
- AI feature toggles

## Key UX behaviors to make explicit in mock notes

The HTML mock should annotate behavior, not just draw pages.

### Draft privacy
- before everyone is ready, card contents remain private
- after reveal, cards are fully visible

### Ready behavior
- ready exists only in writing and voting
- when all are ready, the next phase can begin

### Discussion behavior
- discussing is mostly off-board conversation
- the board is present but not demanding

### Clustering behavior
- clustering is one-off
- it can happen manually or once on vote start
- no repeated reclustering loop

### Action behavior
- actions are generated from prioritized voted pain points
- AI suggestions must never be presented as final decisions

### Team mood behavior
- mood is playful and lightweight
- use current labels:
  - quietly-proud
  - smooth-sailing
  - good-sparks
  - productive-chaos
  - foggy
  - spicy
  - stuck-in-mud
  - needs-a-map
- mood should have a short explanation

## Edge and state coverage

The mock package should include these cases:

- no cards yet
- one person not ready
- all ready, reveal pending
- no connector suggestions available
- many connector suggestions available
- clustering produces obvious good results
- clustering produces questionable results needing human adjustment
- tie in top voted items
- no clear action agreed
- summary send success
- summary send failure

## What the designer should not optimize for yet

- final visual identity
- perfect branding
- implementation constraints
- enterprise admin workflows

Focus first on:

- screen completeness
- state completeness
- interaction clarity
- phase transitions
- trust and privacy cues
- single-view board continuity

## Deliverable expectation

The HTML mock should cover:

- major pages
- important modals/panels
- phase transitions
- empty/loading/error states where relevant
- inline notes describing expected UX behavior

The result should be detailed enough to turn into an MVP product requirements doc after review.
