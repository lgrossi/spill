"use server";

import { revalidatePath } from "next/cache";
import { retryAiJob } from "@/lib/api";
import { field } from "./form-utils";

export async function retryAiJobCommand(formData: FormData) {
  const retroId = field(formData, "retro_id");
  const artifactId = field(formData, "artifact_id");

  await retryAiJob(retroId, artifactId);
  revalidatePath(`/retros/${retroId}`);
}
