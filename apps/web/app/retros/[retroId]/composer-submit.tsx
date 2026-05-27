"use client";

import { useEffect, useRef, useState } from "react";
import { useGifDraftStatus } from "./gif-draft";

export function ComposerSubmit({ className, existingGif }: { className: string; existingGif?: boolean }) {
  const { hasSelectedGif, removed } = useGifDraftStatus();
  const [hasText, setHasText] = useState(false);
  const buttonRef = useRef<HTMLButtonElement>(null);

  useEffect(() => {
    const form = buttonRef.current?.form;
    const textarea = form?.querySelector<HTMLTextAreaElement>('textarea[name="body_text"]');
    const update = () => setHasText(Boolean(textarea?.value.trim()));
    update();
    textarea?.addEventListener("input", update);
    return () => textarea?.removeEventListener("input", update);
  }, []);

  const hasExistingGif = Boolean(existingGif) && !removed;

  return (
    <button
      aria-label="Save card"
      className={className}
      data-intent-card-submit
      disabled={!hasText && !hasSelectedGif && !hasExistingGif}
      ref={buttonRef}
      type="submit"
    >
      ✓
    </button>
  );
}
