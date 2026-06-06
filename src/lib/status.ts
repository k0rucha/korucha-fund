//! API rate-limit + scheduler status. Ports OLD/src/handlers/status_api.rs.
import {
  getStats,
  canRequestApi,
  requestsRemaining,
  MAX_REQUESTS_PER_DAY,
} from "@/db/queries/apiStats";
import { jstNow, jstToday } from "@/lib/format";

/** "YYYY-MM-DD HH:MM:SS" → "YYYY年MM月DD日 HH:MM:SS". */
function formatJstHuman(ts: string): string {
  const m = ts.match(/^(\d{4})-(\d{2})-(\d{2}) (\d{2}):(\d{2}):(\d{2})/);
  return m ? `${m[1]}年${m[2]}月${m[3]}日 ${m[4]}:${m[5]}:${m[6]}` : ts;
}

/**
 * Next execution from a 6-field cron (`sec min hour dom mon dow`). Only plain
 * numeric hour/minute are handled; anything else yields "不明" (matches OLD).
 */
function calcNextCron(cron: string): { next: string; minutes: number } {
  const parts = cron.trim().split(/\s+/);
  if (parts.length < 6) return { next: "不明", minutes: 1440 };

  const minute = Number(parts[1]);
  const hour = Number(parts[2]);
  if (!Number.isInteger(minute) || !Number.isInteger(hour)) {
    return { next: "不明", minutes: 1440 };
  }

  const now = jstNow(); // UTC getters read JST wall-clock.
  const todayTarget = new Date(now);
  todayTarget.setUTCHours(hour, minute, 0, 0);

  const usedToday = todayTarget.getTime() > now.getTime();
  const target = todayTarget;
  if (!usedToday) target.setUTCDate(target.getUTCDate() + 1);

  const minutes = Math.floor((target.getTime() - now.getTime()) / 60000);
  const hh = String(target.getUTCHours()).padStart(2, "0");
  const mm = String(target.getUTCMinutes()).padStart(2, "0");
  const next = usedToday
    ? `${hh}:${mm}`
    : `${String(target.getUTCMonth() + 1).padStart(2, "0")}月${String(
        target.getUTCDate(),
      ).padStart(2, "0")}日 ${hh}:${mm}`;

  return { next, minutes };
}

export interface StatusData {
  today: string;
  api_stats: {
    request_count: number;
    max_requests: number;
    remaining_requests: number;
    can_request: boolean;
    minutes_until_next: number;
    last_request_time: string;
    status_message: string;
  };
  scheduler: {
    cron_expression: string;
    next_execution_time: string;
    minutes_until_next_execution: number;
  };
}

export function computeStatus(): StatusData {
  const stats = getStats();
  const { canRequest, minutesUntilNext } = canRequestApi();
  const remaining = requestsRemaining();

  const lastStr = stats.lastRequestTime
    ? formatJstHuman(stats.lastRequestTime)
    : "未実行";

  const statusMessage = canRequest
    ? "外部APIリクエスト可能"
    : minutesUntilNext > 0
      ? `次回リクエストまで ${minutesUntilNext} 分待機中`
      : "本日のリクエスト上限に達しました";

  const cron = process.env.SCHEDULER_CRON ?? "0 0 23 * * *";
  const { next, minutes } = calcNextCron(cron);

  return {
    today: jstToday(),
    api_stats: {
      request_count: stats.requestCount,
      max_requests: MAX_REQUESTS_PER_DAY,
      remaining_requests: remaining,
      can_request: canRequest,
      minutes_until_next: canRequest ? 0 : minutesUntilNext,
      last_request_time: lastStr,
      status_message: statusMessage,
    },
    scheduler: {
      cron_expression: cron,
      next_execution_time: next,
      minutes_until_next_execution: minutes,
    },
  };
}
