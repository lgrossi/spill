"use server";

import { redirect } from "next/navigation";
import { createRetro, type CreateRetroPayload } from "../api";

export async function createRetroCommand(formData: FormData) {
  const template = String(formData.get("template") ?? "standard");
  const title = String(formData.get("title") ?? "").trim();
  const votingEnabled = formData.getAll("voting_enabled").at(-1) !== "0";
  const voteLimit = votingEnabled ? Number(formData.get("vote_limit") ?? 3) : 0;
  const actionDiscussionEnabled = formData.getAll("action_discussion_enabled").at(-1) === "1";
  const actionDiscussionLimit = actionDiscussionEnabled ? Number(formData.get("action_discussion_limit") ?? 3) : 0;
  const customColumns = formData
    .getAll("custom_column")
    .map((column) => String(column).trim())
    .filter(Boolean);
  const customColumnColors = formData.getAll("custom_column_color").map((color) => String(color).trim());
  const invitees = formData
    .getAll("invitee")
    .map((v) => String(v).trim().toLowerCase())
    .filter((v) => v.includes("@"));

  const payload = retroPayload({
    actionDiscussionEnabled,
    actionDiscussionLimit,
    customColumnColors,
    customColumns,
    template,
    title,
    voteLimit,
    invitees,
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
  voteLimit,
  invitees,
}: {
  actionDiscussionEnabled: boolean;
  actionDiscussionLimit: number;
  customColumnColors: string[];
  customColumns: string[];
  template: string;
  title: string;
  voteLimit: number;
  invitees: string[];
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
    return customPayload(title, ["Wind", "Anchor", "Rocks", "Island"], undefined, voteLimit, actionDiscussionLimit, invitees);
  }
  if (template === "ssc") {
    return customPayload(title, ["Start", "Stop", "Continue"], undefined, voteLimit, actionDiscussionLimit, invitees);
  }
  if (template === "msg") {
    return customPayload(title, ["Mad", "Sad", "Glad"], ["#cf4f4f", "#cf4f4f", "#2f9469"], voteLimit, actionDiscussionLimit, invitees);
  }
  if (template === "4ls") {
    return customPayload(title, fourLs.columns, fourLs.colors, voteLimit, actionDiscussionLimit, invitees);
  }
  if (template === "custom") {
    return customPayload(title, custom.columns, custom.colors, voteLimit, actionDiscussionLimit, invitees);
  }
  return customPayload(title, standard.columns, standard.colors, voteLimit, actionDiscussionLimit, invitees);
}

function customPayload(
  title: string,
  columns: string[],
  columnColors: string[] | undefined,
  voteLimit: number,
  actionDiscussionLimit: number,
  invitees: string[],
): CreateRetroPayload {
  return {
    title,
    template: "custom",
    columns,
    column_colors: columnColors,
    vote_limit: voteLimit,
    action_discussion_limit: actionDiscussionLimit,
    invitees,
  };
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
