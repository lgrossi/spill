"use client";

import { useState } from "react";

const templateCardClass = "relative cursor-pointer rounded-xl border border-spill-line bg-spill-panel p-4 transition hover:-translate-y-0.5 hover:shadow-[0_10px_18px_rgba(42,34,27,0.10)] [&:has(input:checked)]:border-2 [&:has(input:checked)]:border-spill-wrong [&:has(input:checked)]:bg-spill-wrong/10 [&:has(input:checked)]:shadow-[0_10px_18px_rgba(42,34,27,0.14)] [&:has(input:checked)_.template-selected-badge]:opacity-100";
const miniChipClass = "inline-flex rounded-full border border-spill-line px-2 py-0.5 text-xs text-spill-muted";

const templates = [
  {
    id: "standard",
    label: "standard",
    chips: ["mood", "went well", "went wrong", "actions"],
    columns: ["Mood", "Went well", "Went wrong", "Actions"],
  },
  {
    id: "4ls",
    label: "4 Ls",
    chips: ["liked", "lacked", "learned", "longed for"],
    columns: ["Liked", "Lacked", "Learned", "Longed for", "Actions"],
  },
  {
    id: "custom",
    label: "custom",
    chips: ["user deck mode"],
    columns: ["Liked", "Lacked", "Learned", "Longed for", "Actions"],
  },
] as const;

export function TemplateSelector() {
  const [selected, setSelected] = useState<(typeof templates)[number]["id"]>("standard");
  const [customColumns, setCustomColumns] = useState(templates[2].columns.join("\n"));
  const selectedTemplate = templates.find((template) => template.id === selected) ?? templates[0];
  const columnValue = selected === "custom" ? customColumns : selectedTemplate.columns.join("\n");

  return (
    <>
      <div className="mt-3 grid gap-3 md:grid-cols-3">
        {templates.map((template) => (
          <label className={templateCardClass} key={template.id}>
            <input
              className="sr-only"
              defaultChecked={template.id === "standard"}
              name="template"
              onChange={() => setSelected(template.id)}
              type="radio"
              value={template.id}
            />
            <span className="text-base font-extrabold">{template.label}</span>
            <span className="template-selected-badge absolute right-3 top-3 rounded-full bg-spill-wrong px-2 py-0.5 text-[10px] font-extrabold uppercase tracking-wider text-white opacity-0">
              selected
            </span>
            <span className="mt-2 flex flex-wrap gap-1">
              {template.chips.map((item) => (
                <span className={miniChipClass} key={item}>{item}</span>
              ))}
            </span>
          </label>
        ))}
      </div>
      <textarea
        className={selected === "custom" ? "mt-3 min-h-28 px-4 py-3 text-sm font-bold" : "sr-only"}
        name="columns"
        rows={5}
        value={columnValue}
        onChange={(event) => {
          if (selected === "custom") {
            setCustomColumns(event.target.value);
          }
        }}
        aria-label="Custom columns"
        readOnly={selected !== "custom"}
      />
    </>
  );
}
