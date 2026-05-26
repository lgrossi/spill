"use client";

import { createContext, useContext, useState } from "react";
import type { ReactNode } from "react";

export type GifSelection = {
  id: string;
  url: string;
  preview_url: string;
  alt_text: string;
};

type GifDraftContextValue = {
  selectedGif: GifSelection | null;
  removed: boolean;
  hasSelectedGif: boolean;
  selectGif: (gif: GifSelection) => void;
  removeGif: () => void;
};

const GifDraftContext = createContext<GifDraftContextValue | null>(null);

export function GifDraftProvider({ children, initialGif }: { children: ReactNode; initialGif?: GifSelection | null }) {
  const [selectedGif, setSelectedGif] = useState<GifSelection | null>(initialGif ?? null);
  const [removed, setRemoved] = useState(false);

  return (
    <GifDraftContext.Provider
      value={{
        selectedGif,
        removed,
        hasSelectedGif: Boolean(selectedGif),
        selectGif: (gif) => {
          setSelectedGif(gif);
          setRemoved(false);
        },
        removeGif: () => {
          setSelectedGif(null);
          setRemoved(true);
        },
      }}
    >
      {children}
    </GifDraftContext.Provider>
  );
}

export function GifSelectedPreview() {
  const { selectedGif, removed, removeGif } = useGifDraft();

  return (
    <>
      {selectedGif ? (
        <>
          <input name="gif_choice" type="hidden" value={JSON.stringify({ url: selectedGif.url, altText: selectedGif.alt_text })} />
          <div className="relative mb-2 aspect-video w-full min-w-0 max-w-full overflow-hidden rounded-[6px] bg-black/15 shadow-[inset_0_0_0_1px_rgba(255,255,255,0.14)]">
            <img alt="" className="block h-full w-full min-w-0 max-w-full object-contain" loading="lazy" src={selectedGif.preview_url || selectedGif.url} />
            <button aria-label="Remove selected GIF" className="absolute right-2 top-2 grid h-6 w-6 place-items-center rounded-full border border-white/35 bg-black/35 text-[13px] font-extrabold leading-none text-white/90 shadow-[0_1px_2px_rgba(0,0,0,0.16)] transition hover:bg-black/45" onClick={removeGif} type="button">×</button>
          </div>
        </>
      ) : null}
      {removed ? <input name="gif_remove" type="hidden" value="1" /> : null}
    </>
  );
}

export function useGifDraftStatus() {
  const { hasSelectedGif, removed } = useGifDraft();
  return { hasSelectedGif, removed };
}

export function useGifDraft() {
  const context = useContext(GifDraftContext);
  if (!context) {
    throw new Error("GIF draft controls must be rendered inside GifDraftProvider");
  }
  return context;
}
