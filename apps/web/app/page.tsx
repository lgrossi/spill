"use client";

import { useState, type ReactNode } from "react";

type View = "overview" | "create" | "board" | "history";
type Phase = "writing" | "deck" | "discussion" | "clustered" | "voting" | "actions" | "completed";
type Card = { title: string; text?: string; gif?: string; hidden?: boolean; merged?: string[]; action?: boolean; summary?: boolean; vote?: string };
type Column = { title: string; hint: string; count: string; cards: Card[] };

const views: { id: View; label: string }[] = [
  { id: "overview", label: "Overview" },
  { id: "create", label: "Create" },
  { id: "board", label: "Board" },
  { id: "history", label: "History" },
];

const phases: { id: Phase; label: string }[] = [
  { id: "writing", label: "Private writing" },
  { id: "deck", label: "Custom + deck" },
  { id: "discussion", label: "Reveal / discuss" },
  { id: "clustered", label: "Clustered once" },
  { id: "voting", label: "Voting" },
  { id: "actions", label: "Actions" },
  { id: "completed", label: "Completed" },
];

const meta: Record<Phase, { title: string; kicker: string; phase: string; guard: string; note: string; actions: string[]; progress?: string; width?: string }> = {
  writing: { title: "Board - private writing", kicker: "Writing", phase: "Writing", guard: "Reveal locked", note: "5 of 6 ready. Your drafts are readable; everyone else stays blurred.", actions: ["Pending actions: 2", "Imported drafts: 3", "Mark ready"], progress: "Ready only exists in Writing and Voting", width: "83%" },
  deck: { title: "Custom board with bottom deck", kicker: "User deck mode", phase: "Writing", guard: "3/5 ready", note: "Approved suggestions sit in My deck and can be played into columns without navigation.", actions: ["Mark ready"], progress: "My deck is private to you", width: "60%" },
  discussion: { title: "Reveal and discussion", kicker: "Discussing", phase: "Revealed", guard: "No ready button", note: "The same board becomes the shared reference surface. Cluster-fy is available once.", actions: ["Cluster-fy once", "Start voting"] },
  clustered: { title: "Cluster-fy as board mutation", kicker: "Clustered once", phase: "Clustered once", guard: "Manual one-off", note: "Similar cards merge in place. No background reclustering.", actions: ["Explode clusters", "Start voting"] },
  voting: { title: "Voting on the board", kicker: "Voting", phase: "Voting", guard: "Votes left: 1/3", note: "Clustering is frozen. Voting is async, so ready is back for this phase only.", actions: ["Mark voting ready"], progress: "4 of 6 finished voting", width: "67%" },
  actions: { title: "Actions happen in the Actions column", kicker: "Action discussion", phase: "Discuss actions", guard: "No ready state", note: "Top voted problems move into focus with editable action drafts. AI proposes; humans decide.", actions: ["Complete retro"] },
  completed: { title: "Completed board is the summary", kicker: "Completed", phase: "Completed", guard: "Tags generated", note: "Completion adds digest and delivery chips to the same board.", actions: ["Copy board digest", "Close"] },
};

const seed = {
  mood: [{ title: "Mixed 😵‍💫", text: "Proud of launch, but hotfix churn made the week heavier." }, { title: "Steady 🛶", text: "Prep helped even when things got busy." }],
  good: [{ title: "Launch pairing 🎉", text: "Support and product shared context.", gif: "GIF: relaxed mission-control high-five" }, { title: "Demo rehearsal", text: "Questions were sharper and shorter." }],
  bad: [{ title: "Hotfix branches", text: "Three branches split review context." }, { title: "Alert noise", text: "Deploy alerts buried the current decision thread." }],
};

