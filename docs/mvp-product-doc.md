# SpillItOut - MVP Product Doc

## Status

Recommendation: continue.

The MVP is coherent and bounded: a board-first retrospective tool with GIF support, async/sync phase flow, private writing, voting, action follow-through, and optional board-level AI helpers.

## User problem

Teams need a lightweight retrospective tool that remains expressive and simple while supporting remote/hybrid work. Existing tools either removed important expressive features, put useful capabilities behind paid plans, or overcomplicate the meeting.

SpillItOut should let a team:

- write privately before reveal
- discuss together without the tool fighting for attention
- vote on what deserves action
- convert top voted pain into agreed actions
- preserve previous boards
- use GIFs as a core part of expression
- optionally use AI for low-friction organization and follow-through

## Evidence and settled decisions

These decisions are treated as product constraints for MVP:

- Product name: **SpillItOut**
- Developer/project nickname: **spillio**
- Project folder: `/home/lgrossi/projects/spillio`
- Core app works without AI or integrations
- GIF support is core, not optional polish
- Whole retro flow happens in the board view, except overview/history/settings entry points
- Board starts in private draft mode
- Ready state exists only for Writing and Voting
- Default vote count is 3 votes per person
- Actions are discussed from top voted pain points
- Default top voted action discussion count is 3
- Clustering is one-off, either manual or once on vote start
- Personal AI is connector-fed, not an always-on built-in MVP authoring surface
- App AI handles board-level features only
- Team mood is lightweight, team-level, and non-diagnostic

## Assumptions

- Initial users are internal teams already familiar with EasyRetro-like boards.
- First hosted deployment can prioritize company use over public SaaS concerns.
- MVP can use mock or limited connectors if core ingestion API shape is present.
- Meeting notes support can start as uploaded text, not live transcript integration.
- Strong implementation planning comes after this doc, not inside it.

## Non-goals for MVP

- autonomous AI-generated retros
- AI-led conversational facilitation
- employee analytics or individual productivity scoring
- full Slack/Jira/Linear automation parity
- live meeting transcription
- complex analytics dashboards
- enterprise admin, billing, or marketplace concerns
- mobile-first experience

## Success signal

The MVP is successful if a team can run a complete retro end to end and prefer it over EasyRetro for the target use case because it has:

- simple flow
- reliable GIF support
- private writing/reveal
- voting and action discussion discipline
- board history
- optional helpful AI that does not feel intrusive

## MVP scope summary

### In scope

- retro overview/history entry page
- create retro from standard or custom template
- single board view for all retro phases
- private draft writing
- ready tracking for writing and voting
- reveal after writing
- GIF search and attachment
- direct card creation/edit/delete
- drag-and-drop card movement
- per-column ordering controls, including reverse order
- one-off clustering
- voting
- top voted action discussion
- action cards
- summary/completed board state
- board history
- connector ingestion API for draft cards/user deck
- first-party personal AI companion connectors for Pi and Claude Code
- app AI interfaces for GIF suggestions, clustering, action suggestions, summary, mood, and tagging

### Out of scope for first MVP

- production-grade connector marketplace
- personal AI UI inside the retro app
- relying on third parties to provide the first useful personal AI integration
- automatic public card creation by connectors
- repeated background clustering
- per-person mood scoring
- hidden source ingestion
- full metrics product

## Primary personas

### Team member

Wants to add honest retro input with low friction, optionally use prepared suggestions, attach expressive GIFs, vote, and see outcomes.

### Retro host

Wants to create a board, move through phases, know when async steps are complete, keep discussion focused, and finish with clear actions.

### Team lead / recurring participant

Wants past boards, unresolved actions, repeated pain, and lightweight team mood without creating surveillance dynamics.

### Connector / personal AI producer

External harness/skill/tool that helps a user prepare cards and sends approved output to SpillItOut.

For MVP, this is outside the web app, but it should still be first-party. SpillItOut should provide at least a Pi companion and a Claude Code companion so the first personal AI workflow is usable without waiting for external ecosystem adoption.

## Core concepts

### Retro

A retrospective board instance with participants, template, phase, settings, cards, votes, actions, summary, mood, and history metadata.

### Board view

The main surface for writing, revealing, discussing, voting, discussing actions, and completing the retro.

### Card

