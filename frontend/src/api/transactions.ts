import { request, authHeader } from "./client";

export interface Transaction {
  id: number;
  symbol: string;
  txn_type: "BUY" | "SELL";
  quantity: number;
  price: number;
  currency: "JPY" | "USD";
  fee: number;
  txn_date: string;
  fx_rate_to_jpy: number | null;
  notes: string | null;
  created_at: string;
  symbol_name: string | null;
}

export interface TransactionCreate {
  symbol: string;
  txn_type: "BUY" | "SELL";
  quantity: number;
  price: number;
  currency: "JPY" | "USD";
  fee: number;
  txn_date: string;
  fx_rate_to_jpy?: number | null;
  notes?: string | null;
}

export interface ImportResult {
  imported: number;
  skipped: number;
}

function adminHeaders(user: string, pass: string) {
  return authHeader(user, pass);
}

export const getTransactions = (user: string, pass: string) =>
  request<Transaction[]>("/admin/transactions", {
    headers: adminHeaders(user, pass),
  });

export const addTransaction = (data: TransactionCreate, user: string, pass: string) =>
  request<Transaction>("/admin/transactions", {
    method: "POST",
    headers: adminHeaders(user, pass),
    body: JSON.stringify(data),
  });

export const deleteTransaction = (id: number, user: string, pass: string) =>
  fetch(`/api/admin/transactions/${id}`, {
    method: "DELETE",
    headers: adminHeaders(user, pass),
  }).then((r) => {
    if (!r.ok) throw new Error("Delete failed");
  });

export const exportTransactions = (user: string, pass: string) => {
  const headers = adminHeaders(user, pass);
  return fetch("/api/admin/transactions/export", { headers }).then((r) => {
    if (!r.ok) throw new Error("Export failed");
    return r.blob();
  });
};

export const importTransactions = (file: File, user: string, pass: string): Promise<ImportResult> => {
  const form = new FormData();
  form.append("file", file);
  return fetch("/api/admin/transactions/import", {
    method: "POST",
    headers: adminHeaders(user, pass),
    body: form,
  }).then(async (r) => {
    if (!r.ok) {
      const err = await r.json().catch(() => ({ detail: "Import failed" }));
      throw new Error(err.detail);
    }
    return r.json();
  });
};
