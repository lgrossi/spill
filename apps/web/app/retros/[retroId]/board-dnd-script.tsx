export function BoardDndScript({ retroId }: { retroId: string }) {
  const script = `
(() => {
  const retroId = ${JSON.stringify(retroId)};
  if (window.__spillBoardDndRetroId === retroId) return;
  window.__spillBoardDndRetroId = retroId;

  document.addEventListener("mousedown", (event) => {
    if (event.target instanceof Element && event.target.closest("[data-spill-no-drag]")) return;
    const card = event.target instanceof Element ? event.target.closest("[data-spill-card-id]") : null;
    const cardId = card?.getAttribute("data-spill-card-id") || "";
    const fromColumnId = card?.getAttribute("data-spill-card-column-id") || "";
    if (!cardId || !fromColumnId) return;

    const onMouseUp = async (upEvent) => {
      document.removeEventListener("mouseup", onMouseUp);
      const target = document.elementFromPoint(upEvent.clientX, upEvent.clientY);
      const column = target instanceof Element ? target.closest("[data-spill-column-id]") : null;
      const nextColumnId = column?.getAttribute("data-spill-column-id") || "";
      if (!nextColumnId || nextColumnId === fromColumnId) return;

      const response = await fetch("/api/retros/" + retroId + "/cards/" + cardId + "/move", {
        method: "PATCH",
        body: JSON.stringify({ column_id: nextColumnId }),
        headers: { "content-type": "application/json" },
      });
      if (response.ok) window.location.reload();
    };

    document.addEventListener("mouseup", onMouseUp);
  });
})();
`;

  return <script dangerouslySetInnerHTML={{ __html: script }} />;
}
