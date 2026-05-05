use axum::{
    body::Body,
    extract::{Path, State},
    http::{Response, StatusCode, header},
    response::IntoResponse,
};
use chrono::Duration;

use crate::AppState;
use crate::db::{share_cards, ticker_share_cards, snapshots, prices};
use crate::handlers::share::CardHolding;
use crate::util::{format_with_commas, signed_pct};

// ─── Helpers ───────────────────────────────────────────────────────────────

fn xml_escape(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
}

fn render_png(svg: &str) -> Result<Vec<u8>, String> {
    let mut opt = resvg::usvg::Options::default();
    opt.fontdb_mut().load_system_fonts();
    let local = std::path::Path::new("static/NotoSansJP-Regular.ttf");
    if local.exists() {
        let _ = opt.fontdb_mut().load_font_file(local);
    }
    let tree = resvg::usvg::Tree::from_str(svg, &opt).map_err(|e| e.to_string())?;
    let size = tree.size().to_int_size();
    let mut pixmap = resvg::tiny_skia::Pixmap::new(size.width(), size.height())
        .ok_or_else(|| "pixmap allocation failed".to_string())?;
    resvg::render(&tree, resvg::tiny_skia::Transform::default(), &mut pixmap.as_mut());
    pixmap.encode_png().map_err(|e| e.to_string())
}

fn png_response(png: Vec<u8>) -> Response<Body> {
    Response::builder()
        .header(header::CONTENT_TYPE, "image/png")
        .header(header::CACHE_CONTROL, "public, max-age=31536000, immutable")
        .body(Body::from(png))
        .unwrap()
}

fn delta_color(prev: Option<f64>, current: f64) -> &'static str {
    if let Some(p) = prev.filter(|&p| p > 0.0) {
        if current > p { "#34D399" } else if current < p { "#F87171" } else { "#AABBCC" }
    } else {
        "#AABBCC"
    }
}

fn delta_str(current: f64, prev: Option<f64>, unit: &str, integer: bool) -> String {
    let Some(p) = prev.filter(|&p| p > 0.0) else { return "—".to_string() };
    let d = current - p;
    let pct = (d / p) * 100.0;
    let sign = if d >= 0.0 { "+" } else { "-" };
    let abs = if integer { format_with_commas(d.abs()) } else { format!("{:.2}", d.abs()) };
    format!("{}{}{} ({})", sign, unit, abs, signed_pct(pct))
}

// ─── Chart path generation ─────────────────────────────────────────────────

fn chart_line_path(values: &[f64], x: f64, y: f64, w: f64, h: f64) -> Option<String> {
    if values.len() < 2 { return None; }
    let n = values.len();
    let min_v = values.iter().cloned().fold(f64::INFINITY, f64::min);
    let max_v = values.iter().cloned().fold(f64::NEG_INFINITY, f64::max);
    let range = (max_v - min_v).max(1.0);
    let pad = range * 0.08;
    let lo = min_v - pad;
    let span = range + 2.0 * pad;
    let to_x = |i: usize| x + (i as f64 / (n - 1) as f64) * w;
    let to_y = |v: f64| y + h - ((v - lo) / span) * h;
    let mut path = String::new();
    for (i, &v) in values.iter().enumerate() {
        if i == 0 { path.push_str(&format!("M {:.1},{:.1}", to_x(i), to_y(v))); }
        else { path.push_str(&format!(" L {:.1},{:.1}", to_x(i), to_y(v))); }
    }
    Some(path)
}

fn chart_area_path(values: &[f64], x: f64, y: f64, w: f64, h: f64) -> Option<String> {
    let line = chart_line_path(values, x, y, w, h)?;
    let bottom = y + h;
    Some(format!("{} L {:.1},{:.1} L {:.1},{:.1} Z", line, x + w, bottom, x, bottom))
}

