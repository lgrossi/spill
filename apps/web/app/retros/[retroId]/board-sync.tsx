"use client";

import { useEffect } from "react";
import { useRouter } from "next/navigation";

const API_BASE_URL = process.env.NEXT_PUBLIC_SPILLIO_API_URL ?? "http://127.0.0.1:4000";

export function BoardSync({ retroId }: { retroId: string }) {
  const router = useRouter();

  useEffect(() => {
    const socket = new WebSocket(`${toWebSocketUrl(API_BASE_URL)}/api/retros/${retroId}/events`);

    socket.addEventListener("message", () => {
      router.refresh();
    });

    return () => {
      socket.close();
    };
  }, [retroId, router]);

  return null;
}

function toWebSocketUrl(baseUrl: string) {
  const url = new URL(baseUrl);
  url.protocol = url.protocol === "https:" ? "wss:" : "ws:";
  return url.toString().replace(/\/$/, "");
}
