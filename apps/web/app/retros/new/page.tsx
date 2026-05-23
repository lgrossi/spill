import { AppChrome, Pill, Tile } from "../../components/spill-ui";
import { createRetroAction } from "../../lib/actions";
import { TemplateSelector } from "./template-selector";

const inputClass = "mt-2 px-4 py-3 text-lg font-bold";
const labelClass = "text-xs font-bold uppercase tracking-widest text-spill-muted";
const ruleInputClass = "mt-1 border-0 bg-transparent p-0 text-base font-bold shadow-none focus:shadow-none";

export default function NewRetroPage() {
  return (
    <AppChrome
      title="new board"
      actions={
        <>
          <Pill href="/">cancel</Pill>
          <button className="inline-flex items-center justify-center rounded-full border border-spill-wrong bg-spill-wrong px-3 py-1.5 text-sm font-medium leading-none text-white transition hover:brightness-95" form="new-board-form" type="submit">
            pin it up
          </button>
        </>
      }
    >
      <form id="new-board-form" action={createRetroAction} className="grid min-h-[calc(100dvh-5rem)] grid-cols-1 gap-7 p-7 lg:grid-cols-[1fr_380px]">
        <section className="space-y-7">
          <div>
            <label className={labelClass} htmlFor="title">1 · name it</label>
            <input id="title" name="title" required defaultValue="Sprint 42" className={inputClass} aria-label="Retro title" />
            <p className="mt-2 text-sm text-spill-muted">auto-suggested from your last board</p>
          </div>

          <div>
            <p className={labelClass}>2 · template</p>
            <TemplateSelector />
          </div>

          <div>
            <p className={labelClass}>3 · house rules</p>
            <div className="mt-3 grid gap-3 md:grid-cols-2">
              <Tile>
                <label className="text-sm text-spill-muted">
                  votes per person
                  <input className={ruleInputClass} name="vote_limit" type="number" min="1" defaultValue="3" />
                </label>
              </Tile>
              <Tile>
                <label className="text-sm text-spill-muted">
                  top voted → action
                  <input className={ruleInputClass} name="action_discussion_limit" type="number" min="1" defaultValue="3" />
                </label>
              </Tile>
              <Tile>
                <p className="text-sm text-spill-muted">clustering</p>
                <p className="mt-1 font-bold">manual · on demand</p>
              </Tile>
              <Tile>
                <p className="text-sm text-spill-muted">reveal mode</p>
                <p className="mt-1 font-bold">when all marked ready</p>
              </Tile>
            </div>
          </div>
        </section>

        <aside className="space-y-4">
          <p className={labelClass}>preview</p>
          <Tile>
            <div className="flex justify-between text-xs text-spill-muted">
              <span>Sprint 42 · writing</span>
              <span>4 cols · 3 votes</span>
            </div>
            <div className="mt-4 grid grid-cols-4 gap-2">
              {[
                ["#d49a5c", "card"],
                ["#3aa676", "card"],
                ["#dd5c5c", "card"],
                ["#9e6cc4", "card"],
              ].map(([color, label]) => (
                <div key={color}>
                  <div className="mb-1 h-1 rounded-full" style={{ backgroundColor: color }} />
                  <div className="rounded px-2 py-2 text-xs font-bold text-white" style={{ backgroundColor: color }}>{label}</div>
                </div>
              ))}
            </div>
          </Tile>
          <div className="rounded-xl border border-spill-action/40 bg-spill-action/5 p-4">
            <p className="text-xs font-extrabold uppercase tracking-widest text-spill-action">heads up</p>
            <p className="mt-2 text-sm leading-6">
              Board opens in writing mode. Drafts stay private until everyone marks ready. Anyone with the link can pin a card up.
            </p>
          </div>
        </aside>
      </form>
    </AppChrome>
  );
}
