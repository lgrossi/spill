import { NextResponse } from "next/server";
import { moveBoardCard } from "../../../../../../lib/board-commands";

export async function PATCH(
  request: Request,
  { params }: { params: Promise<{ retroId: string; cardId: string }> },
) {
  const { retroId, cardId } = await params;
  const body = await request.json();
  const result = await moveBoardCard(retroId, cardId, body);

  if (!result.ok) {
    return NextResponse.json({ error: result.error }, { status: 400 });
  }

  return NextResponse.json(result.value);
}