A user-authored or user-approved item inside a column. Cards may include text, author visibility metadata, GIF attachment, votes, tags, cluster membership, and action linkage.

### Draft card

A private card visible only to its author before reveal.

### User deck

A private backlog/tray for connector-ingested suggestions when the current board template cannot be directly mapped to known standard columns.

### Cluster

A one-off grouping/merge of similar cards. Clusters are never continuously regenerated.

### Action

A confirmed follow-up card derived from a voted pain point and agreed by the team.

## MVP phase flow requirements

### R1. Writing phase

The board starts in Writing.

Requirements:

- card contents from other users are hidden/blurred
- user can read and edit their own draft cards
- user can add cards manually
- user can attach or search GIFs
- user can mark ready
- host can see ready progress
- reveal cannot happen accidentally before the host/team advances

Acceptance criteria:

- Given a new retro, when a participant opens the board, then the phase is Writing.
- Given another user's draft card exists, when I view the board before reveal, then I cannot read its contents.
- Given I have draft cards, when I view the board before reveal, then I can read and edit my own cards.
- Given I mark ready, when the ready indicator updates, then my ready state is reflected in the board progress.
- Given not everyone is ready, when the host views the board, then the board clearly shows who/what is still pending without exposing private card content.

### R2. Reveal and discussion phase

After writing completes, cards are revealed and the team discusses.

Requirements:

- revealed cards are visible to all participants
- no ready state in discussion
- board remains a reference surface
- Cluster-fy is available if manual clustering mode is enabled
- discussion does not force modal workflows

Acceptance criteria:

- Given writing has ended, when the board enters Discussion, then all draft card contents are visible.
- Given the board is in Discussion, then no ready button is shown.
- Given manual clustering is enabled and clustering has not happened, then Cluster-fy is available once.

### R3. Clustering

Clustering is a one-off board mutation.

Requirements:

- clustering can happen manually once or automatically once when voting starts
- a retro cannot use both manual clustering and auto-on-vote-start clustering in the same run
- clustered state is clearly labeled
- clustered cards preserve original card text/history
- human adjustment is allowed if included in the implementation slice
- no repeated reclustering loop

Acceptance criteria:

- Given manual clustering mode, when the host clicks Cluster-fy, then one clustering pass is applied.
- Given clustering has already run, when the user views the board, then Cluster-fy cannot be run again without an explicit reset/admin action.
- Given auto clustering mode, when Voting starts, then clustering runs once before votes are cast.
- Given voting has started, then clustering no longer changes.

### R4. Voting phase

Voting is async and in-place on the board.

Requirements:

- default votes per participant: 3
- vote count configurable per retro/template
- remaining votes visible
- participant can mark ready after voting
- ready state exists in Voting
- voting targets are cards or clusters depending on current board state
- clustering is frozen before votes are placed

Acceptance criteria:

- Given default settings, when Voting starts, then each participant has 3 votes.
- Given I cast a vote, then my remaining vote count decreases.
- Given I have no votes left, then I cannot cast additional votes.
- Given I mark ready in Voting, then ready progress updates.
- Given the board is in Voting, then votes are cast in the same board view.

### R5. Action discussion phase

Actions are discussed from top voted pain points.

Requirements:

- top voted items drive action discussion
- default top N is 3
- top N configurable per retro/template
- action candidates may be AI-proposed
- humans must confirm/edit/reject actions
- action discussion happens in the board view, anchored to the Actions column or in-place panel
- no ready state in action discussion

Acceptance criteria:

- Given Voting is complete, when Action Discussion starts, then the top voted pain points are shown as the action agenda.
- Given AI action proposals are enabled, when top voted items are available, then proposed actions are displayed as editable drafts, not confirmed actions.
- Given a participant edits and confirms an action, then the action becomes a confirmed action card.
- Given an AI proposal is rejected, then it is not included in confirmed actions.
- Given there are ties around the top N cutoff, then the UI shows the tie and lets the team include/exclude tied items.

### R6. Summary/completed phase

The completed retro remains a board, with digest and follow-up output.

Requirements:

- summary appears as completed board state, not detached page
- summary is concise and outcome-focused
- agreed actions are visible
- team mood is shown if enabled
- send/export status is visible
- summary send failure is visible and recoverable

Acceptance criteria:

