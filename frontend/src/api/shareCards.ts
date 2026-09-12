import { request } from "./client";

export interface CardHolding {
  symbol: string;
  name: string | null;
  quantity: number;
  value_jpy: number;
}

export interface ShareCardCreated {
  id: string;
  url: string;
}

export interface ShareCardRead {
  id: string;
  created_at: string;
  total_value_jpy: number;
  total_cost_jpy: number;
  unrealized_pnl_jpy: number;
  default_span: string;
  holdings: CardHolding[];
  current_value_jpy: number | null;
  value_delta_jpy: number | null;
  history_dates: string[];
  history_values: number[];
  history_pnls: number[];
}

export interface TickerShareCardCreated {
  id: string;
  url: string;
}

export interface TickerShareCardRead {
  id: string;
  created_at: string;
  symbol: string;
  display_name: string | null;
  currency: string;
  issue_price_native: number;
  fx_rate_at_issue: number | null;
  quantity: number | null;
  avg_cost_native: number | null;
  issue_value_jpy: number | null;
  issue_pnl_jpy: number | null;
  default_span: string;
  current_price_native: number | null;
  price_delta_native: number | null;
  history_dates: string[];
  history_prices: number[];
}

export interface StatusResponse {
  request_count: number;
  max_requests_per_day: number;
  remaining_requests: number;
  last_request_time: string | null;
  next_scheduler_run: string | null;
}

export const createShareCard = (span: string) =>
  request<ShareCardCreated>("/share-cards", {
    method: "POST",
    body: JSON.stringify({ span }),
  });

export const getShareCard = (id: string) =>
  request<ShareCardRead>(`/share-cards/${id}`);

export const createTickerShareCard = (symbol: string, span: string) =>
  request<TickerShareCardCreated>("/ticker-share-cards", {
    method: "POST",
    body: JSON.stringify({ symbol, span }),
  });

export const getTickerShareCard = (id: string) =>
  request<TickerShareCardRead>(`/ticker-share-cards/${id}`);

export const getStatus = () => request<StatusResponse>("/status");
