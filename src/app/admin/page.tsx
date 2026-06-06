import { listTransactions } from "@/db/queries/transactions";
import { getSymbolNames } from "@/db/queries/symbols";
import Header from "@/components/Header";
import AdminClient from "@/components/admin/AdminClient";

// Behind Basic auth (see src/middleware.ts). Always fresh.
export const dynamic = "force-dynamic";

export default async function AdminPage() {
  const transactions = listTransactions();
  const symbolNames: Record<string, string> = {};
  for (const s of getSymbolNames()) {
    if (s.name) symbolNames[s.symbol] = s.name;
  }

  return (
    <main className="mx-auto max-w-5xl p-6">
      <Header />
      <AdminClient transactions={transactions} symbolNames={symbolNames} />
    </main>
  );
}
