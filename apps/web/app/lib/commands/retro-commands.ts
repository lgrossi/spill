"use server";

import { redirect } from "next/navigation";
import { createRetro, rescheduleRetro, updateRetroDetails, type CreateRetroPayload } from "@/lib/api";
import type { InviteeRequest, UpdateRetroDetailsPayload } from "@/lib/contracts";
import { field } from "./form-utils";

export async function createRetroCommand(formData: FormData) {
  const template = String(formData.get("template") ?? "standard");
  const title = String(formData.get("title") ?? "").trim();
  const groupName = String(formData.get("group_name") ?? "").trim();
  const coverGifUrl = String(formData.get("cover_gif_url") ?? "").trim();
  const coverGifAltText = String(formData.get("cover_gif_alt_text") ?? "").trim();
  const plannedFor = String(formData.get("planned_for") ?? "").trim();
  const votingEnabled = formData.getAll("voting_enabled").at(-1) !== "0";
  const voteLimit = votingEnabled ? enabledLimit(formData, "vote_limit", 3) : 0;
  const actionDiscussionEnabled = formData.getAll("action_discussion_enabled").at(-1) === "1";
  const actionDiscussionLimit = actionDiscussionEnabled ? enabledLimit(formData, "action_discussion_limit", 3) : 0;
  const clusteringMode = String(formData.getAll("clustering_mode").at(-1) ?? "disabled");
  // The two privacy toggles default to off when their hidden marker isn't
  // present (e.g. an older form that doesn't render the tiles).
  const cardEditPolicy =
    formData.getAll("card_edit_policy").at(-1) === "author_only"
      ? "author_only"
      : "collaborative";
  const anonymousAuthors = formData.getAll("anonymous_authors").at(-1) === "1";
  // Default to per_column when the form field isn't present (e.g. forms
  // submitted before the toggle existed): matches the create form's
  // ship-checked default for new boards. Explicit "big_bang" wins when
  // sent.
  const revealMode: "per_column" | "big_bang" =
    formData.getAll("reveal_mode").at(-1) === "big_bang" ? "big_bang" : "per_column";
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
    coverGifUrl,
    coverGifAltText,
    plannedFor,
    voteLimit,
    invitees,
    clusteringMode,
    cardEditPolicy,
    anonymousAuthors,
    revealMode,
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
  const coverGifUrl = String(formData.get("cover_gif_url") ?? "").trim();
  const coverGifAltText = String(formData.get("cover_gif_alt_text") ?? "").trim();
  const removeCoverGif = String(formData.get("remove_cover_gif") ?? "") === "1";
  const returnTo = String(formData.get("return_to") ?? `/retros/${retroId}`);

  const payload: UpdateRetroDetailsPayload = {
    ...(title ? { title } : {}),
    ...(groupName ? { group_name: groupName } : {}),
    ...(coverGifUrl ? { cover_gif_url: coverGifUrl, cover_gif_alt_text: coverGifAltText || null } : {}),
    ...(removeCoverGif ? { remove_cover_gif: true } : {}),
  };

  // Each config control submits a hidden enable-marker that is always present
  // when its tile renders, so a missing marker means "leave this config alone"
  // (e.g. inline title edits, or the actions tile hidden when no actions column).
  if (formData.has("voting_enabled")) {
    const votingEnabled = formData.getAll("voting_enabled").at(-1) !== "0";
    payload.vote_limit = votingEnabled ? enabledLimit(formData, "vote_limit", 3) : 0;
  }
  if (formData.has("action_discussion_enabled")) {
    const topVotedToActions = formData.getAll("action_discussion_enabled").at(-1) !== "0";
    payload.action_discussion_limit = topVotedToActions
      ? enabledLimit(formData, "action_discussion_limit", 3)
      : 0;
  }
  if (formData.has("clustering_mode")) {
    payload.clustering_mode =
      String(formData.getAll("clustering_mode").at(-1) ?? "disabled") === "auto_on_vote_start"
        ? "auto_on_vote_start"
        : "disabled";
  }
  if (formData.has("card_edit_policy")) {
    payload.card_edit_policy =
      String(formData.getAll("card_edit_policy").at(-1) ?? "collaborative") === "author_only"
        ? "author_only"
        : "collaborative";
  }
  if (formData.has("anonymous_authors")) {
    payload.anonymous_authors = formData.getAll("anonymous_authors").at(-1) === "1";
  }
  if (formData.has("reveal_mode")) {
    payload.reveal_mode =
      formData.getAll("reveal_mode").at(-1) === "big_bang" ? "big_bang" : "per_column";
  }

  await updateRetroDetails(retroId, payload);
  redirect(returnTo.startsWith("/") && !returnTo.startsWith("//") ? returnTo : `/retros/${retroId}`);
}