- Given a retro ends, when the board enters Completed, then the completed board shows summary, mood, and actions.
- Given outbound delivery succeeds, then the board shows delivery success.
- Given outbound delivery fails, then the board shows failure and a retry/reconnect path.
- Given AI summary is disabled, then the board can still be completed manually.

## Board and card requirements

### R7. Templates and columns

Requirements:

- standard template columns:
  - Mood
  - Went well
  - Went wrong
  - Actions
- custom templates with arbitrary columns
- direct-map ingestion for standard semantic columns
- user deck ingestion for custom/non-standard boards

Acceptance criteria:

- Given a standard template, when connector items arrive with mood/wentWell/wentWrong, then they land in matching private draft columns.
- Given a custom template using user deck mode, when connector items arrive, then they land in the user's private deck.
- Given a user deck item, when the user places it in a column, then it becomes a private draft card in that column.

### R8. Card editing and movement

Requirements:

- create card
- edit own card while allowed
- delete own card while allowed
- drag cards across columns where allowed
- reorder cards within a column
- per-column reverse order control

Acceptance criteria:

- Given I own a draft card, when I edit it during Writing, then the updated content is saved.
- Given a column supports reordering, when I drag a card within the column, then the new order is saved.
- Given a column reverse setting is enabled, when the board renders that column, then the order is reversed for that column only.

### R9. GIF support

GIF support is core.

Requirements:

- search GIFs from card composer/edit UI
- attach GIF to card
- replace/remove GIF
- show GIF preview on card
- app AI may suggest GIF searches/results by lenses
- raw search remains available

Suggested GIF lenses:

- literal
- emotional
- chaotic/funny

Acceptance criteria:

- Given I create/edit a card, when I search for a GIF, then I can select and attach a GIF.
- Given a GIF is attached, when the card is shown, then the GIF preview is visible.
- Given AI suggestions are available, when I open GIF suggestions, then I can browse suggested result groups and still run my own search.
- Given GIF provider fails, then card text entry remains usable.

## Connector ingestion requirements

### R10. Ingestion API

The ingestion API accepts user-approved personal AI output.

Minimum payload semantics:

- personal mood
- wins/cheers
- pains

Implementation names:

- `mood`
- `wentWell`
- `wentWrong`

Modes:

- `direct_map`
- `user_deck`

Requirements:

- connector output is private to the user by default
- connector cannot create public cards directly by default
- user can review/edit before reveal
- ingestion records source metadata sufficient for trust/debugging

Acceptance criteria:

- Given a connector sends direct-map content for a standard board, then content appears as private draft cards for that user.
- Given a connector sends user-deck content, then content appears in that user's private deck.
- Given connector content arrives, then other participants cannot read it before reveal.
- Given a connector item includes source metadata, then the user can inspect at least source name/time window in review context.

### R10a. First-party personal AI companions

Personal AI is not built into the web app as an always-on authoring surface, but the MVP should include first-party companion tooling that produces ingestion payloads.

Required MVP companions:

- Pi companion
- Claude Code companion

Companion responsibilities:

- read only explicitly allowed local/session context
- summarize high-level retro-relevant material
- produce user-reviewable suggestions for:
  - mood
  - wins/cheers
  - pains
- avoid workflow nitpicks and personal productivity judgments
- require user approval before sending anything to SpillItOut
- send approved items through the ingestion API

Suggested default companion prompt shape:

```text
Review my allowed AI session context for the current retro period.
Surface only high-level work situations worth sharing with the team.
Prepare concise draft cards for:
- how I felt / team mood contribution
- what went well / wins / cheers
- what went wrong / pains / friction

Do not score me.
Do not mention tool usage trivia, command counts, time spent, or productivity metrics.
Prefer project situations, collaboration friction, decisions, blockers, accomplishments, and support needs.
Keep outputs editable and suitable for me to approve before sending to SpillItOut.
```

Acceptance criteria:

- Given a Pi user has opted into a retro context window, when they run the Pi companion, then it produces reviewable mood/win/pain suggestions.
- Given a Claude Code user has opted into allowed session logs, when they run the Claude companion, then it produces reviewable mood/win/pain suggestions.
- Given a user approves suggestions, when the companion sends them, then SpillItOut receives them as private draft or user deck items.
- Given a user rejects suggestions, then nothing is sent to SpillItOut.
- Given companion output is generated, then the user can see the source category/time window before sending.

