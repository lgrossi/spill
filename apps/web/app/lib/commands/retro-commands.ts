"use server";

import { redirect } from "next/navigation";
import { createRetro, rescheduleRetro, updateRetroDetails, type CreateRetroPayload } from "@/lib/api";
import type { InviteeRequest } from "@/lib/contracts";
import { field } from "./form-utils";

export async function createRetroCommand(formData: FormData) {
  const template = String(formData.get("template") ?? "standard");
  const title = String(formData.get("title") ?? "").trim();
  const groupName = String(formData.get("group_name") ?? "").trim();
  const plannedFor = String(formData.get("planned_for") ?? "").trim();
  const votingEnabled = formData.getAll("voting_enabled").at(-1) !== "0";
  const voteLimit = votingEnabled ? Number(formData.get("vote_limit") ?? 3) : 0;
  const actionDiscussionEnabled = formData.getAll("action_discussion_enabled").at(-1) === "1";
  const actionDiscussionLimit = actionDiscussionEnabled ? Number(formData.get("action_discussion_limit") ?? 3) : 0;
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
    groupName,
    plannedFor,
    voteLimit,
    invitees,
  });

  const board = await createRetro(payload);
  redirect(`/retros/${board.retro.id}`);
}

export async function rescheduleRetroCommand(formData: FormData) {
  const retroId = field(formData, "retro_id");
  const plannedFor = field(formData, "planned_for").trim();

  await rescheduleRetro(retroId, { planned_for: plannedFor || null });
  redirect(`/retros/${retroId}`);
}

export async function updateRetroDetailsCommand(formData: FormData) {
  const retroId = field(formData, "retro_id");
  const title = String(formData.get("title") ?? "").trim();
  const groupName = String(formData.get("group_name") ?? "").trim();
  const returnTo = String(formData.get("return_to") ?? `/retros/${retroId}`);

  await updateRetroDetails(retroId, {
    ...(title ? { title } : {}),
    ...(groupName ? { group_name: groupName } : {}),
  });
  redirect(returnTo.startsWith("/") && !returnTo.startsWith("//") ? returnTo : `/retros/${retroId}`);
}

function retroPayload({
  actionDiscussionEnabled,
  actionDiscussionLimit,
  customColumnColors,
  customColumns,
  template,
  title,
  groupName,
  plannedFor,
  voteLimit,
  invitees,
}: {
  actionDiscussionEnabled: boolean;
  actionDiscussionLimit: number;
  customColumnColors: string[];
  customColumns: string[];
  template: string;
  title: string;
  groupName: string;
  plannedFor: string;
  voteLimit: number;
  invitees: InviteeRequest[];
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
    return customPayload(title, groupName, plannedFor, ["Wind", "Anchor", "Rocks", "Island"], undefined, voteLimit, actionDiscussionLimit, invitees);
  }
  if (template === "ssc") {
    return customPayload(title, groupName, plannedFor, ["Start", "Stop", "Continue"], undefined, voteLimit, actionDiscussionLimit, invitees);
  }
  if (template === "msg") {
    return customPayload(title, groupName, plannedFor, ["Mad", "Sad", "Glad"], ["#cf4f4f", "#cf4f4f", "#2f9469"], voteLimit, actionDiscussionLimit, invitees);
  }
  if (template === "4ls") {
    return customPayload(title, groupName, plannedFor, fourLs.columns, fourLs.colors, voteLimit, actionDiscussionLimit, invitees);
  }
  if (template === "custom") {
    return customPayload(title, groupName, plannedFor, custom.columns, custom.colors, voteLimit, actionDiscussionLimit, invitees);
  }
  return customPayload(title, groupName, plannedFor, standard.columns, standard.colors, voteLimit, actionDiscussionLimit, invitees);
}

function customPayload(
  title: string,
  groupName: string,
  plannedFor: string,
  columns: string[],
  columnColors: string[] | undefined,
  voteLimit: number,
  actionDiscussionLimit: number,
  invitees: InviteeRequest[],
): CreateRetroPayload {
  return {
    title,
    group_name: groupName || null,
    planned_for: plannedFor || null,
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
