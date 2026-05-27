import { NextResponse } from "next/server";
import { moveBoardCard } from "../../../../../../lib/board-commands";
import { assertSameOrigin } from "../../../../../../lib/csrf";

export async function PATCH(
  request: Request,
  { params }: { params: Promise<{ retroId: string; cardId: string }> },
) {
  const { retroId, cardId } = await params;

  const csrf = assertSameOrigin(request);
  if (!csrf.ok) {
    return NextResponse.json({ error: csrf.reason }, { status: 403 });
  }

  const body = await request.json();
  const result = await moveBoardCard(retroId, cardId, body);

  if (!result.ok) {
    return NextResponse.json({ error: result.error }, { status: 400 });
  }

  return NextResponse.json(result.value);
}
