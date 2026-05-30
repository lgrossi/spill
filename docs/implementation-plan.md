# Spill. - MVP Implementation Plan

## Status

Plan artifact for the approved MVP product direction.

Source artifacts:

- `docs/product-overview.md`
- `docs/designer-mock-brief.md`
- `docs/mvp-product-doc.md`
- `docs/tech-stack-design.md`
- `mock/index.html`

## Chosen direction

Build Spill. as a board-first collaborative retrospective web app:

- Rust Axum/Tokio/SQLx backend
- Next.js frontend
- Postgres persistence
- Axum WebSocket board sync
- first-party Pi and Claude Code personal AI companions through the ingestion API
- optional board-level AI jobs for GIF suggestions, clustering, action drafts, summary, mood, and tagging

The core board must remain fully usable without AI or integrations.

## Inline structure note

No separate structure artifact exists yet. This plan uses the following implementation structure as the working outline.

Recommended repo shape:

```text
apps/
  web/                  # Next.js frontend
services/
  api/                  # Rust Axum API/WebSocket service
    crates/
      retro-core/       # domain types and invariants
      retro-db/         # SQLx queries and migrations
      retro-api/        # HTTP routes
      retro-sync/       # WebSocket channels/actors
      retro-ai/         # AI job interfaces and runners
      retro-ingest/     # connector payload validation/placement
companions/
  pi/                   # Pi personal AI companion
  claude-code/          # Claude Code personal AI companion
docs/
mock/
```

This can be adjusted during repo bootstrap, but the ownership boundaries should remain:

- domain rules outside handlers
- WebSocket sync in Rust only
- frontend focused on board UX
- AI as explicit jobs/proposals
- connectors writing only to private draft/deck surfaces

## Sequencing strategy

Implement the MVP as vertical slices that preserve a runnable product throughout.

Priority order:

1. foundation and contracts
2. board creation and static board UX
3. writing/reveal collaboration
4. GIF-first cards
5. voting and action discussion
6. history
7. ingestion and companions
8. board AI
9. deployment hardening and acceptance review

AI comes after the board loop works. GIF support comes early because it is core, not polish.

## Task proposal

No task records have been created yet. This plan should be reviewed first, then converted into implementation tasks.

### Epic: Spill. MVP

Deliver a deployable internal MVP where a team can create and complete a retro end to end, with private writing, GIF cards, voting, action discussion, history, connector ingestion, and optional AI helpers.

## Implementation slices

### Slice 1. Repository and runtime foundation

Classification: AFK  
Dependencies: none

Scope:

- initialize monorepo
- add Next.js app shell
- add Rust workspace/service shell
- add Postgres local dev setup
- add formatting/lint/test commands
- add environment/config conventions
- add README with local run instructions

Acceptance criteria:

- `apps/web` starts locally
- `services/api` starts locally
- backend exposes a health endpoint
- local Postgres can be started and migrated
- one command or documented sequence runs the full local stack
- CI-ready commands exist for frontend and backend checks

Verification:

- frontend build/test command
- Rust test command
- SQLx migration check
- manual health endpoint check

Risks:

- repo shape churn
- contract-sharing decision may affect later slices

Out of scope:

- real auth
- production deployment
- board feature implementation

### Slice 2. Domain model and database schema

Classification: AFK  
Dependencies: Slice 1

Scope:

- implement core domain types in Rust
- model retros, participants, columns, cards, votes, actions, ingested items, AI artifacts
- define phase enum and valid transitions
- add SQLx migrations
- add repository/query layer

Acceptance criteria:

- invalid phase transitions are rejected
- draft/revealed card state is represented explicitly
- vote limits can be represented per retro
- per-column reverse order is represented
- AI artifacts and ingested items have persistence tables

Verification:

- Rust unit tests for phase transitions
- SQLx compile/check for queries
- migration up/down or reset test

Risks:

- over-modeling before UX is exercised

Out of scope:

- realtime sync
- AI execution
- connector clients

### Slice 3. API shell, auth abstraction, and contract strategy

Classification: AFK  
Dependencies: Slice 2

Scope:

- add Axum HTTP route structure
- add platform-header identity abstraction for first internal deployment
- add invite-based retro access model
- define API contract strategy between Rust and Next.js
- add basic error response format

Acceptance criteria:

- frontend can load current user identity through backend/API path
- retro access can be checked from email grants
- routes have structured errors
- contract-sharing approach is documented and used by at least one endpoint

