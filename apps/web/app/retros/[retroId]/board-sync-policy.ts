type BoardEvent = {
  type?: string;
};

export function shouldRefreshBoard(event: BoardEvent) {
  return event.type === "board_snapshot"
    || event.type === "card_changed"
    || event.type === "ready_changed"
    || event.type === "phase_changed"
    || event.type === "clustering_changed";
}