const deckColumns: Column[] = [
  { title: "Kudos", hint: "Private drafts", count: "2", cards: [{ title: "QA pairing 🎉", text: "Demo rehearsal feedback landed early." }, { title: "Private draft", hidden: true }] },
  { title: "Friction", hint: "Private drafts", count: "3", cards: [{ title: "Channel sprawl 😬", text: "Decisions split across release threads." }] },
  { title: "Ideas", hint: "Play from deck", count: "1", cards: [{ title: "One launch note", text: "Keep one pinned current-plan note." }] },
  { title: "Questions", hint: "Play from deck", count: "1", cards: [{ title: "Support timing?", text: "Should sign-off move before deploy starts?" }] },
  { title: "Actions", hint: "Later", count: "0", cards: [{ title: "Actions can wait", text: "Votes create priorities before commitments." }] },
];

export default function Home() {
  const [view, setView] = useState<View>("overview");
  const [phase, setPhase] = useState<Phase>("writing");

  return (
    <main>
      <header className="topbar">
        <div>
          <p className="eyebrow">SpillItOut</p>
          <h1>Board-first retro table</h1>
          <p>Private writing, GIF-native cards, one-off clustering, voting, and actions stay anchored to one board.</p>
        </div>
        <nav className="tabs" aria-label="Shell views">
          {views.map((item) => <button key={item.id} aria-pressed={view === item.id} onClick={() => setView(item.id)}>{item.label}</button>)}
        </nav>
      </header>
      <div className="page">
        <aside className="rail">
          <Panel><h2>Review map</h2><p className="muted">Overview/history are entry points. Core retro phases below are board states, not pages.</p><Chips values={["GIFs are core", "Humans decide", "Board states"]} /></Panel>
          {view === "board" ? <Panel><h2>Board states</h2><div className="state-list">{phases.map((item) => <button key={item.id} aria-pressed={phase === item.id} onClick={() => setPhase(item.id)}>{item.label}</button>)}</div></Panel> : null}
        </aside>
        <section className="content">
          {view === "overview" && <Overview open={() => setView("board")} />}
          {view === "create" && <Create open={() => setView("board")} />}
          {view === "board" && <Board phase={phase} />}
          {view === "history" && <History open={() => setView("board")} />}
        </section>
      </div>
    </main>
  );
}

function Overview({ open }: { open: () => void }) {
  return (
    <Scene kicker="Entry point" title="Retro table" note="A soft landing for active and past boards.">
      <div className="grid three">
        <Panel tape><h3>Nothing spilled yet</h3><div className="empty"><strong>No retro boards yet</strong><p className="muted">Start simple; add GIFs, votes, and actions later.</p><button className="primary">Create first retro</button></div></Panel>
        <Panel tape><h3>On the table now</h3><button className="list-card plain" onClick={open}><span className="row"><strong>Sprint 42 retro</strong><span className="chip blue">Writing</span></span><p className="muted">Private drafts. Reveal is still locked.</p><Chips values={["5/6 ready", "3 votes/person", "Top 3 actions"]} /></button></Panel>
        <Panel tape><h3>Earlier spills</h3><Item title="Sprint 41" text="Tags: release-coordination, alert-noise" /><Item title="Release 7.3 incident" text="1 pending action still open." /><input aria-label="Search past boards" defaultValue="deploy" /></Panel>
      </div>
    </Scene>
  );
}

function Create({ open }: { open: () => void }) {
  return (
    <Scene kicker="Template + defaults" title="Set the table" note="Pick prompts and a few house rules without turning setup into admin work.">
      <div className="grid two">
        <Panel tape><h3>Pick the prompts</h3><Item title="Standard retro" text="Mood / Went well / Went wrong / Actions" chip="Selected" /><Item title="Custom board" text="Kudos / Friction / Ideas / Questions / Actions." chip="Deck friendly" /></Panel>
        <Panel tape><h3>Light house rules</h3><div className="grid three"><label>Votes/person<input defaultValue="3" /></label><label>Top action cards<input defaultValue="3" /></label><label>Stack votes?<select defaultValue="Allowed"><option>Allowed</option></select></label></div><div className="grid two"><Item title="Cluster-fy" text="Manual one-off, or auto once when voting starts. Never loops." /><Item title="External ingest" text="Approved cards land in private drafts or My deck." /></div><button className="primary" onClick={open}>Open board shell</button></Panel>
      </div>
    </Scene>
  );
}

