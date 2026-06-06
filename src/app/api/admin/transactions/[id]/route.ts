import { deleteTransaction } from "@/db/queries/transactions";

export const dynamic = "force-dynamic";

export async function DELETE(
  _req: Request,
  { params }: { params: Promise<{ id: string }> },
) {
  const { id } = await params;
  const n = Number(id);
  if (!Number.isInteger(n)) {
    return Response.json({ error: "invalid id" }, { status: 400 });
  }
  const deleted = deleteTransaction(n);
  return Response.json({ deleted });
}