/// SVG fragment for the chart. Drawn first so gradients overlay it.
/// Returns empty string when fewer than 2 data points.
fn chart_fragment(values: &[f64], x: f64, y: f64, w: f64, h: f64) -> String {
    if values.len() < 2 { return String::new(); }
    let first = *values.first().unwrap();
    let last  = *values.last().unwrap();
    let line_color = if last > first { "#34D399" } else if last < first { "#F87171" } else { "#7799FF" };
    let area = chart_area_path(values, x, y, w, h).unwrap_or_default();
    let line = chart_line_path(values, x, y, w, h).unwrap_or_default();
    let gid = format!("cg{}", x as i32);
    let cid = format!("cc{}", x as i32);
    // NOTE: no fill="#COLOR" style attributes here, so r#"..."# is safe.
    format!(
        r#"<defs>
    <linearGradient id="{gid}" x1="0" y1="0" x2="0" y2="1">
      <stop offset="0%" stop-color="{line_color}" stop-opacity="0.28"/>
      <stop offset="100%" stop-color="{line_color}" stop-opacity="0.0"/>
    </linearGradient>
    <clipPath id="{cid}">
      <rect x="{x:.1}" y="{y:.1}" width="{w:.1}" height="{h:.1}"/>
    </clipPath>
  </defs>
  <path d="{area}" fill="url(#{gid})" clip-path="url(#{cid})"/>
  <path d="{line}" fill="none" stroke="{line_color}" stroke-width="2.5"
        stroke-linejoin="round" stroke-linecap="round" clip-path="url(#{cid})"/>"#
    )
}

// ─── Portfolio OGP SVG (1200×630) ─────────────────────────────────────────
//
// Layers (back → front):
//   1. Dark background
//   2. Chart fill + line  (x=80, y=80, w=1040, h=485)
//   3. Top gradient overlay  (y=0  → y=255, opaque→transparent)
//   4. Bottom gradient overlay (y=480 → y=630, transparent→opaque)
//   5. Text: header / stats / footer

fn portfolio_svg(
    total_value_jpy: f64,
    total_cost_jpy: f64,
    unrealized_pnl_jpy: f64,
    created_at: &str,
    holdings: &[CardHolding],
    chart_values: &[f64],
    prev_day_value: Option<f64>,
    prev_month_value: Option<f64>,
) -> String {
    let pnl_pct = if total_cost_jpy > 0.0 { (unrealized_pnl_jpy / total_cost_jpy) * 100.0 } else { 0.0 };
    let pnl_color   = if unrealized_pnl_jpy >= 0.0 { "#34D399" } else { "#F87171" };
    let pnl_sign    = if unrealized_pnl_jpy >= 0.0 { "+" } else { "-" };
    let day_color   = delta_color(prev_day_value, total_value_jpy);
    let month_color = delta_color(prev_month_value, total_value_jpy);

    let value_str = xml_escape(&format!("¥{}", format_with_commas(total_value_jpy)));
    let pnl_str   = xml_escape(&format!("{}¥{}  ({})", pnl_sign, format_with_commas(unrealized_pnl_jpy.abs()), signed_pct(pnl_pct)));
    let cost_str  = xml_escape(&format!("投資元本 ¥{}", format_with_commas(total_cost_jpy)));
    let date_str  = xml_escape(created_at);
    let day_str   = xml_escape(&delta_str(total_value_jpy, prev_day_value,   "¥", true));
    let month_str = xml_escape(&delta_str(total_value_jpy, prev_month_value, "¥", true));

    let mut sorted = holdings.to_vec();
    sorted.sort_by(|a, b| b.current_value_jpy.partial_cmp(&a.current_value_jpy).unwrap_or(std::cmp::Ordering::Equal));
    let symbols_str: String = sorted.iter().take(6)
        .map(|h| xml_escape(&h.symbol)).collect::<Vec<_>>().join("  ·  ");

    let chart = chart_fragment(chart_values, 80.0, 98.0, 1040.0, 408.0);

    format!(
        r##"<svg xmlns="http://www.w3.org/2000/svg" width="1200" height="630" viewBox="0 0 1200 630">
  <rect width="1200" height="630" fill="#000060"/>
  <rect x="0" y="0" width="1200" height="6" fill="#FB5801"/>

  {chart}

  <defs>
    <linearGradient id="tg" x1="0" y1="0" x2="0" y2="1">
      <stop offset="0%" stop-color="#000060" stop-opacity="1.0"/>
      <stop offset="100%" stop-color="#000060" stop-opacity="0.0"/>
    </linearGradient>
    <linearGradient id="bg" x1="0" y1="0" x2="0" y2="1">
      <stop offset="0%" stop-color="#000060" stop-opacity="0.0"/>
      <stop offset="100%" stop-color="#000060" stop-opacity="1.0"/>
    </linearGradient>
  </defs>
  <rect x="0" y="0"   width="1200" height="385" fill="url(#tg)"/>
  <rect x="0" y="480" width="1200" height="150" fill="url(#bg)"/>

  <text x="80" y="72"
        font-family="'Noto Sans JP','Yu Gothic','Meiryo','MS Gothic',sans-serif"
        font-size="32" font-weight="700" fill="#FFFFFF" opacity="0.9">こるちゃファンド</text>
  <text x="1120" y="72"
        font-family="'Inter','Noto Sans JP',sans-serif"
        font-size="24" fill="#8899CC" text-anchor="end">{date}</text>
  <rect x="80" y="97" width="1040" height="1" fill="#FFFFFF" opacity="0.15"/>

  <text x="80" y="140"
        font-family="'Noto Sans JP','Yu Gothic',sans-serif"
        font-size="21" fill="#8899CC">発行時点の総評価額</text>
  <text x="80" y="248"
        font-family="'Inter','Noto Sans JP','Yu Gothic',sans-serif"
        font-size="88" font-weight="900" fill="#FFFFFF" letter-spacing="-1">{value}</text>
  <text x="80" y="312"
        font-family="'Inter','Noto Sans JP','Yu Gothic',sans-serif"
        font-size="44" font-weight="700" fill="{pnl_color}">{pnl}</text>
  <text x="80" y="364"
        font-family="'Noto Sans JP','Yu Gothic',sans-serif"
        font-size="25" fill="#7788BB">{cost}</text>

  <rect x="80" y="507" width="1040" height="1" fill="#FFFFFF" opacity="0.15"/>

  <text x="80"  y="552"
        font-family="'Noto Sans JP','Inter',sans-serif"
        font-size="22" fill="#6677AA">前日比</text>
  <text x="175" y="552"
        font-family="'Inter','Noto Sans JP',sans-serif"
        font-size="24" font-weight="600" fill="{day_color}">{day_str}</text>
  <text x="660" y="552"
        font-family="'Noto Sans JP','Inter',sans-serif"
        font-size="22" fill="#6677AA">前月比</text>
  <text x="755" y="552"
        font-family="'Inter','Noto Sans JP',sans-serif"
        font-size="24" font-weight="600" fill="{month_color}">{month_str}</text>

  <text x="80" y="602"
        font-family="'Inter','Noto Sans JP',monospace,sans-serif"
        font-size="22" fill="#FFFFFF" opacity="0.45" letter-spacing="1">{symbols}</text>
  <text x="1120" y="602"
        font-family="'Inter',sans-serif"
        font-size="18" fill="#FFFFFF" opacity="0.3" text-anchor="end">fund.korucha.com</text>
</svg>"##,
        chart = chart,
        date = date_str,
        value = value_str,
        pnl_color = pnl_color,
        pnl = pnl_str,
        cost = cost_str,
        day_color = day_color,
        day_str = day_str,
        month_color = month_color,
        month_str = month_str,
        symbols = symbols_str,
    )
}

