"use client";

import type { ReactNode } from "react";
import { useEffect, useRef, useState } from "react";
import { useRouter } from "next/navigation";

export function useDropMarker(columnId: string, enabled: boolean) {
  const [dropState, setDropState] = useState<{ active: boolean; beforeCardId: string }>({ active: false, beforeCardId: "" });

  useEffect(() => {
    function handleDragOver(event: Event) {
      const detail = event instanceof CustomEvent ? event.detail : {};
      const nextColumnId = detail?.columnId || "";
      setDropState({ active: nextColumnId === columnId && enabled, beforeCardId: detail?.beforeCardId || "" });
    }

    function handleDragEnd() {
      setDropState({ active: false, beforeCardId: "" });
    }

    window.addEventListener("spill-drag-over", handleDragOver);
    window.addEventListener("spill-drag-end", handleDragEnd);
    return () => {
      window.removeEventListener("spill-drag-over", handleDragOver);
      window.removeEventListener("spill-drag-end", handleDragEnd);
    };
  }, [columnId, enabled]);

  return dropState;
}

export function DropColumn({
  children,
  columnId,
  enabled,
}: {
  children: ReactNode;
  columnId: string;
  enabled: boolean;
}) {
  const dropState = useDropMarker(columnId, enabled);

  return (
    <section
      className={`relative flex min-h-[460px] min-w-0 flex-col rounded-[12px] transition ${
        dropState.active ? "bg-[linear-gradient(180deg,rgba(255,248,230,0.24),rgba(255,248,230,0.06))]" : ""
      }`}
      data-spill-column-id={enabled ? columnId : undefined}
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
  clusteringEnabled = false,
  movingEnabled = true,
  retroId,
  accent,
}: {
  children: ReactNode;
  cardId: string;
  columnId: string;
  enabled: boolean;
  clusteringEnabled?: boolean;
  movingEnabled?: boolean;
  retroId: string;
  accent: string;
}) {
  const router = useRouter();
  const dragRef = useRef<{
    pointerId: number;
    startX: number;
    startY: number;
    width: number;
    active: boolean;
  } | null>(null);
  const [ghost, setGhost] = useState<{ x: number; y: number; width: number } | null>(null);
  const [insertBefore, setInsertBefore] = useState(false);
  const [clusterTarget, setClusterTarget] = useState(false);

  useEffect(() => {
    function handleDragOver(event: Event) {
      const detail = event instanceof CustomEvent ? event.detail : {};
      setInsertBefore(detail?.beforeCardId === cardId);
      setClusterTarget(detail?.clusterCardId === cardId);
    }

    function handleDragEnd() {
      setInsertBefore(false);
      setClusterTarget(false);
    }

    window.addEventListener("spill-drag-over", handleDragOver);
    window.addEventListener("spill-drag-end", handleDragEnd);
    return () => {
      window.removeEventListener("spill-drag-over", handleDragOver);
      window.removeEventListener("spill-drag-end", handleDragEnd);
    };
  }, [cardId]);

  function updateDragTarget(x: number, y: number) {
    const target = document.elementFromPoint(x, y);
    const column = target instanceof Element ? target.closest("[data-spill-column-id]") : null;
    const nextColumnId = column?.getAttribute("data-spill-column-id") || "";
    const clusterCardId = clusteringEnabled ? clusteringTargetCardId(target, cardId, x, y) : "";
    const beforeCardId = column && !clusterCardId ? insertionBeforeCardId(column, cardId, y) : "";
    window.dispatchEvent(new CustomEvent("spill-drag-over", { detail: { columnId: nextColumnId, beforeCardId, clusterCardId } }));
    return { nextColumnId, beforeCardId, clusterCardId };
  }

  async function finishDrag(x: number, y: number) {
    const drag = dragRef.current;
    dragRef.current = null;
    setGhost(null);
    window.dispatchEvent(new CustomEvent("spill-drag-end"));

    if (!drag?.active) {
      return;
    }

    const { nextColumnId, beforeCardId, clusterCardId } = updateDragTarget(x, y);
    window.dispatchEvent(new CustomEvent("spill-drag-end"));
    if (clusterCardId) {
      const response = await fetch(`/api/retros/${retroId}/cards/${cardId}/cluster`, {
        method: "PATCH",
        body: JSON.stringify({ target_card_id: clusterCardId }),
        headers: { "content-type": "application/json" },
      });
      if (response.ok) {
        router.refresh();
      }
      return;
    }
    if (!nextColumnId || (nextColumnId === columnId && !beforeCardId)) {
      return;
    }
    if (!movingEnabled) {
      return;
    }

    const response = await fetch(`/api/retros/${retroId}/cards/${cardId}/move`, {
      method: "PATCH",
      body: JSON.stringify({ column_id: nextColumnId, before_card_id: beforeCardId === "__end__" ? null : beforeCardId || null }),
      headers: { "content-type": "application/json" },
    });
    if (response.ok) {
      router.refresh();
    }
  }

  return (
    <>
      <article
        className={`transition select-none ${enabled ? "cursor-grab active:cursor-grabbing" : ""} ${ghost ? "opacity-55" : ""} ${clusterTarget ? "scale-[1.01] rounded-[12px] ring-2 ring-white/80" : ""}`}
        data-spill-card-column-id={enabled ? columnId : undefined}
        data-spill-card-id={enabled ? cardId : undefined}
        onPointerDown={(event) => {
          if (!enabled || (event.target instanceof Element && event.target.closest("[data-spill-no-drag]"))) {
            return;
          }
          const rect = event.currentTarget.getBoundingClientRect();
          dragRef.current = {
            pointerId: event.pointerId,
            startX: event.clientX,
            startY: event.clientY,
            width: rect.width,
            active: false,
          };
          event.currentTarget.setPointerCapture(event.pointerId);
        }}
        onPointerMove={(event) => {
          const drag = dragRef.current;
          if (!drag || drag.pointerId !== event.pointerId) {
            return;
          }
          const distance = Math.hypot(event.clientX - drag.startX, event.clientY - drag.startY);
          if (!drag.active && distance < 6) {
            return;
          }
          drag.active = true;
          event.preventDefault();
          setGhost({ x: event.clientX, y: event.clientY, width: drag.width });
          updateDragTarget(event.clientX, event.clientY);
        }}
        onPointerCancel={(event) => {
          if (dragRef.current?.pointerId === event.pointerId) {
            dragRef.current = null;
            setGhost(null);
            window.dispatchEvent(new CustomEvent("spill-drag-end"));
          }
        }}
        onPointerUp={(event) => {
          if (dragRef.current?.pointerId === event.pointerId) {
            event.currentTarget.releasePointerCapture(event.pointerId);
            void finishDrag(event.clientX, event.clientY);
          }
        }}
      >
        {insertBefore ? <InsertionMarker accent={accent} /> : null}
        {children}
      </article>
      {ghost ? (
        <div
          className="pointer-events-none fixed left-0 top-0 z-50 rotate-[-1deg] opacity-70 shadow-[0_18px_34px_-14px_rgba(0,0,0,0.45)]"
          aria-hidden="true"
          data-spill-drag-ghost="true"
          style={{ transform: `translate3d(${ghost.x + 12}px, ${ghost.y + 12}px, 0)`, width: ghost.width }}
        >
          {children}
        </div>
      ) : null}
    </>
  );
}