Verification:

- API integration tests
- contract generation/typecheck where applicable
- frontend call to API in local dev

Risks:

- contract-sharing choice can slow iteration if overbuilt

Out of scope:

- granular role model
- public OAuth flow

### Slice 4. Retro overview and create retro

Classification: AFK  
Dependencies: Slice 3

Scope:

- implement overview/history entry page
- create standard retro template
- create custom template skeleton
- persist retro, columns, participants, settings
- show active and completed retros

Acceptance criteria:

- user can create a standard retro with Mood, Went well, Went wrong, Actions
- user can create a custom retro with arbitrary column names
- newly created retro opens in Writing phase
- overview lists active and completed retros

Verification:

- frontend component tests for create form
- backend integration tests for create/list
- manual local create/open flow

Risks:

- custom templates can expand scope

Out of scope:

- template marketplace
- advanced history analytics

### Slice 5. Board shell from approved mock

Classification: HITL/review-gated  
Dependencies: Slice 4

Scope:

- translate `mock/index.html` into the initial Next.js board shell
- preserve single-board-view principle
- implement phase header, columns, cards, side/bottom trays, and inline notes where useful
- support responsive-enough desktop layout

Acceptance criteria:

- board view visually matches the reviewed mock closely enough for MVP
- writing, discussion, voting, actions, and completed states are represented as states of the same board view
- no core retro phase navigates to a separate page

Verification:

- screenshot/Plannotator review
- frontend build
- manual navigation through mock-backed states

Risks:

- over-polishing UI before behavior exists

Out of scope:

- final brand identity
- mobile-first layout

### Slice 6. Writing phase, draft privacy, and reveal

Classification: AFK  
Dependencies: Slice 5

Scope:

- create/edit/delete own draft cards
- hide/blur other users' draft cards before reveal
- writing ready state
- host/team reveal transition
- persist revealed state

Acceptance criteria:

- user can read/edit own draft cards
- user cannot read others' drafts before reveal
- ready progress updates
- reveal makes all cards visible
- Discussion phase has no ready button

Verification:

- backend visibility tests
- frontend interaction tests
- multi-user manual test with two browser sessions

Risks:

- privacy leaks in API responses if filtering is done only on frontend

Out of scope:

- realtime sync polish
- connector-created drafts

### Slice 7. WebSocket board sync

Classification: AFK  
Dependencies: Slice 6

Scope:

- Axum WebSocket endpoint
- board subscription model
- event fanout for card changes, ready state, phase transitions
- reconnect/resync behavior
- durable state remains in Postgres

Acceptance criteria:

- card changes appear in another browser without refresh
- ready state updates live
- phase transition updates live
- reconnect reloads current board state

Verification:

- backend WebSocket integration tests
- manual two-browser test
- basic reconnect test

Risks:

- shared mutable state bugs

Out of scope:

- multi-instance Redis/pubsub
- CRDT editing inside a card

### Slice 8. GIF-first card experience

Classification: AFK  
Dependencies: Slice 6

Scope:

- GIF provider integration behind service interface
- search GIFs from card composer/editor
- attach/replace/remove GIF
- render GIF preview on card
- graceful provider failure

Acceptance criteria:

- user can search and attach a GIF to a card
- attached GIF appears on card after save
- user can remove/replace GIF
- card text flow works if GIF provider fails

Verification:

- frontend interaction tests with mocked provider
- backend provider/service tests
- manual GIF search/attach flow

Risks:

- provider API limits or content filtering

Out of scope:

- AI GIF suggestions
- self-hosted GIF cache unless needed

### Slice 9. Voting phase

Classification: AFK  
Dependencies: Slices 6, 7

Scope:

- start Voting phase
- default 3 votes per participant
- configurable votes per retro/template
- vote target cards or clusters
- remaining vote count
- voting ready state
- freeze clustering once voting begins

Acceptance criteria:

- default retro gives 3 votes per participant
- vote count decreases when voting
- no extra votes can be cast
- voting ready progress updates
- votes update live across sessions

Verification:

- vote-limit backend tests
- frontend voting tests
- two-browser manual voting flow

Risks:

- race conditions around vote count

Out of scope:

- anonymous voting modes
- complex vote weighting

### Slice 10. One-off clustering

Classification: AFK  
Dependencies: Slice 9 can be parallel after Slice 6 if AI is mocked

