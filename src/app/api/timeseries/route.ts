import { computeTimeseries } from "@/lib/timeseries";

export const dynamic = "force-dynamic";

export async function GET() {
  return Response.json(computeTimeseries());
}
