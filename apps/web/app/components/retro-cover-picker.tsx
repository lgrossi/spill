"use client";

import { useState } from "react";
import type { GifResult } from "@/lib/contracts";
import { updateRetroDetailsAction } from "@/lib/actions";
import { GifPickerOverlay } from "./gif-picker-overlay";

type CoverValue = {
  url: string | null;
  altText: string | null;
};

export function RetroCoverPicker({
  initialCover,
  mode,
  retroId,
  returnTo,
  size = "large",
}: {
  initialCover?: CoverValue;
  mode: "create" | "update";
  retroId?: string;
  returnTo?: string;
  size?: "small" | "large" | "profile" | "hero";
}) {
  const [open, setOpen] = useState(false);
  const [cover, setCover] = useState<CoverValue>({
    url: initialCover?.url ?? null,
    altText: initialCover?.altText ?? null,
  });

  function choose(gif: GifResult) {
    setCover({ url: gif.url, altText: gif.alt_text });
    if (mode === "create") {
      setOpen(false);
    }
  }

  return (
    <div className="relative">
      <input name="cover_gif_url" type="hidden" value={cover.url ?? ""} />
      <input name="cover_gif_alt_text" type="hidden" value={cover.altText ?? ""} />
      <button
        aria-label={cover.url ? "Change cover GIF" : "Pick cover GIF"}
        className="group/cover block rounded-[12px] text-left focus-visible:outline-none focus-visible:shadow-[var(--focus)]"
        onClick={() => setOpen(true)}
        type="button"
      >
        <CoverSquare cover={cover} interactive size={size} />
      </button>
      {cover.url ? (
        mode === "update" ? (
          <form action={updateRetroDetailsAction} className="absolute right-1.5 top-1.5 z-10">
            <input name="retro_id" type="hidden" value={retroId ?? ""} />
            <input name="return_to" type="hidden" value={returnTo ?? (retroId ? `/retros/${retroId}` : "/")} />
            <input name="remove_cover_gif" type="hidden" value="1" />
            <button
              aria-label="Remove cover GIF"
              className="grid h-6 w-6 place-items-center rounded-full border border-white/40 bg-black/45 text-[13px] font-extrabold leading-none text-white shadow-[0_1px_2px_rgba(0,0,0,0.18)] transition hover:bg-black/60 focus-visible:outline-none focus-visible:shadow-[var(--focus)]"
              disabled={!retroId}
              type="submit"
            >
              ×
            </button>
          </form>
        ) : (
          <button
            aria-label="Remove cover GIF"
            className="absolute right-1.5 top-1.5 z-10 grid h-6 w-6 place-items-center rounded-full border border-white/40 bg-black/45 text-[13px] font-extrabold leading-none text-white shadow-[0_1px_2px_rgba(0,0,0,0.18)] transition hover:bg-black/60 focus-visible:outline-none focus-visible:shadow-[var(--focus)]"
            onClick={() => setCover({ url: null, altText: null })}
            type="button"
          >
            ×
          </button>
        )
      ) : null}
      {open ? (
        <CoverPickerPopover
          cover={cover}
          mode={mode}
          onChoose={choose}
          onClose={() => setOpen(false)}
          returnTo={returnTo}
          retroId={retroId}
          setCover={setCover}
        />
      ) : null}
    </div>
  );
}

export function CoverSquare({
  cover,
  interactive = false,
  size = "small",
}: {
  cover?: CoverValue;
  interactive?: boolean;
  size?: "small" | "large" | "profile" | "hero";
}) {
  const className =
    size === "hero"
      ? "h-[118px] w-[118px] rounded-[14px]"
      : size === "profile"
        ? "h-[132px] w-[132px] rounded-[16px]"
      : size === "large"
        ? "h-[92px] w-[92px] rounded-[12px]"
        : "h-12 w-12 rounded-[9px]";

  if (cover?.url) {
    return (
      <span className={`relative block overflow-hidden border border-spill-line bg-[var(--panel-hi)] shadow-[var(--shadow-2)] ${className}`}>
        <img alt={cover.altText ?? "Retro cover GIF"} className="h-full w-full object-cover" loading="lazy" src={cover.url} />
        {interactive ? (
          <span className="absolute inset-x-1.5 bottom-1.5 rounded-[6px] bg-black/55 px-1.5 py-0.5 text-center text-[9px] font-extrabold uppercase tracking-[0.08em] text-white opacity-0 transition group-hover/cover:opacity-100">
            change
          </span>
        ) : null}
      </span>
    );
  }

  return (
    <span className={`grid place-items-center border ${interactive ? "border-dashed" : ""} border-spill-line bg-[var(--panel-hi)] text-spill-muted shadow-[var(--shadow-1)] transition ${interactive ? "group-hover/cover:border-spill-wrong group-hover/cover:text-spill-wrong" : ""} ${className}`}>
      {interactive ? (
        <span className="text-center">
          <span className="block text-[22px] font-extrabold leading-none">+</span>
          {size !== "small" ? <span className="mt-1 block text-[9px] font-extrabold uppercase tracking-[0.1em]">cover</span> : null}
        </span>
      ) : (
        <span className="h-3 w-3 rounded-full bg-spill-line shadow-[inset_0_1px_0_rgba(255,255,255,0.4)]" />
      )}
    </span>
  );
}

function CoverPickerPopover({
  cover,
  mode,
  onChoose,
  onClose,
  returnTo,
  retroId,
  setCover,
}: {
  cover: CoverValue;
  mode: "create" | "update";
  onChoose: (gif: GifResult) => void;
  onClose: () => void;
  returnTo?: string;
  retroId?: string;
  setCover: (cover: CoverValue) => void;
}) {
  return (
    <GifPickerOverlay
      ariaLabel="Cover GIF picker"
      columns="cover"
      emptyText="Search for a GIF to use as the board cover."
      kicker="opened from square"
      onClose={onClose}
      placeholder="team celebration"
      selected={(gif) => cover.url === gif.url}
      title="Pick a cover GIF"
      renderResult={(gif, selected, className, image) =>
        mode === "update" ? (
          <form action={updateRetroDetailsAction} key={`${gif.id}-${gif.url}`}>
            <input name="retro_id" type="hidden" value={retroId ?? ""} />
            <input name="return_to" type="hidden" value={returnTo ?? (retroId ? `/retros/${retroId}` : "/")} />
            <input name="cover_gif_url" type="hidden" value={gif.url} />
            <input name="cover_gif_alt_text" type="hidden" value={gif.alt_text} />
            <button
              className={`${className} ${selected ? "border-spill-wrong ring-2 ring-spill-wrong/45" : "border-spill-line"}`}
              disabled={!retroId}
              type="submit"
            >
              {selected ? <span className="absolute right-2 top-2 z-10 grid h-5 w-5 place-items-center rounded-full bg-spill-wrong text-[12px] font-extrabold text-white">✓</span> : null}
              {image}
            </button>
          </form>
        ) : (
          <button
            className={`${className} ${selected ? "border-spill-wrong ring-2 ring-spill-wrong/45" : "border-spill-line"}`}
            key={`${gif.id}-${gif.url}`}
            onClick={() => onChoose(gif)}
            type="button"
          >
            {selected ? <span className="absolute right-2 top-2 z-10 grid h-5 w-5 place-items-center rounded-full bg-spill-wrong text-[12px] font-extrabold text-white">✓</span> : null}
            {image}
          </button>
        )
      }
    />
  );
}
