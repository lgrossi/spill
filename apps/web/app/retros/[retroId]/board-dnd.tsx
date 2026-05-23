"use client";

import type { ReactNode } from "react";
import { useState } from "react";

let activePointerCardId = "";

export function DropColumn({
  children,
  columnId,
  enabled,
}: {
  children: ReactNode;
  columnId: string;
  enabled: boolean;
}) {
  const [active, setActive] = useState(false);

  return (
    <section
      className={`grid min-h-[460px] grid-rows-[auto_minmax(0,1fr)_auto] gap-3 rounded-xl transition ${active ? "ring-2 ring-spill-wrong" : ""}`}
      data-spill-column-id={enabled ? columnId : undefined}
      onPointerEnter={() => {
        if (activePointerCardId && enabled) {
          setActive(true);
        }
      }}
      onPointerLeave={() => {
        setActive(false);
      }}
    >
      {children}
    </section>
  );
}

export function DraggableCard({
  children,
  cardId,
  columnId,
  enabled,
}: {
  children: ReactNode;
  cardId: string;
  columnId: string;
  enabled: boolean;
}) {
  return (
    <article
      className={`rounded-xl p-3 text-white shadow-[0_8px_14px_rgba(42,34,27,0.13)] transition select-none ${enabled ? "cursor-grab active:cursor-grabbing" : ""}`}
      data-spill-card-column-id={enabled ? columnId : undefined}
      data-spill-card-id={enabled ? cardId : undefined}
      onPointerDown={(event) => {
        if (enabled) {
          activePointerCardId = cardId;
          const onPointerUp = (upEvent: PointerEvent) => {
            document.removeEventListener("pointerup", onPointerUp);
            activePointerCardId = "";
          };
          document.addEventListener("pointerup", onPointerUp);
          event.currentTarget.setPointerCapture(event.pointerId);
        }
      }}
    >
      {children}
    </article>
  );
}
