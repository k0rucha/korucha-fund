import { request } from "./client";

export interface HoldingView {
  symbol: string;
  name: string | null;
  quantity: number;
  avg_cost_native: number;
  currency: string;
  current_price: number | null;
  current_value_jpy: number | null;
  cost_jpy: number;
  unrealized_pnl_jpy: number | null;
  pnl_pct: number | null;
  dod_delta_jpy: number | null;
  mom_delta_jpy: number | null;
}

export interface DashboardResponse {
  holdings: HoldingView[];
  total_cost_jpy: number;
  total_value_jpy: number | null;
  total_unrealized_pnl_jpy: number | null;
  total_pnl_pct: number | null;
  realized_pnl_jpy: number;
  cumulative_pnl_jpy: number | null;
  dod_delta_jpy: number | null;
  mom_delta_jpy: number | null;
  last_updated: string | null;
}

export interface CompositionItem {
  symbol: string;
  label: string;
  value_jpy: number;
}

export interface TimeseriesResponse {
  dates: string[];
  values: number[];
  costs: number[];
  pnls: number[];
}

export interface RefreshResponse {
  ok: boolean;
  updated_from_api: boolean;
  remaining_api_requests: number;
  message: string | null;
}

export const getDashboard = () =>
  request<DashboardResponse>("/portfolio/dashboard");

export const getComposition = () =>
  request<CompositionItem[]>("/portfolio/composition");

export const getTimeseries = () =>
  request<TimeseriesResponse>("/portfolio/timeseries");

export const refreshPrices = () =>
  request<RefreshResponse>("/portfolio/refresh", { method: "POST" });
