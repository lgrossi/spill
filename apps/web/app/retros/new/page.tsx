import Link from "next/link";
import { createRetroAction } from "../../lib/actions";

export default function NewRetroPage() {
  return (
    <main>
      <header className="topbar">
        <div>
          <p className="eyebrow">Create</p>
          <h1>Set the table</h1>
        </div>
        <nav className="tabs" aria-label="Main navigation">
          <Link className="button" href="/">
            Overview
          </Link>
          <Link className="button" href="/history">
            History
          </Link>
        </nav>
      </header>
      <div className="page single">
        <form className="scene" action={createRetroAction}>
          <div className="scene-head">
            <p className="eyebrow">Template + defaults</p>
            <h2>Create a retro</h2>
            <p>New boards always start in Writing. Drafts stay private and blurred until reveal work lands in the next slice.</p>
          </div>
          <div className="grid two">
            <article className="panel tape">
              <h3>Board setup</h3>
              <label>
                Retro title
                <input name="title" required defaultValue="Sprint retro" />
              </label>
              <label>
                Template
                <select name="template" defaultValue="standard">
                  <option value="standard">Standard retro</option>
                  <option value="custom">Custom columns</option>
                </select>
              </label>
              <label>
                Custom columns, one per line
                <textarea name="columns" rows={6} defaultValue={"Kudos\nFriction\nIdeas\nQuestions\nActions"} />
              </label>
            </article>
            <article className="panel tape">
              <h3>Light house rules</h3>
              <div className="grid two">
                <label>
                  Votes/person
                  <input name="vote_limit" type="number" min="1" defaultValue="3" />
                </label>
                <label>
                  Top action cards
                  <input name="action_discussion_limit" type="number" min="1" defaultValue="3" />
                </label>
              </div>
              <div className="list-card">
                <strong>Writing first</strong>
                <p className="muted">Creating a retro persists the board, columns, host participant, and settings before opening the board route.</p>
              </div>
              <button className="primary" type="submit">
                Create and open board
              </button>
            </article>
          </div>
        </form>
      </div>
    </main>
  );
}
