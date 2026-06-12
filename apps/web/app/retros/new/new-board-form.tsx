"use client";

import { useState } from "react";
import { Btn, Tile, fieldControlClass, spillColors } from "@/components/spill-ui";
import { createRetroAction } from "@/lib/actions";
import { InvitePanel } from "@/components/invite-panel";
import { localDateString } from "@/lib/retro-dates";
import { RetroCoverPicker } from "@/components/retro-cover-picker";

export type TemplateId = "standard" | "4ls" | "custom";

const ruleInputClass = `${fieldControlClass} mt-1 min-h-8 py-1.5 text-[13.5px]`;
const customColorOptions = [spillColors.mood, spillColors.well, spillColors.wrong, spillColors.action, "#0f5f72"];
const defaultCustomColumns = ["Wins", "Pains", "Questions", "Love notes"];
const defaultCustomColors = [spillColors.well, spillColors.wrong, "#0f5f72", spillColors.action];
const templates: {
  id: TemplateId;
  label: string;
  sub: string;
  chips: string[];
}[] = [
  { id: "standard", label: "Standard", sub: "recommended", chips: ["How are you feeling?", "went well", "to improve"] },
  { id: "4ls", label: "4 Ls", sub: "reflective", chips: ["liked", "lacked", "learned", "longed for"] },
  { id: "custom", label: "Custom", sub: "design your own", chips: ["any number of columns"] },
];