Scope:

- one-off Cluster-fy operation
- manual clustering mode
- auto-on-vote-start mode
- cluster persistence
- preserve originals
- prevent repeated clustering
- store tags/categories metadata

Acceptance criteria:

- manual Cluster-fy runs once
- auto clustering runs once when voting starts if configured
- manual and auto modes cannot both run in the same retro
- cluster state is labeled
- original cards remain recoverable/inspectable

Verification:

- backend clustering state tests
- frontend cluster state tests
- manual cluster then vote flow

Risks:

- AI unavailable; use deterministic/mock clustering first

Out of scope:

- continuous background clustering
- complex cluster editor

### Slice 11. Action discussion and action cards

Classification: AFK  
Dependencies: Slice 9

Scope:

- compute top voted pain points
- configurable top N, default 3
- handle ties around cutoff
- create editable action drafts
- confirm/reject/edit actions
- store action status and optional owner/link

Acceptance criteria:

- Action Discussion shows top voted items
- AI/manual action drafts are not final by default
- confirmed actions appear in Actions column
- rejected drafts do not become actions
- tie around top N can be resolved by the team

Verification:

- backend top-vote selection tests
- frontend action draft tests
- manual action discussion flow

Risks:

- unclear UX for ties; keep simple

Out of scope:

- full external issue sync

### Slice 12. Completed board and history

Classification: AFK  
Dependencies: Slice 11

Scope:

- complete retro
- read-only/reloaded completed board
- summary placeholder/manual summary if AI disabled
- previous retros list
- unresolved actions list
- recurring tag foundation

Acceptance criteria:

- completed retro appears in history
- past retro opens as completed board
- actions and votes are visible
- unresolved actions are visible
- tags can be stored for future recurrence

Verification:

- history API tests
- frontend history tests
- manual complete/reopen flow

Risks:

- recurrence UI can become analytics-heavy

Out of scope:

- dashboards
- trend charts

### Slice 13. Connector ingestion API and user deck

Classification: AFK  
Dependencies: Slices 6, 8

Scope:

- HTTP ingestion API
- strict JSON validation
- idempotency key
- source metadata
- direct map mode
- user deck mode
- private placement into draft/deck
- deck UI placement into board columns

Acceptance criteria:

- direct-map payload creates private draft cards in standard columns
- user-deck payload creates private deck items
- other users cannot read ingested content before reveal
- malformed/oversized payloads are rejected
- source category/time window is inspectable by receiving user

Verification:

- API contract tests
- privacy tests
- frontend deck tests
- manual ingest-to-board flow

Risks:

- connector trust and auth needs may grow quickly

Out of scope:

- connector marketplace
- public card creation by connectors

### Slice 14. First-party Pi and Claude Code companions

Classification: AFK  
Dependencies: Slice 13

Scope:

- Pi companion command/skill
- Claude Code companion command/skill
- default personal retro prompt
- local/session context opt-in
- review/edit/reject before send
- send approved mood/wentWell/wentWrong payloads

Acceptance criteria:

- Pi companion can generate reviewable mood/win/pain suggestions
- Claude Code companion can generate reviewable mood/win/pain suggestions
- user approval is required before sending
- approved suggestions appear as private drafts or deck items
- rejected suggestions are not sent

Verification:

- companion dry-run test with fixture session logs
- ingestion API integration test
- manual companion-to-board flow

Risks:

- session-log parsing differs by environment
- privacy expectations are high

Out of scope:

- always-on personal AI inside web app
- third-party connector ecosystem

### Slice 15. Board AI jobs

Classification: AFK  
Dependencies: Slices 10, 11, 12

Scope:

- AI job table/status model
- GIF suggestion job
- clustering job interface
- action proposal job
- summary job
- team mood job
- tag/category job
- visible failed/retry states

Acceptance criteria:

- AI jobs are explicit persisted records
- outputs are proposals/drafts, not final
- summary works from board data
- summary can use uploaded meeting notes when provided
- team mood uses quietly-proud/smooth-sailing/good-sparks/productive-chaos/foggy/spicy/stuck-in-mud/needs-a-map labels
- AI features can be disabled without breaking the board

Verification:

- job state tests
- provider abstraction tests with fake provider
- manual AI-disabled board completion
- manual AI-enabled summary/mood flow

Risks:

- model/provider output quality
- scope creep into autonomous facilitation

Out of scope:

