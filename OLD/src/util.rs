//! Shared formatting + time helpers used across handlers.

use chrono::{FixedOffset, NaiveDate};

/// JST timezone (spec §11: all dates are JST).
pub fn jst() -> FixedOffset {
    FixedOffset::east_opt(9 * 3600).expect("static offset")
}

/// Today's date in JST.
pub fn jst_today() -> NaiveDate {
    chrono::Utc::now().with_timezone(&jst()).date_naive()
}

/// Format an integer-like number with thousands separators.
/// Negative numbers retain a leading `-`.
pub fn format_with_commas(n: f64) -> String {
    let s = format!("{:.0}", n);
    let neg = s.starts_with('-');
    let digits = if neg { &s[1..] } else { &s[..] };
    let mut chars: Vec<char> = digits.chars().collect();
    let mut i = chars.len() as isize - 3;
    while i > 0 {
        chars.insert(i as usize, ',');
        i -= 3;
    }
    let body: String = chars.into_iter().collect();
    if neg { format!("-{}", body) } else { body }
}

/// `+12,345` / `-12,345` style.
pub fn signed_with_commas(n: f64) -> String {
    let sign = if n >= 0.0 { "+" } else { "-" };
    format!("{}{}", sign, format_with_commas(n.abs()))
}

/// `+12.34%` / `-12.34%` style.
pub fn signed_pct(n: f64) -> String {
    let sign = if n >= 0.0 { "+" } else { "-" };
    format!("{}{:.2}%", sign, n.abs())
}