export function NewBoardForm({ selectedTemplate }: { selectedTemplate: TemplateId }) {
  const [template, setTemplate] = useState<TemplateId>(selectedTemplate);
  const [customColumns, setCustomColumns] = useState(defaultCustomColumns);
  const [customColors, setCustomColors] = useState(defaultCustomColors);
  const [votingEnabled, setVotingEnabled] = useState(true);
  const [topVotedToActions, setTopVotedToActions] = useState(true);
  const [autoOrganize, setAutoOrganize] = useState(false);
  const [invitees, setInvitees] = useState<{ email: string; role: "host" | "member" }[]>([]);
  const today = localDateString(new Date());

  function updateColumn(index: number, value: string) {
    setCustomColumns((columns) => columns.map((column, columnIndex) => (columnIndex === index ? value : column)));
  }

  function addColumn() {
    setCustomColumns((columns) => [...columns, `Column ${columns.length + 1}`]);
    setCustomColors((colors) => [...colors, customColorOptions[colors.length % customColorOptions.length]]);
  }

  function removeColumn(index: number) {
    setCustomColumns((columns) => (columns.length <= 1 ? columns : columns.filter((_, columnIndex) => columnIndex !== index)));
    setCustomColors((colors) => (colors.length <= 1 ? colors : colors.filter((_, colorIndex) => colorIndex !== index)));
  }

  function updateColumnColor(index: number, color: string) {
    setCustomColors((colors) => colors.map((item, colorIndex) => (colorIndex === index ? color : item)));
  }

  return (
    <form id="new-board-form" action={createRetroAction} className="grid flex-1 grid-cols-1 gap-8 overflow-auto p-6 md:p-8 lg:grid-cols-[minmax(0,1fr)_360px]">
      <section className="min-w-0 space-y-7">
        <div>
          <div className="mb-2 flex items-center gap-2">
            <StepNum n="1" />
            <label className="text-[13px] font-bold text-spill-fg" htmlFor="title">Name it</label>
            <span className="text-[11px] text-spill-muted">what should people recognize?</span>
          </div>
          <div className="flex flex-col gap-4 sm:flex-row sm:items-start">
            <RetroCoverPicker mode="create" size="profile" />
            <div className="min-w-0 flex-1">
              <input id="title" name="title" required placeholder="e.g. Team retro - May 25" className={`${fieldControlClass} min-h-[56px] border-2 border-spill-wrong px-4 text-2xl font-bold tracking-[-0.02em] shadow-[var(--focus)]`} aria-label="Retro title" />
              <p className="mt-2 text-[11px] font-semibold text-spill-muted">Click the square to add a cover GIF.</p>
            </div>
          </div>
        </div>

        <div>
          <div className="mb-2 flex items-center gap-2">
            <StepNum n="2" />
            <label className="text-[13px] font-bold text-spill-fg" htmlFor="group_name">Group</label>
            <span className="text-[11px] text-spill-muted">squad, team, or stream</span>
          </div>
          <input id="group_name" name="group_name" placeholder="e.g. Growth squad" className={`${fieldControlClass} min-h-11 w-full max-w-sm`} aria-label="Retro group" />
        </div>

        <div>
          <div className="mb-2 flex items-center gap-2">
            <StepNum n="3" />
            <label className="text-[13px] font-bold text-spill-fg" htmlFor="planned_for">Retro date</label>
            <span className="text-[11px] text-spill-muted">future dates wait on the wall</span>
          </div>
          <input id="planned_for" name="planned_for" type="date" defaultValue={today} className={`${fieldControlClass} min-h-11 w-full max-w-sm`} />
        </div>

        <div>
          <div className="mb-2 flex items-center gap-2">
            <StepNum n="4" />
            <p className="text-[13px] font-bold text-spill-fg">Pick a shape</p>
          </div>
          <TemplatePicker template={template} setTemplate={setTemplate} />
          {template === "custom" ? (
            <CustomColumns columns={customColumns} colors={customColors} updateColumn={updateColumn} updateColumnColor={updateColumnColor} addColumn={addColumn} removeColumn={removeColumn} />
          ) : null}
        </div>

        <div>
          <div className="mb-3 flex items-center gap-2">
            <StepNum n="5" />
            <p className="text-[13px] font-bold text-spill-fg">House rules</p>
          </div>
          <div className="grid gap-2.5 md:grid-cols-2">
            <Tile className="flex items-center gap-3">
              <RuleMark>●●●</RuleMark>
              <div className="min-w-0 flex-1">
                <label className="group/check flex min-w-0 cursor-pointer items-center justify-between gap-3">
                  <input name="voting_enabled" type="hidden" value="0" />
                  <input className="sr-only" name="voting_enabled" type="checkbox" value="1" checked={votingEnabled} onChange={(event) => setVotingEnabled(event.currentTarget.checked)} />
                  <span className="min-w-0">
                    <span className="block text-[10.5px] font-extrabold uppercase tracking-[0.12em] text-spill-muted">voting</span>
                    <span className={`mt-1 flex items-center gap-1.5 text-[12px] font-semibold transition ${votingEnabled ? "text-[var(--fg-2)]" : "text-spill-muted/60"}`}>
                      <input
                        className="h-6 w-8 bg-transparent px-0 text-center text-[12px] font-extrabold text-spill-fg outline-none disabled:text-spill-muted/45"
                        name="vote_limit"
                        type="number"
                        min="1"
                        defaultValue="3"
                        disabled={!votingEnabled}
                        required={votingEnabled}
                        aria-label="Votes per person"
                        style={{ borderWidth: "0 0 1px", borderStyle: "solid", borderColor: "var(--line)", borderRadius: 0, boxShadow: "none" }}
                      />
                      votes per person
                    </span>
                  </span>
                  <span className="grid h-6 w-6 shrink-0 place-items-center rounded-[6px] border border-spill-line bg-[var(--paper)] text-[14px] font-extrabold text-transparent transition group-has-[input:checked]/check:border-spill-well group-has-[input:checked]/check:bg-spill-well group-has-[input:checked]/check:text-white">
                    ✓
                  </span>
                </label>
              </div>
            </Tile>
            <Tile className="flex items-center gap-3">
              <RuleMark>★</RuleMark>
              <div className="min-w-0 flex-1">
                <label className="group/check flex min-w-0 cursor-pointer items-center justify-between gap-3">
                  <input name="action_discussion_enabled" type="hidden" value="0" />
                  <input className="sr-only" name="action_discussion_enabled" type="checkbox" value="1" checked={topVotedToActions} onChange={(event) => setTopVotedToActions(event.currentTarget.checked)} />
                  <span className="min-w-0">
                    <span className="block text-[10.5px] font-extrabold uppercase tracking-[0.12em] text-spill-muted">action column</span>
                    <span className={`mt-1 flex items-center gap-1.5 text-[12px] font-semibold transition ${topVotedToActions ? "text-[var(--fg-2)]" : "text-spill-muted/60"}`}>
                      move
                      <input
                        className="h-6 w-8 bg-transparent px-0 text-center text-[12px] font-extrabold text-spill-fg outline-none disabled:text-spill-muted/45"
                        name="action_discussion_limit"
                        type="number"
                        min="1"
                        defaultValue="3"
                        disabled={!topVotedToActions}
                        required={topVotedToActions}
                        aria-label="Number of top voted cards moved to actions"
                        style={{ borderWidth: "0 0 1px", borderStyle: "solid", borderColor: "var(--line)", borderRadius: 0, boxShadow: "none" }}
                      />
                      top voted cards to actions
                    </span>
                  </span>
                  <span className="grid h-6 w-6 shrink-0 place-items-center rounded-[6px] border border-spill-line bg-[var(--paper)] text-[14px] font-extrabold text-transparent transition group-has-[input:checked]/check:border-spill-well group-has-[input:checked]/check:bg-spill-well group-has-[input:checked]/check:text-white">
                    ✓
                  </span>
                </label>
              </div>
            </Tile>
            <Tile className="flex items-center gap-3">
              <RuleMark>◆</RuleMark>
              <label className="group/check flex min-w-0 flex-1 cursor-pointer items-center justify-between gap-3">
                <input name="clustering_mode" type="hidden" value="disabled" />
                <input className="sr-only" name="clustering_mode" type="checkbox" value="auto_on_vote_start" checked={autoOrganize} onChange={(event) => setAutoOrganize(event.currentTarget.checked)} />
                <span className="min-w-0">
                  <span className="block text-[10.5px] font-extrabold uppercase tracking-[0.12em] text-spill-muted">organize before voting</span>
                  <span className={`mt-1 block text-[11px] font-semibold transition ${autoOrganize ? "text-[var(--fg-2)]" : "text-spill-muted/60"}`}>
                    {autoOrganize ? "AI groups and tags cards first" : "manual grouping"}
                  </span>
                </span>
                <span className="grid h-6 w-6 shrink-0 place-items-center rounded-[6px] border border-spill-line bg-[var(--paper)] text-[14px] font-extrabold text-transparent transition group-has-[input:checked]/check:border-spill-well group-has-[input:checked]/check:bg-spill-well group-has-[input:checked]/check:text-white">
                  ✓
                </span>
              </label>
            </Tile>
            <Tile className="flex items-center gap-3">
              <RuleMark>☉</RuleMark>
              <CheckboxRule
                label="reveal mode"
                name="reveal_mode"
                offValue="host"
                onValue="ready"
              />
            </Tile>
          </div>
        </div>

        <div>
          <div className="mb-3 flex items-center gap-2">
            <StepNum n="6" />
            <p className="text-[13px] font-bold text-spill-fg">Invite the crew</p>
          </div>
          <InvitePanel mode="create" onInviteesChange={setInvitees} />
          {invitees.map(({ email, role }) => (
            <span key={email} style={{ display: "none" }}>
              <input type="hidden" name="invitee_email" value={email} />
              <input type="hidden" name="invitee_role" value={role} />
            </span>
          ))}
        </div>
      </section>

      <aside className="space-y-4">
        <p className="text-[10.5px] font-extrabold uppercase tracking-[0.12em] text-spill-muted">live preview</p>
        <BoardPreview columns={previewColumns(template, customColumns, customColors, topVotedToActions)} />

        <Tile className="border-spill-action/60 bg-spill-action/10">
          <p className="text-[10.5px] font-extrabold uppercase tracking-[0.1em] text-spill-action">how it works</p>
          <p className="mt-2 text-[12.5px] leading-6 text-[var(--fg-2)]">Today and past dates open in writing mode. Future dates wait on the wall until the host starts them. Board access is invite-only.</p>
        </Tile>

        <Tile>
          <p className="text-[10.5px] font-extrabold uppercase tracking-[0.1em] text-spill-muted">opens with</p>
          <div className="mt-2 space-y-2 text-[12.5px] text-[var(--fg-2)]">
            <Line>action ideas can be captured while writing</Line>
            <Line>paper grain and semantic column colors</Line>
            <Line>presence, voting, manual grouping, and wrap-up</Line>
          </div>
        </Tile>

        <Btn className="w-full md:hidden" kind="primary" type="submit">pin it up</Btn>
      </aside>
    </form>
  );
}

