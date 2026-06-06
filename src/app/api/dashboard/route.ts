import { computeDashboard } from "@/lib/dashboard";

export const dynamic = "force-dynamic";

export async function GET() {
  return Response.json(computeDashboard());
}