// ─── Ticker OGP SVG (1200×630) ────────────────────────────────────────────

fn ticker_svg(
    symbol: &str,
    display_name: &str,
    currency: &str,
    issue_price_native: f64,
    fx_rate_at_issue: Option<f64>,
    quantity: Option<f64>,
    avg_cost_native: Option<f64>,
    issue_pnl_jpy: Option<f64>,
    created_at: &str,
    chart_values: &[f64],
    prev_day_price: Option<f64>,
    prev_month_price: Option<f64>,
) -> String {
    let unit = if currency == "JPY" { "¥" } else { "$" };
    let integer = currency == "JPY";

    let price_str = xml_escape(&format!("{}{}", unit, format_with_commas(issue_price_native)));
    let name_str  = if !display_name.is_empty() {
        format!("{} ({})", xml_escape(display_name), xml_escape(symbol))
    } else {
        xml_escape(symbol)
    };
    let date_str      = xml_escape(created_at);
    let day_color     = delta_color(prev_day_price, issue_price_native);
    let month_color   = delta_color(prev_month_price, issue_price_native);
    let day_str       = xml_escape(&delta_str(issue_price_native, prev_day_price,   unit, integer));
    let month_str     = xml_escape(&delta_str(issue_price_native, prev_month_price, unit, integer));

    // JPY equivalent line (USD only)
    let jpy_line = if currency == "USD" {
        if let Some(fx) = fx_rate_at_issue {
            let jpy = xml_escape(&format_with_commas(issue_price_native * fx));
            format!(
                r##"<text x="80" y="315"
        font-family="'Inter','Noto Sans JP',sans-serif"
        font-size="30" fill="#8899CC">≈ ¥{jpy}</text>"##
            )
        } else { String::new() }
    } else { String::new() };

    // Position info lines
    let pnl_lines = match (quantity, avg_cost_native, issue_pnl_jpy) {
        (Some(qty), Some(avg), Some(pnl)) if qty > 0.0 => {
            let pnl_pct   = if avg > 0.0 { ((issue_price_native - avg) / avg) * 100.0 } else { 0.0 };
            let pnl_color = if pnl >= 0.0 { "#34D399" } else { "#F87171" };
            let pnl_sign  = if pnl >= 0.0 { "+" } else { "-" };
            let pnl_str   = xml_escape(&format!("{}¥{}  ({})", pnl_sign, format_with_commas(pnl.abs()), signed_pct(pnl_pct)));
            let qty_str   = xml_escape(&format!("保有数 {:.2} 株", qty));
            format!(
                r##"<text x="80" y="365"
        font-family="'Inter','Noto Sans JP','Yu Gothic',sans-serif"
        font-size="44" font-weight="700" fill="{pnl_color}">{pnl_str}</text>
  <text x="80" y="420"
        font-family="'Noto Sans JP','Yu Gothic',sans-serif"
        font-size="25" fill="#7788BB">{qty_str}</text>"##
            )
        }
        _ => String::new(),
    };

    let chart = chart_fragment(chart_values, 80.0, 98.0, 1040.0, 408.0);

    format!(
        r##"<svg xmlns="http://www.w3.org/2000/svg" width="1200" height="630" viewBox="0 0 1200 630">
  <rect width="1200" height="630" fill="#000060"/>
  <rect x="0" y="0" width="1200" height="6" fill="#FB5801"/>

  {chart}

  <defs>
    <linearGradient id="tg" x1="0" y1="0" x2="0" y2="1">
      <stop offset="0%" stop-color="#000060" stop-opacity="1.0"/>
      <stop offset="100%" stop-color="#000060" stop-opacity="0.0"/>
    </linearGradient>
    <linearGradient id="bg" x1="0" y1="0" x2="0" y2="1">
      <stop offset="0%" stop-color="#000060" stop-opacity="0.0"/>
      <stop offset="100%" stop-color="#000060" stop-opacity="1.0"/>
    </linearGradient>
  </defs>
  <rect x="0" y="0"   width="1200" height="385" fill="url(#tg)"/>
  <rect x="0" y="480" width="1200" height="150" fill="url(#bg)"/>

  <text x="80" y="72"
        font-family="'Noto Sans JP','Yu Gothic','Meiryo','MS Gothic',sans-serif"
        font-size="32" font-weight="700" fill="#FFFFFF" opacity="0.9">こるちゃファンド</text>
  <text x="1120" y="72"
        font-family="'Inter','Noto Sans JP',sans-serif"
        font-size="24" fill="#8899CC" text-anchor="end">{date}</text>
  <rect x="80" y="97" width="1040" height="1" fill="#FFFFFF" opacity="0.15"/>

  <text x="80" y="140"
        font-family="'Noto Sans JP','Yu Gothic','Inter',sans-serif"
        font-size="28" fill="#8899CC">{name}</text>
  <text x="80" y="248"
        font-family="'Inter','Noto Sans JP','Yu Gothic',sans-serif"
        font-size="88" font-weight="900" fill="#FFFFFF" letter-spacing="-1">{price}</text>
  {jpy_line}
  {pnl_lines}

  <rect x="80" y="507" width="1040" height="1" fill="#FFFFFF" opacity="0.15"/>

  <text x="80"  y="552"
        font-family="'Noto Sans JP','Inter',sans-serif"
        font-size="22" fill="#6677AA">前日比</text>
  <text x="175" y="552"
        font-family="'Inter','Noto Sans JP',sans-serif"
        font-size="24" font-weight="600" fill="{day_color}">{day_str}</text>
  <text x="660" y="552"
        font-family="'Noto Sans JP','Inter',sans-serif"
        font-size="22" fill="#6677AA">前月比</text>
  <text x="755" y="552"
        font-family="'Inter','Noto Sans JP',sans-serif"
        font-size="24" font-weight="600" fill="{month_color}">{month_str}</text>

  <text x="1120" y="602"
        font-family="'Inter',sans-serif"
        font-size="18" fill="#FFFFFF" opacity="0.3" text-anchor="end">fund.korucha.com</text>
</svg>"##,
        chart = chart,
        date = date_str,
        name = name_str,
        price = price_str,
        jpy_line = jpy_line,
        pnl_lines = pnl_lines,
        day_color = day_color,
        day_str = day_str,
        month_color = month_color,
        month_str = month_str,
    )
}

