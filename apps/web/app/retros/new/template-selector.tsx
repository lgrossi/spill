const templateCardClass = "relative cursor-pointer rounded-xl border border-spill-line bg-spill-panel p-4 transition hover:-translate-y-0.5 hover:shadow-[0_10px_18px_rgba(42,34,27,0.10)] [&:has(input:checked)]:border-2 [&:has(input:checked)]:border-spill-wrong [&:has(input:checked)]:bg-spill-wrong/10 [&:has(input:checked)]:shadow-[0_10px_18px_rgba(42,34,27,0.14)] [&:has(input:checked)_.template-selected-badge]:opacity-100";
const miniChipClass = "inline-flex rounded-full border border-spill-line px-2 py-0.5 text-xs text-spill-muted";

const templates = [
  {
    id: "standard",
    label: "standard",
    chips: ["mood", "went well", "went wrong", "actions"],
  },
  {
    id: "4ls",
    label: "4 Ls",
    chips: ["liked", "lacked", "learned", "longed for"],
  },
  {
    id: "custom",
    label: "custom",
    chips: ["up to 4 columns", "+ actions"],
  },
] as const;

export function TemplateSelector() {
  return (
    <div className="[&:has(input[value=custom]:checked)_.custom-template-editor]:block">
      <div className="mt-3 grid gap-3 md:grid-cols-3">
        {templates.map((template) => (
          <label className={templateCardClass} key={template.id}>
            <input
              className="sr-only"
              defaultChecked={template.id === "standard"}
              name="template"
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

      <div className="custom-template-editor mt-3 hidden rounded-xl border border-spill-action/40 bg-spill-action/5 p-4">
        <div className="flex items-baseline justify-between gap-4">
          <p className="text-xs font-extrabold uppercase tracking-widest text-spill-action">custom columns</p>
          <p className="text-xs text-spill-muted">1-4 columns · Actions is added underneath</p>
        </div>
        <div className="mt-3 grid gap-2 md:grid-cols-2">
          {[
            ["wins", "Wins"],
            ["pains", "Pains"],
            ["questions", "Questions"],
            ["love notes", "Love notes"],
          ].map(([placeholder, value], index) => (
            <label className="text-xs font-bold uppercase tracking-wider text-spill-muted" key={placeholder}>
              column {index + 1}
              <input
                className="mt-1 px-3 py-2 text-sm font-bold"
                defaultValue={value}
                maxLength={32}
                name="custom_column"
                placeholder={placeholder}
              />
            </label>
          ))}
        </div>
      </div>
    </div>
  );
}