## AI requirements

### R11. App AI GIF suggestions

Requirements:

- suggest GIFs/searches for existing cards
- group suggestions by lens
- never remove manual search

Acceptance criteria:

- Given a card has text, when GIF suggestions are requested, then the app suggests GIF options grouped by lens.
- Given suggestions are bad, then the user can ignore them and search manually.

### R12. App AI clustering

Requirements:

- cluster semantically similar cards
- preserve originals
- generate optional tags/categories for later history/metrics
- run once per retro

Acceptance criteria:

- Given unclustered cards, when clustering runs, then similar cards are grouped without losing original content.
- Given tags are generated, then they are stored as metadata even if not user-visible in MVP.

### R13. App AI action proposals

Requirements:

- propose actions only from prioritized voted pain points
- produce editable drafts
- avoid final/authoritative language

Acceptance criteria:

- Given top voted pain points exist, when action proposals are generated, then each proposal links back to its source voted item.
- Given a proposal is accepted, then it becomes a confirmed action only after human confirmation.

### R14. App AI summary

Requirements:

- generate concise summary at completion
- include wins, pains, actions, and unresolved items
- use uploaded meeting notes when provided
- avoid acting as full meeting notes

Acceptance criteria:

- Given cards, votes, and actions exist, when summary is generated, then it produces a compact digest.
- Given uploaded meeting notes exist, when summary is generated, then the digest can use them as additional context.
- Given meeting notes are absent, then summary still works from board data.

### R15. App AI team mood

Requirements:

- team-level only
- optional
- playful labels:
  - Stormy
  - Off
  - Mixed
  - Steady
  - Flying
- short explanation
- may use uploaded meeting notes when provided
- no per-person mood scoring

Acceptance criteria:

- Given mood is enabled, when the retro completes, then the board shows one team mood label and a short explanation.
- Given evidence is insufficient, then the mood can be omitted or shown with low-confidence language.
- Given individual card authors exist, then team mood does not expose individual scoring.

## History and follow-through requirements

### R16. Previous boards

Requirements:

- list previous retros
- open a completed retro as a read-only/reloaded board
- show summary, mood, votes, and actions

Acceptance criteria:

- Given a retro is completed, when I view history, then the retro appears in the list.
- Given I open a past retro, then I see the completed board and associated summary/actions.

### R17. Recurring memory

MVP should start the metadata foundation for retro memory without becoming a heavy analytics product.

Requirements:

- store AI/user tags for themes
- show simple recurring themes if available
- show unresolved actions
- avoid heavy dashboards

Acceptance criteria:

- Given multiple retros have tagged themes, when I view history, then recurring themes can be listed.
- Given actions remain unresolved, when I view history, then unresolved actions are visible.

## Settings requirements

### R18. Board defaults

Requirements:

- votes per person
- top N action discussion count
- clustering mode
- direct map vs user deck ingestion mode
- AI feature toggles
- outbound delivery configuration

Acceptance criteria:

- Given a team default is set, when a new retro is created, then it inherits that default.
- Given a retro-specific setting is changed, then it applies to that retro without changing team defaults unless explicitly saved as default.

## Privacy, trust, and safety requirements

### R19. Privacy defaults

Requirements:

- draft cards private before reveal
- connector ingestion private by default
- source ingestion opt-in
- source metadata visible to the user receiving suggestions
- no individual productivity scoring

Acceptance criteria:

- Given AI/connectors are disabled, then the board still works.
- Given connector content exists, then it is not visible to others until the user places/sends/reveals cards through normal board flow.
- Given AI output references sources, then the user can understand what source category/time window produced the suggestion.

### R20. AI behavior boundaries

Requirements:

- AI should surface team-relevant work situations
- AI may surface individual stories only when useful for celebration, support, or shared learning
- AI must not nitpick tool usage, command counts, time spent, or personal productivity trivia
- AI output should be editable/rejectable

Acceptance criteria:

- Given AI action or summary output appears, then users can edit or reject it before finalization where applicable.
- Given an AI theme is generated, then it uses situation/project framing rather than individual judgment language.

## Technical direction for MVP

Source design artifact: `docs/tech-stack-design.md`.

### Recommendation

Use a split two-service deployment:

