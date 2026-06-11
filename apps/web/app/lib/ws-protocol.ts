// Browser-safe constant (no Node deps) shared by the client board-sync and the
// server-only token minter. Keep this free of any `crypto`/Node imports.
export const WS_SUBPROTOCOL = "spillio.ws.v1";