function TemplatePicker({ template, setTemplate }: { template: TemplateId; setTemplate: (template: TemplateId) => void }) {
  return (
    <div className="mt-3 grid gap-3 md:grid-cols-3">
      {templates.map((option) => (
        <label
          className="sp-panel-grain relative min-h-[84px] cursor-pointer rounded-[12px] border border-spill-line bg-spill-panel p-3.5 shadow-[var(--shadow-1)] transition hover:-translate-y-0.5 hover:border-spill-wrong/60 hover:shadow-[var(--shadow-2)] [&:has(input:checked)]:border-spill-wrong [&:has(input:checked)]:bg-spill-wrong/10 [&:has(input:checked)]:shadow-[var(--shadow-2)] [&:has(input:checked)_.template-selected-badge]:opacity-100"
          key={option.id}
        >
          <input checked={template === option.id} className="sr-only" name="template" onChange={() => setTemplate(option.id)} type="radio" value={option.id} />
          <span className="block pr-7 text-[15px] font-bold leading-tight tracking-[-0.01em] text-spill-fg">{option.label}</span>
          {option.sub ? <span className="mt-1 block text-[10.5px] font-bold uppercase tracking-[0.08em] text-spill-muted">{option.sub}</span> : null}
          <span className="template-selected-badge absolute right-3 top-3 grid h-[18px] w-[18px] place-items-center rounded-full bg-spill-wrong text-[11px] font-extrabold text-white opacity-0">✓</span>
          <span className="mt-2 flex flex-wrap gap-1.5">
            {option.chips.map((item, index) => {
              const accent = colorForPreviewColumn(item, index);
              return (
                <span className="rounded-full border px-2 py-1 text-[10px] font-semibold leading-none" key={item} style={{ borderColor: `${accent}44`, background: `${accent}1c`, color: accent }}>
                  {item}
                </span>
              );
            })}
          </span>
        </label>
      ))}
    </div>
  );
}

