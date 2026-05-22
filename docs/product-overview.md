# SpillItOut - Product Overview

## Product summary

SpillItOut is a cooperative retrospective tool for teams that want a simple board first, with optional AI assistance layered on top.

It is not an "AI writes your retro for you" product. It is a board for writing, discussing, voting, deciding actions, and closing the loop, with AI used to help teams organize, summarize, and follow through.

## Product goals

- Make retros easy to run remotely and asynchronously.
- Preserve what made retro boards useful and expressive, with GIF support treated as a core product feature, not a nice-to-have.
- Keep previous retro boards and expose recurring themes over time.
- Support optional AI without making the core board dependent on it.
- Turn voted pain points into discussed, human-approved actions.

## Product principles

### 1. Board-first
The product must work well with zero AI and zero integrations.

### 2. Safe place to share
The board should support blunt, factual, human sharing of wins, pains, and mood without over-coaching the team.

### 3. AI curates, humans decide
AI can suggest, cluster, summarize, and pre-propose actions, but final board content and final actions remain human-controlled.

### 4. Team-relevant, not nitpicky
AI should surface work situations, project friction, recurring patterns, and meaningful individual stories worth sharing. It should not shame or micro-analyze tool usage or personal productivity details.

### 5. Privacy and trust by default
All AI and connector use must be opt-in, visible, attributable, and disableable.

## Core user experience

### Standard retro flow

1. **Writing** - async  
   The board opens in draft mode. Users write cards privately. Card contents are blurred from others until everyone is ready.

2. **Discussing** - sync  
   The team discusses the revealed cards. The board is mostly a shared reference surface.

3. **Voting** - async  
   Voting opens. Users cast votes and mark ready when done.

4. **Discussing actions** - sync  
   The team discusses the top voted issues one by one. AI may pre-propose action candidates, but actions must be discussed and confirmed by humans.

5. **Summarizing** - end  
   AI generates a concise retro summary, a lightweight team mood, and sends outputs to configured destinations.

## Default board model

Default standard retro template:

- Mood
- Went well
- Went wrong
- Actions

The product must also support custom retro shapes with additional or different columns.

## Locked product decisions

### Draft mode first
Boards always start in private draft mode. Others cannot read your cards until the writing step is complete.

### Ready state only in async phases
Ready flags exist only for:

- Writing
- Voting

No ready state for discussing, action discussion, or summary.

### Votes
- Default: 3 votes per person
- Configurable per board/template
- Product should allow configuration for whether multiple votes can land on the same card

### Actions come from voted priorities
- Actions are always discussed from the top voted pain points
- Default should prioritize top 3 voted items
- Configurable top N
- AI may pre-populate action candidates for those items, especially from "Went wrong"

### Clustering
- Clustering is not a formal team phase
- It happens either:
  - on demand via a one-off "Cluster-fy" action, or
  - automatically once when voting starts
- Never both in the same retro run
- No repeated reclustering loops

### GIF support
GIFs are first-class. The product should support:

- full search
- AI-assisted suggestions
- suggestions by multiple lenses, not a single narrow result set

AI suggestions accelerate expression but do not constrain it. This is a core part of the product experience.

## Personal AI and app AI split

### Personal AI
Personal AI is not a core in-app authoring feature in MVP. It should be supported through connectors feeding the retro product.

Its job is to help a user prepare:

- mood
- wins / cheers
- pains

The user reviews the suggestions and sends approved items to the board.

Current preferred MVP shape:

- a connector or skill, likely harness-based for early use cases
- user-approved suggestions only
- output mapped to the ingest contract for private board drafts or user deck placement

The product should not require a built-in always-on personal AI workflow in MVP.

### App AI
App AI operates at board level:

- GIF suggestions for cards
- clustering suggestions
- action proposals for top voted issues
- categorization / tagging of themes for later history and metrics
- team mood proposal
- retro summary

## Connector ingestion model

One ingestion contract is enough.

Input categories:

- mood
- wentWell
- wentWrong

Two ingestion modes:

### 1. Direct column map
For standard retros. Approved items land directly in the user's private draft cards in the mapped columns.

### 2. User deck / backlog
For custom retro templates. Approved items land in a private user-only deck so the user can place them into the right columns manually.

### Connector rule
Connectors should not write public board cards directly by default. They write to the user's private draft space or user-only deck.

## AI source boundaries

Likely sources include:

- AI session JSONL logs
- Jira / Linear
- Slack
- calendars / agendas
- incidents / deploy logs

These sources are valuable because they can expose relevant work situations and project-level patterns.

The AI should surface:

- project friction
- blocked decisions
- coordination problems
- recurring pain
- notable individual wins worth sharing
- notable individual friction worth support and discussion

The AI should not surface:

- shell command counts
- ticket churn as a personal metric
- time spent judgments
- productivity scoring

## Team mood

The product proposes a lightweight, playful team mood at the end of the retro.

Current preferred scale:

- Stormy
- Off
- Mixed
- Steady
- Flying

The mood is:

- team-level only
- non-diagnostic
- explained briefly
- optional

Later versions may improve mood generation with meeting notes in addition to cards and votes.

## Retro memory

Tracking previous retro boards is non-negotiable.

The product must support:

- previous retro history
- actions from previous retros
- recurring themes over time
- visibility into unresolved recurring pain

This should grow into retro memory, not just archive storage.

## Non-goals for MVP

- fully autonomous retro generation
- AI-led conversational facilitation
- employee analytics or performance scoring
- mandatory integrations
- replacing Jira, Slack, or meeting notes tools

## MVP feature set

### Core board
- create retro from template
- board list and past retros
- standard board with columns
- custom board support
- private draft mode
- reveal after writing ready state
- card creation / edit / delete
- drag and drop within columns
- optional reverse order per column for discussion flow
- GIF attach/search

### Flow
- writing phase
- discussion phase
- voting phase
- action discussion phase
- summary phase
- ready tracking for writing and voting

### AI
- GIF suggestions
- one-off clustering
- action suggestions for top voted issues
- end-of-retro summary
- team mood proposal

### History and follow-through
- previous boards
- action tracking
- recurring action/theme visibility

### Connectors
- ingestion API
- direct map mode
- user deck mode

## Open questions

- exact retro history views and recurrence visualizations
- whether app AI tagging is user-visible in MVP or stored as metadata first
- how strict the first connector contract should be for harness-based personal AI
- what uploaded meeting notes format is accepted in the first notes-aware version
- whether meeting notes are captured in-product, uploaded, or connector-fed in later versions
