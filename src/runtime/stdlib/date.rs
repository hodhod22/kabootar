//! Date helpers — UTC/local getters, timezone offset, ISO strings.

use crate::value::{Environment, Value};
use std::cell::RefCell;
use std::collections::HashMap;
use std::time::{SystemTime, UNIX_EPOCH};

const DATE_MARKER: &str = "__kab_date";

thread_local! {
    static TZ_OFFSET_MINUTES: RefCell<i64> = const { RefCell::new(0) };
}

#[derive(Clone, Copy, Debug)]
struct CivilParts {
    year: i64,
    month: i64,
    day: i64,
    hour: i64,
    minute: i64,
    second: i64,
    millisecond: i64,
}

fn unix_ms_now() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_millis() as i64)
        .unwrap_or(0)
}

fn timezone_offset_minutes() -> i64 {
    TZ_OFFSET_MINUTES.with(|o| *o.borrow())
}

fn civil_from_unix_ms(ms: i64) -> CivilParts {
    let secs = ms.div_euclid(1000);
    let rem_ms = ms.rem_euclid(1000);
    let days = secs.div_euclid(86_400);
    let day_secs = secs.rem_euclid(86_400);
    let hour = day_secs / 3600;
    let minute = (day_secs % 3600) / 60;
    let second = day_secs % 60;
    let z = days + 719_468;
    let era = if z >= 0 { z } else { z - 146_096 } / 146_097;
    let doe = z - era * 146_097;
    let yoe = (doe - doe / 1460 + doe / 36524 - doe / 146_096) / 365;
    let y = yoe + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let day = doy - (153 * mp + 2) / 5 + 1;
    let month = mp + if mp < 10 { 3 } else { -9 };
    let year = y + if month <= 2 { 1 } else { 0 };
    CivilParts {
        year,
        month,
        day,
        hour,
        minute,
        second,
        millisecond: rem_ms,
    }
}

fn unix_ms_from_civil(p: CivilParts) -> i64 {
    let month = p.month;
    let year = p.year;
    let y_adj = if month <= 2 { year - 1 } else { year };
    let era = if y_adj >= 0 {
        y_adj / 400
    } else {
        (y_adj - 399) / 400
    };
    let yoe = y_adj - era * 400;
    let month_idx = if month > 2 { month - 3 } else { month + 9 };
    let doy = (153 * month_idx + 2) / 5 + p.day - 1;
    let doe = yoe * 365 + yoe / 4 - yoe / 100 + doy;
    let days = era * 146_097 + doe - 719_468;
    let secs = days * 86_400 + p.hour * 3600 + p.minute * 60 + p.second;
    secs * 1000 + p.millisecond
}

fn weekday_sun0(year: i64, month: i64, day: i64) -> i64 {
    let civil = CivilParts {
        year,
        month,
        day,
        hour: 12,
        minute: 0,
        second: 0,
        millisecond: 0,
    };
    let days = unix_ms_from_civil(civil).div_euclid(86_400);
    (days + 4).rem_euclid(7)
}

fn parts_for_ms(ms: i64, utc: bool) -> CivilParts {
    if utc {
        civil_from_unix_ms(ms)
    } else {
        let offset = timezone_offset_minutes();
        civil_from_unix_ms(ms - offset * 60_000)
    }
}

pub fn is_date_value(v: &Value) -> bool {
    match v {
        Value::Object(map) => {
            matches!(map.get(DATE_MARKER), Some(Value::Bool(true)))
                || map.contains_key("ms")
                    && map.contains_key("year")
                    && map.contains_key("month")
        }
        _ => false,
    }
}

pub fn date_epoch_ms(v: &Value) -> Result<i64, String> {
    date_ms(v)
}

fn date_ms(v: &Value) -> Result<i64, String> {
    let Value::Object(map) = v else {
        return Err("expected date object".into());
    };
    match map.get("ms") {
        Some(Value::Number(n)) => Ok(*n),
        Some(Value::Float(f)) => Ok(*f as i64),
        _ => Err("expected date object".into()),
    }
}

