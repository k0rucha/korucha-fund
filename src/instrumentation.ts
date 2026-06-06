// Runs once on server startup (Node runtime only). Starts the daily scheduler.
// Suits a single always-on instance — the same model as the old single binary.
export async function register() {
  if (process.env.NEXT_RUNTIME === "nodejs") {
    const { startScheduler } = await import("@/lib/scheduler");
    startScheduler();
  }
}