function Board({ phase }: { phase: Phase }) {
  const m = meta[phase];
  const custom = phase === "deck";

  return (
    <Scene kicker={m.kicker} title={m.title} note={m.note}>
      <article className="board">
        <div className="board-head"><div><Chips values={[m.phase, "Sprint 42", m.guard]} colorFirst /><h3>{custom ? "Weekly team pulse" : "Product Platform retro"}</h3><p className="muted">{m.note}</p></div><div className="actions">{m.actions.map((action) => <button key={action} className={/Mark|Start|Complete|Close/.test(action) ? "primary" : ""}>{action}</button>)}</div></div>
        {m.progress && <div className="phase-strip"><strong>{m.progress}</strong><div className="progress"><span style={{ width: m.width }} /></div></div>}
        <div className={`columns ${custom ? "five" : ""}`}>{(custom ? deckColumns : columnsFor(phase)).map((column) => <ColumnView key={column.title} column={column} />)}</div>
        {custom && <Deck />}
        {phase === "writing" && <div className="phase-strip"><strong>All ready, reveal still deliberate</strong><p className="muted">Even at 6/6 ready, cards stay blurred until the facilitator clicks Reveal board.</p><button className="primary">Reveal board</button></div>}
        {phase === "completed" && <div className="phase-strip"><strong>Delivery</strong><Chips values={["Comms sent", "Linear reconnect needed"]} /></div>}
      </article>
    </Scene>
  );
}

function History({ open }: { open: () => void }) {
  return (
    <Scene kicker="History entry point" title="Memory without analytics cosplay" note="Past boards reload as read-only boards with practical tags and unresolved actions.">
      <div className="grid four"><Panel tape><h3>Boards</h3>{["Sprint 42 - Steady", "Sprint 41 - Mixed", "Release 7.3 - Incident"].map((item) => <div className="list-card" key={item}>{item}</div>)}<button onClick={open}>Open Sprint 41 board</button></Panel><Panel tape><h3>Tags</h3><Chips values={["release-coordination x3", "alert-noise x2", "support-signoff x2"]} /></Panel><Panel tape><h3>Pending actions</h3><Item title="Route deploy noise" text="Shows as a chip before the next retro." /></Panel><Panel tape><h3>Repeated pain</h3><p className="muted">Context fragmentation keeps appearing as branch sprawl, thread sprawl, and unclear ownership.</p></Panel></div>
    </Scene>
  );
}

function Scene({ kicker, title, note, children }: { kicker: string; title: string; note: string; children: ReactNode }) {
  return <section className="scene"><div className="scene-head"><p className="eyebrow">{kicker}</p><h2>{title}</h2><p>{note}</p></div>{children}</section>;
}

function Panel({ children, tape }: { children: ReactNode; tape?: boolean }) {
  return <article className={`panel ${tape ? "tape" : ""}`}>{children}</article>;
}

function Chips({ values, colorFirst }: { values: string[]; colorFirst?: boolean }) {
  return <div className="chips">{values.map((value, index) => <span className={colorFirst && index === 0 ? "chip blue" : "chip"} key={value}>{value}</span>)}</div>;
}

function Item({ title, text, chip }: { title: string; text: string; chip?: string }) {
  return <div className="list-card"><span className="row"><strong>{title}</strong>{chip && <span className="chip blue">{chip}</span>}</span><p className="muted">{text}</p></div>;
}

function ColumnView({ column }: { column: Column }) {
  return <section className="column"><div className="column-head"><div><h4>{column.title}</h4><small>{column.hint}</small></div><span className="chip">{column.count}</span></div>{column.cards.map((card) => <Sticky key={card.title} card={card} />)}</section>;
}

