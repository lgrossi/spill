import { NextResponse } from "next/server";
import { clusterCards } from "../../../../../../lib/api";

export async function PATCH(
  request: Request,
  { params }: { params: Promise<{ retroId: string; cardId: string }> },
) {
  const { retroId, cardId } = await params;
  const body = await request.json();
  const targetCardId = typeof body?.target_card_id === "string" ? body.target_card_id : "";

  if (!targetCardId) {
    return NextResponse.json({ error: "target_card_id is required" }, { status: 400 });
  }

  const cluster = await clusterCards(retroId, cardId, targetCardId);
  return NextResponse.json(cluster);
}