fn parts_map_from_civil(ms: i64, p: CivilParts) -> HashMap<String, Value> {
    let mut map = HashMap::new();
    map.insert(DATE_MARKER.into(), Value::Bool(true));
    map.insert("ms".into(), Value::Number(ms));
    map.insert("year".into(), Value::Number(p.year));
    map.insert("month".into(), Value::Number(p.month));
    map.insert("day".into(), Value::Number(p.day));
    map.insert("hour".into(), Value::Number(p.hour));
    map.insert("minute".into(), Value::Number(p.minute));
    map.insert("second".into(), Value::Number(p.second));
    map.insert("millisecond".into(), Value::Number(p.millisecond));
    map
}

fn make_date_object(ms: i64) -> Value {
    Value::from_object(parts_map_from_civil(ms, civil_from_unix_ms(ms)))
}

fn iso_string_from_ms(ms: i64) -> String {
    let p = civil_from_unix_ms(ms);
    format!(
        "{:04}-{:02}-{:02}T{:02}:{:02}:{:02}.{:03}Z",
        p.year, p.month, p.day, p.hour, p.minute, p.second, p.millisecond
    )
}

fn parse_digits(s: &str, len: usize) -> Option<i64> {
    if s.len() < len || !s[..len].chars().all(|c| c.is_ascii_digit()) {
        return None;
    }
    s[..len].parse().ok()
}

fn parse_iso_ms(text: &str) -> Option<i64> {
    let s = text.trim();
    if s.len() < 20 {
        return None;
    }
    let year = parse_digits(s, 4)?;
    if s.as_bytes().get(4) != Some(&b'-') {
        return None;
    }
    let month = parse_digits(&s[5..], 2)?;
    if s.as_bytes().get(7) != Some(&b'-') {
        return None;
    }
    let day = parse_digits(&s[8..], 2)?;
    if s.as_bytes().get(10) != Some(&b'T') {
        return None;
    }
    let hour = parse_digits(&s[11..], 2)?;
    if s.as_bytes().get(13) != Some(&b':') {
        return None;
    }
    let minute = parse_digits(&s[14..], 2)?;
    if s.as_bytes().get(16) != Some(&b':') {
        return None;
    }
    let second = parse_digits(&s[17..], 2)?;
    let mut millisecond = 0i64;
    let mut tail = &s[19..];
    if tail.starts_with('.') {
        tail = &tail[1..];
        let mut digits = String::new();
        for c in tail.chars() {
            if c.is_ascii_digit() {
                digits.push(c);
            } else {
                break;
            }
        }
        if !digits.is_empty() {
            let padded = format!("{:0<3}", &digits[..digits.len().min(3)]);
            millisecond = padded[..3].parse().ok()?;
        }
        tail = &tail[digits.len()..];
    }
    let utc = tail == "Z" || tail.starts_with("+00:00") || tail == "z";
    if !utc && !tail.is_empty() && tail != "Z" && tail != "z" {
        return None;
    }
    Some(unix_ms_from_civil(CivilParts {
        year,
        month,
        day,
        hour,
        minute,
        second,
        millisecond,
    }))
}

fn parse_ms(text: &str) -> Result<i64, String> {
    if let Some(ms) = parse_iso_ms(text) {
        return Ok(ms);
    }
    if let Ok(n) = text.parse::<i64>() {
        return Ok(n);
    }
    if let Ok(f) = text.parse::<f64>() {
        return Ok(f as i64);
    }
    Err(format!("date_parse() could not parse {:?}", text))
}

fn num_arg(v: Option<&Value>) -> Result<i64, String> {
    match v {
        Some(Value::Number(n)) => Ok(*n),
        Some(Value::Float(f)) => Ok(*f as i64),
        _ => Err("expected number".into()),
    }
}

fn date_arg(args: &[Value], idx: usize, name: &str) -> Result<i64, String> {
    let d = args.get(idx).ok_or(name)?;
    date_ms(d)
}

fn date_now_ms_native(_args: &[Value], _env: &mut Environment) -> Result<Value, String> {
    Ok(Value::Number(unix_ms_now()))
}

fn date_now_native(_args: &[Value], _env: &mut Environment) -> Result<Value, String> {
    Ok(make_date_object(unix_ms_now()))
}