function Sticky({ card }: { card: Card }) {
  return <article className={`card ${card.hidden ? "hidden" : ""} ${card.action ? "action" : ""} ${card.summary ? "summary" : ""}`}><span className="row"><strong>{card.title}</strong>{card.vote && <span className="vote">{card.vote}</span>}</span>{card.text && <p>{card.text}</p>}{card.hidden && <><span className="skeleton" /><span className="skeleton short" /></>}{card.gif && <div className="gif">{card.gif}</div>}{card.merged?.map((text) => <p className="merged" key={text}>{text}</p>)}</article>;
}

function Deck() {
  return <div className="deck" aria-label="My deck"><button>‹</button>{["QA caught empty state 🎯", "Slack thread sprawl 🧵", "Earlier sign-off? 🤔", "Pinned launch note 📌"].map((card) => <div className="hand-card" key={card}>{card}</div>)}<button>›</button><span>⌄</span></div>;
}

function columnsFor(phase: Phase): Column[] {
  if (phase === "writing") return [{ title: "How do you feel?", hint: "Private until reveal", count: "2", cards: [seed.mood[0], { title: "Someone else", hidden: true }] }, { title: "What went well?", hint: "Your cards + hidden team drafts", count: "7", cards: [seed.good[0], { title: "Private draft", hidden: true }] }, { title: "What went wrong?", hint: "Likely vote targets later", count: "7", cards: [seed.bad[0], { title: "Private draft", hidden: true }] }, { title: "Actions", hint: "Can stay empty until voting", count: "1", cards: [{ title: "Maybe", text: "Try a release lead for launch week." }] }];
  if (phase === "clustered" || phase === "voting") return [{ title: "How do you feel?", hint: "Revealed", count: "6", cards: [{ title: "Mixed but steady", text: "Launch was stressful but contained." }] }, { title: "What went well?", hint: "Clustered", count: "5", cards: [{ title: "Cross-team prep worked", merged: ["Support joined launch room early.", "Demo rehearsal made the walkthrough sharper."], vote: phase === "voting" ? "+" : undefined }] }, { title: "What went wrong?", hint: phase === "voting" ? "Main vote area" : "Clustered", count: "4", cards: [{ title: "Release coordination got scattered", merged: ["Hotfix branches split review context.", "Slack threads split the current plan."], vote: phase === "voting" ? "+ · you placed 2" : undefined }, { title: "Alert noise", text: "Deploy alerts buried the current decision thread.", vote: phase === "voting" ? "+" : undefined }] }, { title: "Actions", hint: "Later", count: "Notes", cards: [{ title: "Action discussion waits for top votes", text: "The highest voted pain cards will move here." }] }];
  if (phase === "actions" || phase === "completed") return [{ title: "How do you feel?", hint: "Team mood", count: phase === "completed" ? "Steady" : "Revealed", cards: [{ title: "Mixed but steady", text: "Pressure was real, but shared context kept the team moving." }] }, { title: "What went well?", hint: "Context", count: "Visible", cards: [seed.good[0]] }, { title: "What went wrong?", hint: "Digest source", count: "Discussed", cards: [{ title: phase === "completed" ? "Board digest" : "Other pain cards remain here", text: "Hotfix branching and alert noise made alignment harder.", summary: phase === "completed" }] }, { title: "Actions", hint: "Top voted", count: "2 confirmed, 1 unresolved", cards: [{ title: "🎯 Release lead rotation", text: "Buddy: Ava. Check-in next retro.", action: true }, { title: "🕐 Earlier support sign-off", text: "Buddy: Lee. Try by Friday.", action: true }, { title: "🔕 Alert noise", text: "Unresolved recurring pain." }] }];
  return [{ title: "How do you feel?", hint: "Revealed", count: "6", cards: seed.mood }, { title: "What went well?", hint: "Revealed", count: "7", cards: seed.good }, { title: "What went wrong?", hint: "Revealed", count: "7", cards: seed.bad }, { title: "Actions", hint: "Notes", count: "1", cards: [{ title: "Maybe release lead", text: "One person narrates the current plan." }] }];
}