export function DropEndMarker({ accent, columnId, enabled }: { accent: string; columnId: string; enabled: boolean }) {
  const dropState = useDropMarker(columnId, enabled);
  return dropState.active && dropState.beforeCardId === "__end__" ? <InsertionMarker accent={accent} /> : null;
}

export function InsertionMarker({ accent }: { accent: string }) {
  return (
    <div className="pointer-events-none my-2 flex items-center gap-2 px-1">
      <span className="h-2 w-2 rounded-full shadow-[0_0_0_3px_rgba(255,248,230,0.9)]" style={{ background: accent }} />
      <span className="h-1 flex-1 rounded-full bg-[var(--paper)] shadow-[inset_0_0_0_1px_rgba(74,52,20,0.08),0_1px_4px_rgba(74,52,20,0.12)]" />
      <span className="h-2 w-2 rounded-full shadow-[0_0_0_3px_rgba(255,248,230,0.9)]" style={{ background: accent }} />
    </div>
  );
}

function insertionBeforeCardId(column: Element, draggedCardId: string, pointerY: number) {
  const cards = [...column.querySelectorAll<HTMLElement>("[data-spill-card-id]")].filter((card) => card.dataset.spillCardId !== draggedCardId);
  for (const card of cards) {
    const rect = card.getBoundingClientRect();
    if (pointerY < rect.top + rect.height / 2) {
      return card.dataset.spillCardId || "";
    }
  }
  return "__end__";
}

function clusteringTargetCardId(target: Element | null, draggedCardId: string, pointerX: number, pointerY: number) {
  const card = target?.closest<HTMLElement>("[data-spill-card-id]");
  if (!card || card.dataset.spillCardId === draggedCardId || card.closest("[data-spill-drag-ghost]")) {
    return cardUnderPointer(draggedCardId, pointerX, pointerY);
  }
  return card.dataset.spillCardId || "";
}

function cardUnderPointer(draggedCardId: string, pointerX: number, pointerY: number) {
  const cards = [...document.querySelectorAll<HTMLElement>("[data-spill-card-id]")].filter((card) => {
    if (card.dataset.spillCardId === draggedCardId || card.closest("[data-spill-drag-ghost]")) {
      return false;
    }
    const rect = card.getBoundingClientRect();
    return pointerX >= rect.left && pointerX <= rect.right && pointerY >= rect.top && pointerY <= rect.bottom;
  });
  return cards.at(-1)?.dataset.spillCardId || "";
}