fn date_parse_native(args: &[Value], _env: &mut Environment) -> Result<Value, String> {
    let text = match args.first() {
        Some(Value::String(s)) => s.as_str(),
        _ => return Err("date_parse(text)".into()),
    };
    Ok(Value::Number(parse_ms(text)?))
}

fn date_format_native(args: &[Value], _env: &mut Environment) -> Result<Value, String> {
    let ms = num_arg(args.first())?;
    Ok(make_date_object(ms))
}

fn date_iso_native(args: &[Value], _env: &mut Environment) -> Result<Value, String> {
    let ms = match args.first() {
        Some(Value::Number(n)) => *n,
        Some(Value::Float(f)) => *f as i64,
        None => unix_ms_now(),
        _ => return Err("date_iso(ms?) expects number".into()),
    };
    Ok(Value::String(iso_string_from_ms(ms)))
}

fn date_new_native(args: &[Value], _env: &mut Environment) -> Result<Value, String> {
    let ms = match args.first() {
        Some(Value::Number(n)) => *n,
        Some(Value::Float(f)) => *f as i64,
        Some(Value::String(s)) => parse_ms(s)?,
        None => unix_ms_now(),
        _ => return Err("date_new(ms?) expects number or string".into()),
    };
    Ok(make_date_object(ms))
}

fn date_get_time_native(args: &[Value], _env: &mut Environment) -> Result<Value, String> {
    Ok(Value::Number(date_arg(args, 0, "date_get_time(date)")?))
}

fn date_value_of_native(args: &[Value], env: &mut Environment) -> Result<Value, String> {
    date_get_time_native(args, env)
}

fn date_get_full_year_native(args: &[Value], _env: &mut Environment) -> Result<Value, String> {
    let ms = date_arg(args, 0, "date_get_full_year(date)")?;
    Ok(Value::Number(parts_for_ms(ms, false).year))
}

fn date_get_utc_full_year_native(args: &[Value], _env: &mut Environment) -> Result<Value, String> {
    let ms = date_arg(args, 0, "date_get_utc_full_year(date)")?;
    Ok(Value::Number(parts_for_ms(ms, true).year))
}

fn date_get_month_native(args: &[Value], _env: &mut Environment) -> Result<Value, String> {
    let ms = date_arg(args, 0, "date_get_month(date)")?;
    Ok(Value::Number(parts_for_ms(ms, false).month - 1))
}

fn date_get_utc_month_native(args: &[Value], _env: &mut Environment) -> Result<Value, String> {
    let ms = date_arg(args, 0, "date_get_utc_month(date)")?;
    Ok(Value::Number(parts_for_ms(ms, true).month - 1))
}

fn date_get_date_native(args: &[Value], _env: &mut Environment) -> Result<Value, String> {
    let ms = date_arg(args, 0, "date_get_date(date)")?;
    Ok(Value::Number(parts_for_ms(ms, false).day))
}

fn date_get_utc_date_native(args: &[Value], _env: &mut Environment) -> Result<Value, String> {
    let ms = date_arg(args, 0, "date_get_utc_date(date)")?;
    Ok(Value::Number(parts_for_ms(ms, true).day))
}

fn date_get_day_native(args: &[Value], _env: &mut Environment) -> Result<Value, String> {
    let ms = date_arg(args, 0, "date_get_day(date)")?;
    let p = parts_for_ms(ms, false);
    Ok(Value::Number(weekday_sun0(p.year, p.month, p.day)))
}

fn date_get_utc_day_native(args: &[Value], _env: &mut Environment) -> Result<Value, String> {
    let ms = date_arg(args, 0, "date_get_utc_day(date)")?;
    let p = parts_for_ms(ms, true);
    Ok(Value::Number(weekday_sun0(p.year, p.month, p.day)))
}

fn date_get_hours_native(args: &[Value], _env: &mut Environment) -> Result<Value, String> {
    let ms = date_arg(args, 0, "date_get_hours(date)")?;
    Ok(Value::Number(parts_for_ms(ms, false).hour))
}

fn date_get_utc_hours_native(args: &[Value], _env: &mut Environment) -> Result<Value, String> {
    let ms = date_arg(args, 0, "date_get_utc_hours(date)")?;
    Ok(Value::Number(parts_for_ms(ms, true).hour))
}