- invisible background interpretation
- per-person mood scoring

### Slice 16. Upload meeting notes for summary/mood context

Classification: AFK  
Dependencies: Slice 15

Scope:

- upload/paste meeting notes text
- attach notes to retro
- include notes as optional AI context for summary and mood
- avoid turning summary into full meeting notes

Acceptance criteria:

- user can add notes before completing retro
- summary/mood jobs can include notes
- retro can complete without notes
- notes are visible/auditable to participants with board access

Verification:

- upload/paste tests
- AI job input tests
- manual complete with/without notes

Risks:

- privacy expectations around meeting notes

Out of scope:

- live transcription
- external notes integrations

### Slice 17. Delivery/export follow-through

Classification: AFK  
Dependencies: Slice 12

Scope:

- copy/export completed summary
- Slack-style webhook/export abstraction
- external action link placeholder
- delivery status and retry

Acceptance criteria:

- completed summary can be copied/exported
- delivery success is visible
- delivery failure is visible with retry/reconnect path
- action cards can store external links

Verification:

- delivery service tests with fake target
- frontend send-status tests
- manual send success/failure flow

Risks:

- real third-party APIs can balloon scope

Out of scope:

- full Jira/Linear/Slack product integration parity

### Slice 18. Deployment and operational hardening

Classification: AFK  
Dependencies: Slices 1-17 as needed

Scope:

- production-ish service configs
- Rust and Next.js Dockerfiles
- database migration execution
- secrets/config wiring
- logs/metrics basics
- backup note for Postgres
- deployment/onboarding ownership note

Acceptance criteria:

- frontend service deploys
- backend service deploys
- database migrations run
- health checks exist
- logs expose startup/errors
- Rust service ownership is documented

Verification:

- deployment dry run
- health endpoint check
- migration check
- basic smoke test against deployed environment

Risks:

- Rust service hosting ownership and maintenance

Out of scope:

- multi-tenant SaaS ops
- billing

### Slice 19. End-to-end product acceptance

Classification: HITL/review-gated  
Dependencies: Slices 1-18

Scope:

- run a complete retro with at least two users/sessions
- compare against approved mock/product docs
- validate no internal names leak into repo/docs/UI
- validate no AI/integration dependency blocks core board
- collect final gaps before MVP implementation close

Acceptance criteria:

- team can create a retro
- team can write privately, reveal, discuss, vote, discuss actions, complete
- GIFs work
- history works
- ingestion path works
- at least one companion path works enough for internal use
- AI can be disabled and the retro still completes
- Plannotator/manual review accepts the MVP flow

Verification:

- manual QA script
- screenshots/video or Plannotator review
- automated test suite passing
- repo leak check for internal platform/company names

Risks:

- late UX issues around single-board flow

Out of scope:

- public launch readiness

## Cross-cutting verification strategy

Automated:

- Rust unit tests for domain rules
- Rust integration tests for API/database
- SQLx query checks
- frontend component/interaction tests
- API contract tests
- companion fixture tests

Manual:

- two-browser collaborative board test
- GIF search/attach test
- complete retro smoke test
- Plannotator review for board UX parity
- no-internal-name repo scan

Suggested leak check:

```bash
rg -n -f /path/to/local-forbidden-terms .
```

## Parallelization strategy

The first implementation pass should use many agents, but only after the shared repo skeleton exists. Without worktree separation, agents must avoid editing the same paths at the same time.

### Coordination rules

- One agent owns one slice/path group at a time.
- No two agents edit the same package, migration file, or generated contract at once.
- Shared contracts are stabilized before dependent frontend/backend work fans out.
- Broad formatting, dependency upgrades, and generated-code refreshes are serialized.
- Every agent starts by reading this plan plus the relevant product docs.
- Every agent reports touched paths and verification commands.

### Wave 0: serial bootstrap

Run one agent only.

Task:

- Slice 1: Repository and runtime foundation

Reason:

- creates shared repo shape
- establishes commands
- prevents every other agent from fighting over package manifests, workspace config, and local dev setup

### Wave 1: maximum safe fan-out after bootstrap

Can run in parallel without worktree separation if path ownership is respected.