function CustomColumns({
  columns,
  colors,
  updateColumn,
  updateColumnColor,
  addColumn,
  removeColumn,
}: {
  columns: string[];
  colors: string[];
  updateColumn: (index: number, value: string) => void;
  updateColumnColor: (index: number, value: string) => void;
  addColumn: () => void;
  removeColumn: (index: number) => void;
}) {
  return (
    <div className="mt-3 rounded-[12px] border border-spill-action/40 bg-spill-action/5 p-4">
      <div className="flex flex-wrap items-center justify-between gap-3">
        <p className="text-[10.5px] font-extrabold uppercase tracking-[0.12em] text-spill-action">custom columns</p>
        <button aria-label="Add column" className="grid h-8 w-8 place-items-center rounded-[8px] border border-spill-line bg-[var(--panel-hi)] text-[18px] font-extrabold leading-none text-[var(--fg-2)] shadow-[inset_0_1px_0_rgba(255,255,255,0.6),0_1px_0_rgba(74,52,20,0.06)] transition hover:border-spill-action hover:text-spill-action" onClick={addColumn} type="button">+</button>
      </div>
      <div className="mt-3 grid gap-2">
        {columns.map((column, index) => (
          <label className="grid gap-1 text-[10px] font-bold uppercase tracking-[0.1em] text-spill-muted sm:grid-cols-[40px_minmax(0,1fr)_auto_auto] sm:items-center" key={index}>
            <span>{index + 1}</span>
            <input className={`${fieldControlClass} text-sm`} maxLength={32} name="custom_column" onChange={(event) => updateColumn(index, event.currentTarget.value)} placeholder={`Column ${index + 1}`} required value={column} />
            <input aria-label={`Column ${index + 1} color`} className="h-8 w-8 cursor-pointer rounded-[7px] border border-spill-line bg-[var(--panel-hi)] p-1" name="custom_column_color" onChange={(event) => updateColumnColor(index, event.currentTarget.value)} type="color" value={colors[index] ?? customColorOptions[index % customColorOptions.length]} />
            <button aria-label={`Remove column ${index + 1}`} className="grid h-8 w-8 place-items-center rounded-[7px] border border-spill-line text-[16px] font-extrabold leading-none text-spill-muted transition hover:border-spill-wrong hover:text-spill-wrong disabled:pointer-events-none disabled:opacity-35" disabled={columns.length <= 1} onClick={() => removeColumn(index)} type="button">×</button>
          </label>
        ))}
      </div>
    </div>
  );
}

