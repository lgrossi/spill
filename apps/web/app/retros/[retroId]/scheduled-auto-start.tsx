"use client";

import { useEffect, useRef } from "react";
import { startScheduledRetroAction } from "@/lib/actions";

export function ScheduledAutoStart({ retroId }: { retroId: string }) {
  const formRef = useRef<HTMLFormElement | null>(null);

  useEffect(() => {
    formRef.current?.requestSubmit();
  }, []);

  return (
    <form action={startScheduledRetroAction} ref={formRef} className="contents">
      <input name="retro_id" type="hidden" value={retroId} />
    </form>
  );
}