| Agent lane | Owns | Slices | Notes |
| --- | --- | --- | --- |
| Backend domain/db | `services/api/crates/retro-core`, `services/api/crates/retro-db`, migrations | Slice 2 | Owns schema and core invariants. Avoid frontend edits. |
| Frontend shell | `apps/web` board shell/components/styles | Slice 5 | Can build against mock/stub data while backend domain work happens. |
| API/auth/contracts | `services/api/crates/retro-api`, contract docs/generated types | Slice 3 | Should coordinate with backend domain/db before finalizing endpoint shapes. |
| Product QA/docs | `docs`, review scripts, leak checks | Slice 19 prep | Can prepare manual QA checklist and review harness without touching app code. |

Recommended concurrency: 3-4 agents.

### Wave 2: collaboration loop

Start after Slices 2, 3, and the initial board shell are stable.

| Agent lane | Owns | Slices | Notes |
| --- | --- | --- | --- |
| Writing/reveal | board card APIs plus frontend writing/reveal UI | Slice 6 | Needs backend visibility tests. |
| WebSocket sync | `retro-sync` and frontend subscription hook | Slice 7 | Should not modify card domain rules except through agreed interfaces. |
| GIF experience | `gif-service` plus card GIF UI | Slice 8 | Can use fake provider first. |
| Overview/create | retro list/create UI and API | Slice 4 | Can complete if not already covered by shell work. |

Recommended concurrency: 3-4 agents.

### Wave 3: prioritization and follow-through

Start after writing/reveal and basic sync work.

| Agent lane | Owns | Slices | Notes |
| --- | --- | --- | --- |
| Voting | vote APIs/domain/UI | Slice 9 | Must coordinate with WebSocket lane for live vote events. |
| Clustering | cluster model/job/UI state | Slice 10 | Can start with deterministic/mock clustering. |
| Actions | top-voted agenda and action cards | Slice 11 | Depends on voting selection shape. |
| History | completed board/history/unresolved actions | Slice 12 | Depends on stable action/card models. |

Recommended concurrency: 3 agents initially; add history once actions shape stabilizes.

### Wave 4: ingestion, companions, AI

Start after draft/deck/card placement and action/history models are stable.

| Agent lane | Owns | Slices | Notes |
| --- | --- | --- | --- |
| Ingestion/deck | ingestion API, validation, user deck UI | Slice 13 | Must preserve private-by-default behavior. |
| Companions | `companions/pi`, `companions/claude-code` | Slice 14 | Can work mostly independently against ingestion contract fixtures. |
| Board AI jobs | `retro-ai`, job persistence, fake provider | Slice 15 | Should use explicit job records. |
| Notes/delivery | notes context and export/delivery abstraction | Slices 16-17 | Can start after completed board exists. |

Recommended concurrency: 4 agents.

### Wave 5: hardening and acceptance

Mostly serial/review-gated.

Tasks:

- Slice 18: Deployment and operational hardening
- Slice 19: End-to-end product acceptance

Reason:

- touches repo-wide config, service ownership docs, deployment, and product acceptance
- should happen after feature slices converge

### Practical max concurrency

Without worktree separation:

- safe early max: 3-4 agents
- safe mid-project max: 4 agents
- risky above 5 unless agents are strictly path-isolated

With no worktree separation, "more agents" only helps when ownership boundaries are crisp. The best initial split is:

1. backend domain/db
2. frontend board shell
3. API/auth/contracts
4. docs/QA harness

After that, expand to GIF, sync, voting, ingestion, companions, and AI lanes.

## Main risks

### Privacy leakage

Draft card and connector content must be filtered server-side before reveal.

Mitigation:

- write backend visibility tests early
- do not rely on frontend blur alone

### Realtime race conditions

Votes, ready states, and phase transitions are collaborative and can conflict.

Mitigation:

- keep durable state in Postgres
- treat WebSocket as sync/fanout, not source of truth
- test concurrent vote limits

### AI scope creep

AI can easily become a facilitator or surveillance layer.

Mitigation:

- AI jobs are explicit and user-triggered
- outputs are proposals/drafts
- no per-person scoring
- no autonomous public card creation

### Stack ownership

Rust backend ownership requires deliberate maintenance.

Mitigation:

- document ownership during deployment setup
- keep Axum handlers thin
- isolate domain logic and provider integrations

## Implementation planning output

Recommended created task set after review:

- Epic: Spill. MVP
- 19 vertical slice tasks matching the slices above
- HITL gates:
  - Slice 5 board shell review
  - Slice 19 end-to-end product acceptance

Do not create implementation tasks until this plan is reviewed and accepted.