- Rust API/WebSocket backend
- Next.js frontend
- Postgres database

The product is greenfield and expected to be vibe-coded. Stack choice must therefore optimize both platform fit and AI training-data coverage.

### Chosen stack

#### Backend: Rust

The backend should be Rust, using the Axum + Tokio + SQLx stack.

Backend owns:

- HTTP API
- WebSocket board sync
- Postgres access
- AI job dispatch
- connector ingestion API
- board phase/domain invariants

Rust is a strong fit for SpillItOut because the domain has constrained rules:

- phase transitions
- draft card visibility
- vote counts
- one-off clustering
- action confirmation states
- connector payload validation

These should be represented with explicit domain types where practical.

#### Frontend: Next.js

The frontend should be Next.js / React, deployed as a Node.js SSR service.

Frontend owns:

- initial page rendering
- board UI
- interactive client components
- GIF picker UI
- realtime state subscription
- phase transitions and board controls

Next.js fits common hosted Node.js SSR environments and has strong AI/code-generation coverage for the UI patterns SpillItOut needs.

#### Database: Postgres via SQLx

Use Postgres with SQLx.

SQLx gives async Postgres access with compile-time query validation. Use raw SQL and migrations via `sqlx migrate`; do not introduce an ORM for MVP.

#### Realtime: Axum WebSocket

Board sync should run over WebSocket managed by the Rust backend.

Realtime surfaces:

- card updates
- ready state
- vote counts
- phase transitions
- summary/send status

The browser connects directly to the Rust WebSocket endpoint. Next.js should not act as a WebSocket relay for MVP.

### Deployment model

Deploy as two separate services:

1. Rust API/WebSocket service
2. Next.js frontend service

Next.js talks to Rust over internal HTTP. Browser clients connect to the Rust WebSocket endpoint directly.

This avoids conflating frontend hosting with API/WebSocket ownership and keeps frontend and backend hosting concerns separate.

### Rust service ownership implications

The Rust service has different ownership needs than a standard Node.js frontend service.

The team owns:

- Rust Dockerfile
- Datadog tracer wiring
- library upgrades
- CVE patching
- Rust service runtime maintenance

This ownership must be called out in deployment/onboarding documentation.

### Rejected technical directions

- Tauri: desktop app framework, incompatible with a server-authoritative multi-user web tool.
- SvelteKit: no meaningful product advantage over Next.js and less AI prior art for vibe-coded UI implementation.
- Raw static SPA: no gain for this app; higher maintenance friction around hosting, auth, and SSR needs.
- Rust serving the Next.js build: messy build pipeline and conflates frontend hosting with API/WebSocket ownership.
- Board sync in Next.js API routes: WebSocket ownership belongs in Rust.

### Technical architecture principles

The implementation should follow the AI-first architecture guidance:

- configuration as data
- structured outputs from AI and connectors
- constrained domain types for phases, columns, cards, votes, and actions
- clear service boundaries for AI, GIF search, connectors, voting, clustering, and summary
- read-first and user-confirmed flows for AI-generated changes

Additional implementation rules:

- Axum handlers should stay thin.
- Domain logic should live in typed service modules behind trait boundaries.
- Phase state machine should be explicit.
- Connector ingestion should strictly validate payloads at the boundary.
- AI output should always be stored as draft/proposal pending human confirmation.
- Repeated background reclustering must not exist.

### Suggested module boundaries

- `retro-core`: phases, templates, columns, cards, votes, actions
- `board-sync`: Axum WebSocket board state, channels/actor pattern, and presence
- `gif-service`: search, suggestions, attachment metadata
- `ingestion-service`: connector payload validation and placement
- `ai-service`: clustering, tags, actions, summary, mood, GIF suggestions
- `history-service`: completed boards, unresolved actions, recurring tags
- `delivery-service`: Slack/Jira/Linear/export send status
- `companion-clients`: Pi and Claude Code personal AI tooling

### Realtime requirements and scale

For MVP, a single Axum instance with in-memory board fanout is acceptable if all durable state is persisted to Postgres.

The MVP does not require CRDT-level collaborative text editing inside a single card. Card edits can be last-write-wins or locked per editing user as long as conflicts are understandable.

Open scale question: define what traffic/team-count threshold triggers moving board sync to Redis pub/sub or another multi-instance fanout mechanism.

