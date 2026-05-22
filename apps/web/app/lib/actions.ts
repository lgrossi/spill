"use server";

import { redirect } from "next/navigation";
import { createRetro, type CreateRetroPayload } from "./api";

export async function createRetroAction(formData: FormData) {
  const template = String(formData.get("template") ?? "standard");
  const title = String(formData.get("title") ?? "").trim();
  const voteLimit = Number(formData.get("vote_limit") ?? 3);
  const actionDiscussionLimit = Number(formData.get("action_discussion_limit") ?? 3);

  const payload: CreateRetroPayload =
    template === "custom"
      ? {
          title,
          template: "custom",
          columns: String(formData.get("columns") ?? "")
            .split("\n")
            .map((column) => column.trim())
            .filter(Boolean),
          vote_limit: voteLimit,
          action_discussion_limit: actionDiscussionLimit,
        }
      : {
          title,
          template: "standard",
          vote_limit: voteLimit,
          action_discussion_limit: actionDiscussionLimit,
        };

  const board = await createRetro(payload);
  redirect(`/retros/${board.retro.id}`);
}