function previewColumns(template: TemplateId, customColumns: string[], customColors: string[], includeActions: boolean) {
  const labels = template === "4ls"
    ? ["liked", "lacked", "learned", "longed"]
    : template === "custom"
      ? customColumns.map((column) => column.trim() || "column")
      : ["How are you feeling?", "Went well", "To improve"];
  const columns = labels.map((label, index) => ({ label, color: template === "custom" ? customColors[index] ?? colorForPreviewColumn(label, index) : colorForPreviewColumn(label, index) }));
  return includeActions ? [...columns, { label: "actions", color: spillColors.action }] : columns;
}

function BoardPreview({ columns }: { columns: { label: string; color: string }[] }) {
  return (
    <Tile className="overflow-hidden p-0 shadow-[var(--shadow-3)]">
      <div className="sp-panel-grain flex justify-between border-b border-spill-line bg-spill-panel px-3.5 py-2.5 text-[11px] text-spill-muted">
        <span className="font-semibold text-spill-fg">New board</span>
        <span>{columns.length} cols. 3 votes</span>
      </div>
      <div className="sp-paper p-3">
        <div className="grid gap-1.5" style={{ gridTemplateColumns: `repeat(${Math.max(1, columns.length)}, minmax(0, 1fr))` }}>
          {columns.map(({ label, color }, index) => {
            return (
              <div className="min-w-0" key={`${label}-${index}`}>
                <div className="mb-1 flex items-center gap-1">
                  <span className="h-1.5 w-1.5 rounded-full" style={{ backgroundColor: color }} />
                  <span className="truncate text-[8px] font-extrabold uppercase text-spill-muted">{label}</span>
                </div>
                <div className="h-6 rounded-[4px]" style={{ backgroundColor: color, boxShadow: "inset 0 -2px 4px rgba(0,0,0,0.10)" }} />
                {index === 1 ? <div className="mt-1 h-4 rounded-[4px] opacity-75" style={{ backgroundColor: color }} /> : null}
              </div>
            );
          })}
        </div>
      </div>
    </Tile>
  );
}

function colorForPreviewColumn(label: string, index = 0) {
  const normalized = label.toLowerCase();
  if (normalized.includes("action")) return spillColors.action;
  if (normalized.includes("well") || normalized.includes("liked")) return spillColors.well;
  if (normalized.includes("wrong") || normalized.includes("lacked") || normalized.includes("improve")) return spillColors.wrong;
  if (normalized.includes("learned")) return "#0f5f72";
  if (normalized.includes("longed")) return spillColors.mood;
  if (normalized.includes("feeling")) return "#0f5f72";
  if (normalized.includes("mood")) return spillColors.mood;
  return customColorOptions[index % customColorOptions.length];
}

function StepNum({ n }: { n: string }) {
  return <span className="grid h-[22px] w-[22px] shrink-0 place-items-center rounded-full bg-spill-fg text-[11px] font-extrabold text-[var(--paper)] shadow-[var(--shadow-1)]">{n}</span>;
}

function RuleMark({ children }: { children: string }) {
  return <span className="grid h-[34px] w-[42px] shrink-0 place-items-center rounded-[8px] border border-spill-action/35 bg-spill-action/10 text-[12px] font-extrabold text-spill-action">{children}</span>;
}

function CheckboxRule({
  label,
  name,
  offValue,
  onValue,
}: {
  label: string;
  name: string;
  offValue: string;
  onValue: string;
}) {
  return (
    <label className="group/check flex min-w-0 flex-1 cursor-pointer items-center justify-between gap-3">
      <input name={name} type="hidden" value={offValue} />
      <input className="sr-only" name={name} type="checkbox" value={onValue} defaultChecked />
      <p className="text-[10.5px] font-extrabold uppercase tracking-[0.12em] text-spill-muted">{label}</p>
      <span className="grid h-6 w-6 shrink-0 place-items-center rounded-[6px] border border-spill-line bg-[var(--paper)] text-[14px] font-extrabold text-transparent transition group-has-[input:checked]/check:border-spill-well group-has-[input:checked]/check:bg-spill-well group-has-[input:checked]/check:text-white">
        ✓
      </span>
    </label>
  );
}

function Line({ children }: { children: string }) {
  return (
    <p className="flex items-center gap-2">
      <span className="h-2 w-2 rounded-full bg-spill-well" />
      <span>{children}</span>
    </p>
  );
}
