import { computeComposition } from "@/lib/composition";

export const dynamic = "force-dynamic";

export async function GET() {
  return Response.json(computeComposition());
}