fn date_get_minutes_native(args: &[Value], _env: &mut Environment) -> Result<Value, String> {
    let ms = date_arg(args, 0, "date_get_minutes(date)")?;
    Ok(Value::Number(parts_for_ms(ms, false).minute))
}

fn date_get_utc_minutes_native(args: &[Value], _env: &mut Environment) -> Result<Value, String> {
    let ms = date_arg(args, 0, "date_get_utc_minutes(date)")?;
    Ok(Value::Number(parts_for_ms(ms, true).minute))
}

fn date_get_seconds_native(args: &[Value], _env: &mut Environment) -> Result<Value, String> {
    let ms = date_arg(args, 0, "date_get_seconds(date)")?;
    Ok(Value::Number(parts_for_ms(ms, false).second))
}

fn date_get_utc_seconds_native(args: &[Value], _env: &mut Environment) -> Result<Value, String> {
    let ms = date_arg(args, 0, "date_get_utc_seconds(date)")?;
    Ok(Value::Number(parts_for_ms(ms, true).second))
}

fn date_get_milliseconds_native(args: &[Value], _env: &mut Environment) -> Result<Value, String> {
    let ms = date_arg(args, 0, "date_get_milliseconds(date)")?;
    Ok(Value::Number(parts_for_ms(ms, false).millisecond))
}

fn date_get_utc_milliseconds_native(args: &[Value], _env: &mut Environment) -> Result<Value, String> {
    let ms = date_arg(args, 0, "date_get_utc_milliseconds(date)")?;
    Ok(Value::Number(parts_for_ms(ms, true).millisecond))
}

fn date_get_timezone_offset_native(
    _args: &[Value],
    _env: &mut Environment,
) -> Result<Value, String> {
    Ok(Value::Number(timezone_offset_minutes()))
}

fn date_set_timezone_offset_native(args: &[Value], _env: &mut Environment) -> Result<Value, String> {
    let minutes = num_arg(args.first())?;
    TZ_OFFSET_MINUTES.with(|o| *o.borrow_mut() = minutes);
    Ok(Value::Number(minutes))
}

fn date_set_time_native(args: &[Value], _env: &mut Environment) -> Result<Value, String> {
    let ms = num_arg(args.get(1))?;
    Ok(make_date_object(ms))
}

fn date_set_utc_time_native(args: &[Value], _env: &mut Environment) -> Result<Value, String> {
    date_set_time_native(args, _env)
}

fn set_civil_date(
    args: &[Value],
    utc: bool,
    year: Option<i64>,
    month0: Option<i64>,
    day: Option<i64>,
) -> Result<Value, String> {
    let ms = date_arg(args, 0, "date_set(date, ...)")?;
    let mut p = parts_for_ms(ms, utc);
    if let Some(y) = year {
        p.year = y;
    }
    if let Some(m) = month0 {
        p.month = m + 1;
    }
    if let Some(d) = day {
        p.day = d;
    }
    let new_ms = if utc {
        unix_ms_from_civil(p)
    } else {
        let offset = timezone_offset_minutes();
        unix_ms_from_civil(p) + offset * 60_000
    };
    Ok(make_date_object(new_ms))
}

fn date_set_full_year_native(args: &[Value], _env: &mut Environment) -> Result<Value, String> {
    set_civil_date(args, false, Some(num_arg(args.get(1))?), None, None)
}

fn date_set_month_native(args: &[Value], _env: &mut Environment) -> Result<Value, String> {
    set_civil_date(args, false, None, Some(num_arg(args.get(1))?), None)
}

fn date_set_date_native(args: &[Value], _env: &mut Environment) -> Result<Value, String> {
    set_civil_date(args, false, None, None, Some(num_arg(args.get(1))?))
}

fn date_set_utc_full_year_native(args: &[Value], _env: &mut Environment) -> Result<Value, String> {
    set_civil_date(args, true, Some(num_arg(args.get(1))?), None, None)
}

fn date_set_utc_month_native(args: &[Value], _env: &mut Environment) -> Result<Value, String> {
    set_civil_date(args, true, None, Some(num_arg(args.get(1))?), None)
}

