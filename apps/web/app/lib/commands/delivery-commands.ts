"use server";

import { revalidatePath } from "next/cache";
import { createDelivery, createMeetingNote, retryDelivery, type Delivery } from "../api";

export async function createMeetingNoteCommand(formData: FormData) {
  const retroId = field(formData, "retro_id");
  const title = field(formData, "title").trim();
  const bodyText = field(formData, "body_text").trim();

  await createMeetingNote(retroId, title, bodyText);
  revalidatePath(`/retros/${retroId}`);
}

export async function createDeliveryCommand(formData: FormData) {
  const retroId = field(formData, "retro_id");
  const kind = field(formData, "kind") as Delivery["kind"];
  const fail = formData.get("fail") === "on";

  await createDelivery(retroId, kind, fail);
  revalidatePath(`/retros/${retroId}`);
}

export async function retryDeliveryCommand(formData: FormData) {
  const retroId = field(formData, "retro_id");
  const deliveryId = field(formData, "delivery_id");

  await retryDelivery(retroId, deliveryId);
  revalidatePath(`/retros/${retroId}`);
}

function field(formData: FormData, name: string) {
  return String(formData.get(name) ?? "");
}