// ─── Handlers ─────────────────────────────────────────────────────────────

pub async fn share_ogp(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> Result<impl IntoResponse, StatusCode> {
    let card = share_cards::get_share_card(&state.db, &id)
        .await
        .map_err(|e| { tracing::error!("share ogp: db: {}", e); StatusCode::INTERNAL_SERVER_ERROR })?
        .ok_or(StatusCode::NOT_FOUND)?;

    let jst = chrono::FixedOffset::east_opt(9 * 3600).unwrap();
    let created_jst = chrono::TimeZone::from_utc_datetime(&jst, &card.created_at);
    let issue_date  = created_jst.date_naive();
    let created_at  = created_jst.format("%Y-%m-%d").to_string();

    let holdings: Vec<CardHolding> = serde_json::from_str(&card.holdings_json).unwrap_or_default();

    let since = issue_date - Duration::days(30);
    let chart_values: Vec<f64> = snapshots::list_snapshots(&state.db)
        .await.unwrap_or_default()
        .into_iter()
        .filter(|s| s.date >= since && s.date <= issue_date)
        .map(|s| s.total_value_jpy)
        .collect();

    let prev_day_value = snapshots::get_snapshot_on_or_before(&state.db, issue_date - Duration::days(1))
        .await.unwrap_or(None).map(|s| s.total_value_jpy);
    let prev_month_value = snapshots::get_snapshot_on_or_before(&state.db, issue_date - Duration::days(30))
        .await.unwrap_or(None).map(|s| s.total_value_jpy);

    let svg = portfolio_svg(
        card.total_value_jpy, card.total_cost_jpy, card.unrealized_pnl_jpy,
        &created_at, &holdings, &chart_values, prev_day_value, prev_month_value,
    );

    let png = tokio::task::spawn_blocking(move || render_png(&svg))
        .await
        .map_err(|e| { tracing::error!("share ogp: join: {}", e); StatusCode::INTERNAL_SERVER_ERROR })?
        .map_err(|e| { tracing::error!("share ogp: render: {}", e); StatusCode::INTERNAL_SERVER_ERROR })?;

    Ok(png_response(png))
}

pub async fn ticker_ogp(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> Result<impl IntoResponse, StatusCode> {
    let card = ticker_share_cards::get_ticker_share_card(&state.db, &id)
        .await
        .map_err(|e| { tracing::error!("ticker ogp: db: {}", e); StatusCode::INTERNAL_SERVER_ERROR })?
        .ok_or(StatusCode::NOT_FOUND)?;

    let jst = chrono::FixedOffset::east_opt(9 * 3600).unwrap();
    let created_jst = chrono::TimeZone::from_utc_datetime(&jst, &card.created_at);
    let issue_date  = created_jst.date_naive();
    let created_at  = created_jst.format("%Y-%m-%d").to_string();

    let since = issue_date - Duration::days(30);
    let chart_values: Vec<f64> = prices::list_history(&state.db, &card.symbol, issue_date)
        .await.unwrap_or_default()
        .into_iter()
        .filter(|(d, _)| *d >= since)
        .map(|(_, p)| p)
        .collect();

    let prev_day_price = prices::get_price_on_or_before(&state.db, &card.symbol, issue_date - Duration::days(1))
        .await.unwrap_or(None);
    let prev_month_price = prices::get_price_on_or_before(&state.db, &card.symbol, issue_date - Duration::days(30))
        .await.unwrap_or(None);

    let display_name = card.display_name.as_deref().unwrap_or("");
    let svg = ticker_svg(
        &card.symbol, display_name, &card.currency,
        card.issue_price_native, card.fx_rate_at_issue,
        card.quantity, card.avg_cost_native, card.issue_pnl_jpy,
        &created_at, &chart_values, prev_day_price, prev_month_price,
    );

    let png = tokio::task::spawn_blocking(move || render_png(&svg))
        .await
        .map_err(|e| { tracing::error!("ticker ogp: join: {}", e); StatusCode::INTERNAL_SERVER_ERROR })?
        .map_err(|e| { tracing::error!("ticker ogp: render: {}", e); StatusCode::INTERNAL_SERVER_ERROR })?;

    Ok(png_response(png))
}