function enabledLimit(formData: FormData, name: string, fallback: number) {
  const raw = String(formData.get(name) ?? "").trim();
  const value = raw ? Number(raw) : fallback;
  return Math.max(1, Number.isFinite(value) ? value : fallback);
}

function retroPayload({
  actionDiscussionEnabled,
  actionDiscussionLimit,
  customColumnColors,
  customColumns,
  template,
  title,
  groupName,
  coverGifUrl,
  coverGifAltText,
  plannedFor,
  voteLimit,
  invitees,
  clusteringMode,
  cardEditPolicy,
  anonymousAuthors,
  revealMode,
}: {
  actionDiscussionEnabled: boolean;
  actionDiscussionLimit: number;
  customColumnColors: string[];
  customColumns: string[];
  template: string;
  title: string;
  groupName: string;
  coverGifUrl: string;
  coverGifAltText: string;
  plannedFor: string;
  voteLimit: number;
  invitees: InviteeRequest[];
  clusteringMode: string;
  cardEditPolicy: "collaborative" | "author_only";
  anonymousAuthors: boolean;
  revealMode: "per_column" | "big_bang";
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
    return customPayload(title, groupName, coverGifUrl, coverGifAltText, plannedFor, ["Wind", "Anchor", "Rocks", "Island"], undefined, voteLimit, actionDiscussionLimit, invitees, clusteringMode, cardEditPolicy, anonymousAuthors, revealMode);
  }
  if (template === "ssc") {
    return customPayload(title, groupName, coverGifUrl, coverGifAltText, plannedFor, ["Start", "Stop", "Continue"], undefined, voteLimit, actionDiscussionLimit, invitees, clusteringMode, cardEditPolicy, anonymousAuthors, revealMode);
  }
  if (template === "msg") {
    return customPayload(title, groupName, coverGifUrl, coverGifAltText, plannedFor, ["Mad", "Sad", "Glad"], ["#cf4f4f", "#cf4f4f", "#2f9469"], voteLimit, actionDiscussionLimit, invitees, clusteringMode, cardEditPolicy, anonymousAuthors, revealMode);
  }
  if (template === "4ls") {
    return customPayload(title, groupName, coverGifUrl, coverGifAltText, plannedFor, fourLs.columns, fourLs.colors, voteLimit, actionDiscussionLimit, invitees, clusteringMode, cardEditPolicy, anonymousAuthors, revealMode);
  }
  if (template === "custom") {
    return customPayload(title, groupName, coverGifUrl, coverGifAltText, plannedFor, custom.columns, custom.colors, voteLimit, actionDiscussionLimit, invitees, clusteringMode, cardEditPolicy, anonymousAuthors, revealMode);
  }
  return customPayload(title, groupName, coverGifUrl, coverGifAltText, plannedFor, standard.columns, standard.colors, voteLimit, actionDiscussionLimit, invitees, clusteringMode, cardEditPolicy, anonymousAuthors, revealMode);
}

function customPayload(
  title: string,
  groupName: string,
  coverGifUrl: string,
  coverGifAltText: string,
  plannedFor: string,
  columns: string[],
  columnColors: string[] | undefined,
  voteLimit: number,
  actionDiscussionLimit: number,
  invitees: InviteeRequest[],
  clusteringMode: string,
  cardEditPolicy: "collaborative" | "author_only",
  anonymousAuthors: boolean,
  revealMode: "per_column" | "big_bang",
): CreateRetroPayload {
  return {
    title,
    group_name: groupName || null,
    cover_gif_url: coverGifUrl || null,
    cover_gif_alt_text: coverGifAltText || null,
    planned_for: plannedFor || null,
    template: "custom",
    columns,
    column_colors: columnColors,
    vote_limit: voteLimit,
    action_discussion_limit: actionDiscussionLimit,
    clustering_mode: clusteringMode === "auto_on_vote_start" ? "auto_on_vote_start" : "disabled",
    // Only include privacy/flow fields when they diverge from defaults so we
    // don't expand serialized payload size unnecessarily — the server treats
    // missing values as the per-field default (collaborative card edits,
    // visible authors, per_column reveal).
    ...(cardEditPolicy === "author_only" ? { card_edit_policy: "author_only" as const } : {}),
    ...(anonymousAuthors ? { anonymous_authors: true as const } : {}),
    ...(revealMode === "big_bang" ? { reveal_mode: "big_bang" as const } : {}),
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