### AI execution model

AI operations should be explicit jobs:

- GIF suggestions for a card
- one-off clustering
- action proposal generation
- summary generation
- team mood proposal
- tag/category generation

Each job should store:

- input summary
- output
- status
- creator/trigger
- timestamp
- error state if failed

This makes AI observable and reviewable instead of magical background behavior.

### Connector API shape

The ingestion API should accept structured JSON payloads with:

- target retro
- target user
- mode: `direct_map` or `user_deck`
- items grouped by `mood`, `wentWell`, `wentWrong`
- source metadata
- idempotency key

The API should validate payloads strictly and reject malformed/oversized input.

### Auth/access model

Preferred first auth model:

- use platform-provided identity headers in the first internal deployment
- no complex role/permission model in MVP
- link-based board access: if you have the retro link, you can view/edit the board

The auth layer must be abstracted cleanly so the initial identity provider can be swapped out when the tool is open-sourced or deployed elsewhere.

### Deployment assumptions

MVP can start as an internally hosted app.

Minimum operational needs:

- authenticated users
- persistent Postgres
- HTTPS
- secret management for GIF/AI/providers
- basic logs/metrics
- backups for retro history

Advanced needs like multi-tenant billing, public marketplace installation, and enterprise policy controls are post-MVP.

### Technical open questions

- GIF provider: Giphy, Tenor, or self-hosted proxy.
- AI provider abstraction: single MVP provider or provider-agnostic interface from day one.
- WebSocket scale threshold for Redis pub/sub or equivalent.
- Exact contract-sharing strategy between Rust and Next.js.

## Data objects for planning

These are not implementation schemas, but they define the minimum concepts implementation planning should account for.

### Retro

- id
- title
- team id
- template id
- phase
- settings
- participants
- created/completed timestamps

### Column

- id
- retro id
- title
- semantic key if known
- order
- reverse order flag

### Card

- id
- retro id
- column id
- author id
- text
- gif attachment
- draft/revealed state
- order
- votes
- tags
- cluster id optional

### Vote

- id
- retro id
- card/cluster id
- participant id
- count

### Action

- id
- retro id
- source card/cluster id
- text
- owner optional
- status
- external link optional

### Ingested item

- id
- user id
- retro id
- mode
- category
- text
- source metadata
- target column optional
- accepted/rejected state

### AI artifact

- id
- retro id
- type
- inputs summary
- output
- status
- created timestamp

## MVP implementation slices

This is the recommended implementation decomposition for the next planning phase.

### Slice 1. Static shell and core data model

- app navigation
- retro list
- create retro
- template/column model
- board render

### Slice 2. Writing and reveal

- draft cards
- private visibility rules
- ready state for writing
- reveal transition

### Slice 3. GIF-first card experience

- GIF search/attach/remove
- card preview
- graceful provider failure

### Slice 4. Voting

- vote settings
- vote casting
- remaining vote count
- voting ready state

### Slice 5. Actions

- top voted agenda
- action draft/confirm/edit/reject
- action cards

### Slice 6. History

- completed board
- previous retros
- unresolved actions

### Slice 7. Connector ingestion

- ingestion API
- direct map mode
- user deck mode
- source metadata
- Pi companion
- Claude Code companion

### Slice 8. Board AI

- clustering
- GIF suggestions
- action proposals
- summary
- mood
- tagging metadata

## MVP release criteria

The MVP is releasable when:

- a team can create and complete a retro end to end
- draft privacy works
- GIF support works reliably enough to be a core feature
- writing/voting ready states work
- voting drives action discussion
- top voted issues produce confirmed actions
- completed retros are preserved
- at least one ingestion path can create private draft/user deck items
- Pi and Claude Code personal AI companion paths are defined, with at least one implemented enough for internal use
- AI features can be disabled without breaking the board
- summary/mood/actions are clearly human-reviewable

## Open decisions for implementation planning

- GIF provider
- first AI provider/model abstraction
- first connector/harness format for Pi and Claude Code companions
- exact contract-sharing strategy between Rust and Next.js
- WebSocket scale threshold for Redis pub/sub or equivalent
- whether tagging is hidden metadata only in MVP or lightly visible in history
- whether uploaded meeting notes are plain text only or support file upload in first version
