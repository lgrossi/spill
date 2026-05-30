"use server";

import { redirect } from "next/navigation";
import { cloneRetro, createRetro, updateRetroMetadata, type CreateRetroPayload } from "@/lib/api";
import type { InviteeRequest } from "@/lib/contracts";

export async function createRetroCommand(formData: FormData) {
  const template = String(formData.get("template") ?? "standard");
  const title = String(formData.get("title") ?? "").trim();
  const scheduledAt = String(formData.get("scheduled_at") ?? "").trim();
  const votingEnabled = formData.getAll("voting_enabled").at(-1) !== "0";
  const voteLimit = votingEnabled ? Number(formData.get("vote_limit") ?? 3) : 0;
  const actionDiscussionEnabled = formData.getAll("action_discussion_enabled").at(-1) === "1";
  const actionDiscussionLimit = actionDiscussionEnabled ? Number(formData.get("action_discussion_limit") ?? 3) : 0;
  const clusteringMode = String(formData.get("clustering_mode") ?? "disabled");
  const customColumns = formData
    .getAll("custom_column")
    .map((column) => String(column).trim())
    .filter(Boolean);
  const customColumnColors = formData.getAll("custom_column_color").map((color) => String(color).trim());
  const inviteeEmails = formData
    .getAll("invitee_email")
    .map((v) => String(v).trim().toLowerCase());
  const inviteeRoles = formData.getAll("invitee_role").map((v) => String(v).trim());
  const invitees: InviteeRequest[] = inviteeEmails
    .filter((v) => v.includes("@"))
    .map((email, i) => ({
      email,
      role: inviteeRoles[i] === "host" ? "host" : "member",
    }));

  const payload = retroPayload({
    actionDiscussionEnabled,
    actionDiscussionLimit,
    customColumnColors,
    customColumns,
    template,
    title,
    scheduledAt,
    voteLimit,
    invitees,
    clusteringMode,
  });

  const board = await createRetro(payload);
  redirect(`/retros/${board.retro.id}`);
}

function retroPayload({
  actionDiscussionEnabled,
  actionDiscussionLimit,
  customColumnColors,
  customColumns,
  template,
  title,
  scheduledAt,
  voteLimit,
  invitees,
  clusteringMode,
}: {
  actionDiscussionEnabled: boolean;
  actionDiscussionLimit: number;
  customColumnColors: string[];
  customColumns: string[];
  template: string;
  title: string;
  scheduledAt: string;
  voteLimit: number;
  invitees: InviteeRequest[];
  clusteringMode: string;
}): CreateRetroPayload {
  const standard = withActionColumn({
    actionDiscussionEnabled,
    colors: ["#0f5f72", "#2f9469", "#cf4f4f"],
    columns: ["How are you feeling?", "Went well", "To improve"],
  });
  const fourLs = withActionColumn({
    actionDiscussionEnabled,
    colors: ["#2f9469", "#cf4f4f", "#0f5f72", "#cf8a3f"],
    columns: ["Liked", "Lacked", "Learned", "Longed for"],
  });
  const custom = withActionColumn({
    actionDiscussionEnabled,
    colors: customColumnColors,
    columns: customColumns,
  });

  if (template === "sailboat") {
    return customPayload(title, scheduledAt, ["Wind", "Anchor", "Rocks", "Island"], undefined, voteLimit, actionDiscussionLimit, invitees, clusteringMode);
  }
  if (template === "ssc") {
    return customPayload(title, scheduledAt, ["Start", "Stop", "Continue"], undefined, voteLimit, actionDiscussionLimit, invitees, clusteringMode);
  }
  if (template === "msg") {
    return customPayload(title, scheduledAt, ["Mad", "Sad", "Glad"], ["#cf4f4f", "#cf4f4f", "#2f9469"], voteLimit, actionDiscussionLimit, invitees, clusteringMode);
  }
  if (template === "4ls") {
    return customPayload(title, scheduledAt, fourLs.columns, fourLs.colors, voteLimit, actionDiscussionLimit, invitees, clusteringMode);
  }
  if (template === "custom") {
    return customPayload(title, scheduledAt, custom.columns, custom.colors, voteLimit, actionDiscussionLimit, invitees, clusteringMode);
  }
  return customPayload(title, scheduledAt, standard.columns, standard.colors, voteLimit, actionDiscussionLimit, invitees, clusteringMode);
}

function customPayload(
  title: string,
  scheduledAt: string,
  columns: string[],
  columnColors: string[] | undefined,
  voteLimit: number,
  actionDiscussionLimit: number,
  invitees: InviteeRequest[],
  clusteringMode: string,
): CreateRetroPayload {
  return {
    title,
    scheduled_at: scheduledAt || null,
    template: "custom",
    columns,
    column_colors: columnColors,
    vote_limit: voteLimit,
    action_discussion_limit: actionDiscussionLimit,
    clustering_mode: clusteringMode === "auto_on_vote_start" ? "auto_on_vote_start" : "disabled",
    invitees,
  };
}

export async function updateRetroMetadataCommand(formData: FormData) {
  const retroId = String(formData.get("retro_id") ?? "");
  const title = String(formData.get("title") ?? "").trim();
  const scheduledAt = String(formData.get("scheduled_at") ?? "").trim();
  const coverGifUrl = String(formData.get("cover_gif_url") ?? "").trim();
  const coverGifAltText = String(formData.get("cover_gif_alt_text") ?? "").trim();
  if (!retroId || !title) return;
  await updateRetroMetadata(retroId, title, scheduledAt, coverGifUrl, coverGifAltText);
}

export async function cloneRetroCommand(formData: FormData) {
  const sourceRetroId = String(formData.get("source_retro_id") ?? "");
  const title = String(formData.get("title") ?? "").trim();
  const scheduledAt = String(formData.get("scheduled_at") ?? "").trim();
  const suggestTitle = formData.get("suggest_title") === "1";
  if (!sourceRetroId) return;
  const board = await cloneRetro(sourceRetroId, title, scheduledAt, suggestTitle);
  redirect(`/retros/${board.retro.id}`);
}

function withActionColumn({
  actionDiscussionEnabled,
  colors,
  columns,
}: {
  actionDiscussionEnabled: boolean;
  colors: string[];
  columns: string[];
}) {
  if (!actionDiscussionEnabled) {
    return { columns, colors };
  }
  const pairs = columns
    .map((column, index) => ({ column, color: colors[index] ?? "#cf8a3f" }))
    .filter((item) => item.column.toLowerCase() !== "actions");
  return {
    columns: [...pairs.map((item) => item.column), "Actions"],
    colors: [...pairs.map((item) => item.color), "#8757b6"],
  };
}