fn date_set_utc_date_native(args: &[Value], _env: &mut Environment) -> Result<Value, String> {
    set_civil_date(args, true, None, None, Some(num_arg(args.get(1))?))
}

fn date_to_iso_string_native(args: &[Value], _env: &mut Environment) -> Result<Value, String> {
    let ms = if let Some(d) = args.first() {
        if matches!(d, Value::Number(_) | Value::Float(_)) {
            num_arg(Some(d))?
        } else {
            date_ms(d)?
        }
    } else {
        unix_ms_now()
    };
    Ok(Value::String(iso_string_from_ms(ms)))
}

fn date_to_string_native(args: &[Value], _env: &mut Environment) -> Result<Value, String> {
    let ms = date_arg(args, 0, "date_to_string(date)")?;
    let p = parts_for_ms(ms, false);
    Ok(Value::String(format!(
        "{:04}-{:02}-{:02} {:02}:{:02}:{:02}",
        p.year, p.month, p.day, p.hour, p.minute, p.second
    )))
}

fn is_date_native(args: &[Value], _env: &mut Environment) -> Result<Value, String> {
    let v = args.first().ok_or("is_date(v)")?;
    Ok(Value::Bool(is_date_value(v)))
}

pub fn register_date(env: &mut Environment) {
    let fns: &[(&str, fn(&[Value], &mut Environment) -> Result<Value, String>)] = &[
        ("Date_now", date_now_ms_native),
        ("date_now_ms", date_now_ms_native),
        ("date_now", date_now_native),
        ("date_parse", date_parse_native),
        ("date_format", date_format_native),
        ("date_iso", date_iso_native),
        ("date_new", date_new_native),
        ("date_get_time", date_get_time_native),
        ("date_value_of", date_value_of_native),
        ("date_get_full_year", date_get_full_year_native),
        ("date_get_utc_full_year", date_get_utc_full_year_native),
        ("date_get_month", date_get_month_native),
        ("date_get_utc_month", date_get_utc_month_native),
        ("date_get_date", date_get_date_native),
        ("date_get_utc_date", date_get_utc_date_native),
        ("date_get_day", date_get_day_native),
        ("date_get_utc_day", date_get_utc_day_native),
        ("date_get_hours", date_get_hours_native),
        ("date_get_utc_hours", date_get_utc_hours_native),
        ("date_get_minutes", date_get_minutes_native),
        ("date_get_utc_minutes", date_get_utc_minutes_native),
        ("date_get_seconds", date_get_seconds_native),
        ("date_get_utc_seconds", date_get_utc_seconds_native),
        ("date_get_milliseconds", date_get_milliseconds_native),
        ("date_get_utc_milliseconds", date_get_utc_milliseconds_native),
        ("date_get_timezone_offset", date_get_timezone_offset_native),
        ("date_set_timezone_offset", date_set_timezone_offset_native),
        ("date_set_time", date_set_time_native),
        ("date_set_utc_time", date_set_utc_time_native),
        ("date_set_full_year", date_set_full_year_native),
        ("date_set_month", date_set_month_native),
        ("date_set_date", date_set_date_native),
        ("date_set_utc_full_year", date_set_utc_full_year_native),
        ("date_set_utc_month", date_set_utc_month_native),
        ("date_set_utc_date", date_set_utc_date_native),
        ("date_to_iso_string", date_to_iso_string_native),
        ("date_to_string", date_to_string_native),
        ("is_date", is_date_native),
    ];
    for (name, func) in fns {
        env.set(name.to_string(), Value::NativeFunction(*func));
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn epoch_roundtrip() {
        let ms = 0i64;
        assert_eq!(unix_ms_from_civil(civil_from_unix_ms(ms)), ms);
    }

    #[test]
    fn iso_parse_epoch() {
        assert_eq!(parse_iso_ms("1970-01-01T00:00:00.000Z").unwrap(), 0);
    }

    #[test]
    fn local_offset_shifts_hours() {
        TZ_OFFSET_MINUTES.with(|o| *o.borrow_mut() = -60);
        let p = parts_for_ms(0, false);
        assert_eq!(p.hour, 1);
        TZ_OFFSET_MINUTES.with(|o| *o.borrow_mut() = 0);
    }
}
