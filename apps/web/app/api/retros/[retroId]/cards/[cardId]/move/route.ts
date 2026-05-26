import { NextResponse } from "next/server";
import { moveDraftCard } from "../../../../../../lib/api";

export async function PATCH(
  request: Request,
  { params }: { params: Promise<{ retroId: string; cardId: string }> },
) {
  const { retroId, cardId } = await params;
  const body = await request.json();
  const columnId = typeof body?.column_id === "string" ? body.column_id : "";
  const beforeCardId = typeof body?.before_card_id === "string" ? body.before_card_id : undefined;

  if (!columnId) {
    return NextResponse.json({ error: "column_id is required" }, { status: 400 });
  }

  const card = await moveDraftCard(retroId, cardId, columnId, beforeCardId);
  return NextResponse.json(card);
}
