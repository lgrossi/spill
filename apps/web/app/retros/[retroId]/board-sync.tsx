"use client";

import { useEffect, useRef } from "react";
import { useRouter } from "next/navigation";
import { shouldRefreshBoard } from "./board-sync-policy";

const CONFIGURED_API_BASE_URL = process.env.NEXT_PUBLIC_SPILLIO_API_URL;
const POLL_INTERVAL_MS = 5000;
const RECONNECT_DELAY_MS = 1500;

export function BoardSync({ retroId }: { retroId: string }) {
  const router = useRouter();
  const routerRef = useRef(router);

  routerRef.current = router;

  useEffect(() => {
    let closed = false;
    let socket: WebSocket | null = null;
    let pollTimer: number | null = null;
    let reconnectTimer: number | null = null;

    function stopPolling() {
      if (pollTimer) {
        window.clearInterval(pollTimer);
        pollTimer = null;
      }
    }

    function startPolling() {
      if (!pollTimer) {
        pollTimer = window.setInterval(() => routerRef.current.refresh(), POLL_INTERVAL_MS);
      }
    }

    function connect() {
      if (closed) return;

      socket = new WebSocket(`${toWebSocketUrl(browserApiBaseUrl())}/api/retros/${retroId}/events`);
      socket.addEventListener("open", stopPolling);
      socket.addEventListener("message", (event) => {
        if (shouldRefreshBoard(parseBoardEvent(event.data))) {
          routerRef.current.refresh();
        }
      });
      socket.addEventListener("close", scheduleReconnect);
      socket.addEventListener("error", () => {
        socket?.close();
      });
    }

    function scheduleReconnect() {
      if (closed) return;
      startPolling();
      if (!reconnectTimer) {
        reconnectTimer = window.setTimeout(() => {
          reconnectTimer = null;
          connect();
        }, RECONNECT_DELAY_MS);
      }
    }

    connect();

    return () => {
      closed = true;
      socket?.close();
      stopPolling();
      if (reconnectTimer) {
        window.clearTimeout(reconnectTimer);
      }
    };
  }, [retroId]);

  return null;
}

function browserApiBaseUrl() {
  if (CONFIGURED_API_BASE_URL) {
    return CONFIGURED_API_BASE_URL;
  }

  const url = new URL(window.location.href);
  url.pathname = "";
  url.search = "";
  url.hash = "";
  url.port = "4000";
  return url.toString().replace(/\/$/, "");
}

function toWebSocketUrl(baseUrl: string) {
  const url = new URL(baseUrl);
  url.protocol = url.protocol === "https:" ? "wss:" : "ws:";
  return url.toString().replace(/\/$/, "");
}

function parseBoardEvent(value: string) {
  try {
    return JSON.parse(value) as { type?: string };
  } catch {
    return {};
  }
}
