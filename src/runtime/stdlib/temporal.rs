//! Temporal API (polyfill-level subset) — PlainDate, Instant, Now.

use crate::value::{Environment, Value};
use std::collections::HashMap;
use std::time::{SystemTime, UNIX_EPOCH};

const PLAIN_DATE_MARKER: &str = "__kab_temporal_plain_date";
const INSTANT_MARKER: &str = "__kab_temporal_instant";

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct PlainDateParts {
    year: i64,
    month: i64,
    day: i64,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct InstantParts {
    epoch_ms: i64,
}

fn unix_ms_now() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_millis() as i64)
        .unwrap_or(0)
}

fn civil_from_epoch_days(days: i64) -> PlainDateParts {
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
    PlainDateParts { year, month, day }
}

fn epoch_days_from_civil(p: PlainDateParts) -> i64 {
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
    era * 146_097 + doe - 719_468
}

fn plain_date_object(p: PlainDateParts) -> Value {
    let mut map = HashMap::new();
    map.insert(PLAIN_DATE_MARKER.into(), Value::Bool(true));
    map.insert("year".into(), Value::Number(p.year));
    map.insert("month".into(), Value::Number(p.month));
    map.insert("day".into(), Value::Number(p.day));
    Value::Object(map)
}

fn instant_object(ms: i64) -> Value {
    let mut map = HashMap::new();
    map.insert(INSTANT_MARKER.into(), Value::Bool(true));
    map.insert("epochMilliseconds".into(), Value::Number(ms));
    Value::Object(map)
}

fn plain_date_parts(v: &Value) -> Result<PlainDateParts, String> {
    let Value::Object(map) = v else {
        return Err("expected PlainDate".into());
    };
    if !matches!(map.get(PLAIN_DATE_MARKER), Some(Value::Bool(true))) {
        return Err("expected PlainDate".into());
    }
    Ok(PlainDateParts {
        year: match map.get("year") {
            Some(Value::Number(n)) => *n,
            _ => return Err("invalid PlainDate.year".into()),
        },
        month: match map.get("month") {
            Some(Value::Number(n)) => *n,
            _ => return Err("invalid PlainDate.month".into()),
        },
        day: match map.get("day") {
            Some(Value::Number(n)) => *n,
            _ => return Err("invalid PlainDate.day".into()),
        },
    })
}

fn instant_ms(v: &Value) -> Result<i64, String> {
    let Value::Object(map) = v else {
        return Err("expected Instant".into());
    };
    if !matches!(map.get(INSTANT_MARKER), Some(Value::Bool(true))) {
        return Err("expected Instant".into());
    }
    match map.get("epochMilliseconds") {
        Some(Value::Number(n)) => Ok(*n),
        Some(Value::Float(f)) => Ok(*f as i64),
        _ => Err("invalid Instant.epochMilliseconds".into()),
    }
}

fn plain_date_to_string(p: PlainDateParts) -> String {
    format!("{:04}-{:02}-{:02}", p.year, p.month, p.day)
}

fn instant_to_string(ms: i64) -> String {
    let secs = ms.div_euclid(1000);
    let rem_ms = ms.rem_euclid(1000);
    let days = secs.div_euclid(86_400);
    let day_secs = secs.rem_euclid(86_400);
    let hour = day_secs / 3600;
    let minute = (day_secs % 3600) / 60;
    let second = day_secs % 60;
    let p = civil_from_epoch_days(days);
    format!(
        "{:04}-{:02}-{:02}T{:02}:{:02}:{:02}.{:03}Z",
        p.year, p.month, p.day, hour, minute, second, rem_ms
    )
}

fn parse_fields_object(v: &Value) -> Result<PlainDateParts, String> {
    let Value::Object(map) = v else {
        return Err("PlainDate.from() expects fields object".into());
    };
    let year = map
        .get("year")
        .and_then(|v| match v {
            Value::Number(n) => Some(*n),
            _ => None,
        })
        .ok_or("PlainDate fields require year")?;
    let month = map
        .get("month")
        .and_then(|v| match v {
            Value::Number(n) => Some(*n),
            _ => None,
        })
        .ok_or("PlainDate fields require month")?;
    let day = map
        .get("day")
        .and_then(|v| match v {
            Value::Number(n) => Some(*n),
            _ => None,
        })
        .ok_or("PlainDate fields require day")?;
    Ok(PlainDateParts { year, month, day })
}

fn add_days(p: PlainDateParts, days: i64) -> PlainDateParts {
    let epoch = epoch_days_from_civil(p);
    civil_from_epoch_days(epoch + days)
}

fn plain_date_from_native(args: &[Value], _env: &mut Environment) -> Result<Value, String> {
    let fields = args.first().ok_or("temporal_plain_date_from(fields)")?;
    Ok(plain_date_object(parse_fields_object(fields)?))
}

fn plain_date_new_native(args: &[Value], _env: &mut Environment) -> Result<Value, String> {
    let year = match args.first() {
        Some(Value::Number(n)) => *n,
        _ => return Err("temporal_plain_date_new(year, month, day)".into()),
    };
    let month = match args.get(1) {
        Some(Value::Number(n)) => *n,
        _ => return Err("temporal_plain_date_new(year, month, day)".into()),
    };
    let day = match args.get(2) {
        Some(Value::Number(n)) => *n,
        _ => return Err("temporal_plain_date_new(year, month, day)".into()),
    };
    Ok(plain_date_object(PlainDateParts { year, month, day }))
}

fn plain_date_to_string_native(args: &[Value], _env: &mut Environment) -> Result<Value, String> {
    let v = args.first().ok_or("temporal_plain_date_to_string(date)")?;
    Ok(Value::String(plain_date_to_string(plain_date_parts(v)?)))
}

fn plain_date_equals_native(args: &[Value], _env: &mut Environment) -> Result<Value, String> {
    let a = plain_date_parts(args.first().ok_or("temporal_plain_date_equals(a, b)")?)?;
    let b = plain_date_parts(args.get(1).ok_or("temporal_plain_date_equals(a, b)")?)?;
    Ok(Value::Bool(a == b))
}

fn plain_date_add_native(args: &[Value], _env: &mut Environment) -> Result<Value, String> {
    let p = plain_date_parts(args.first().ok_or("temporal_plain_date_add(date, duration)")?)?;
    let duration = args.get(1).ok_or("temporal_plain_date_add(date, duration)")?;
    let Value::Object(map) = duration else {
        return Err("duration must be object".into());
    };
    let days = map
        .get("days")
        .and_then(|v| match v {
            Value::Number(n) => Some(*n),
            _ => None,
        })
        .unwrap_or(0);
    Ok(plain_date_object(add_days(p, days)))
}

fn instant_from_native(args: &[Value], _env: &mut Environment) -> Result<Value, String> {
    let ms = match args.first() {
        Some(Value::Number(n)) => *n,
        Some(Value::String(s)) => {
            if let Ok(n) = s.parse::<i64>() {
                n
            } else {
                return Err("Temporal.Instant.from() expects epoch ms or ISO string".into());
            }
        }
        _ => return Err("Temporal.Instant.from() expects epoch ms".into()),
    };
    Ok(instant_object(ms))
}

fn instant_now_native(_args: &[Value], _env: &mut Environment) -> Result<Value, String> {
    Ok(instant_object(unix_ms_now()))
}

fn instant_epoch_ms_native(args: &[Value], _env: &mut Environment) -> Result<Value, String> {
    let v = args.first().ok_or("temporal_instant_epoch_ms(instant)")?;
    Ok(Value::Number(instant_ms(v)?))
}

fn instant_to_string_native(args: &[Value], _env: &mut Environment) -> Result<Value, String> {
    let v = args.first().ok_or("temporal_instant_to_string(instant)")?;
    Ok(Value::String(instant_to_string(instant_ms(v)?)))
}

fn plain_date_now_iso_native(_args: &[Value], _env: &mut Environment) -> Result<Value, String> {
    let ms = unix_ms_now();
    let days = ms.div_euclid(1000).div_euclid(86_400);
    Ok(Value::String(plain_date_to_string(civil_from_epoch_days(days))))
}

fn is_plain_date_native(args: &[Value], _env: &mut Environment) -> Result<Value, String> {
    let v = args.first().ok_or("is_temporal_plain_date(v)")?;
    Ok(Value::Bool(plain_date_parts(v).is_ok()))
}

fn is_instant_native(args: &[Value], _env: &mut Environment) -> Result<Value, String> {
    let v = args.first().ok_or("is_temporal_instant(v)")?;
    Ok(Value::Bool(instant_ms(v).is_ok()))
}

pub fn build_temporal_namespace() -> Value {
    let mut plain_date = HashMap::new();
    plain_date.insert("__kab_temporal_plain_date_ctor".into(), Value::Bool(true));
    plain_date.insert("from".into(), Value::NativeFunction(plain_date_from_native));

    let mut instant = HashMap::new();
    instant.insert("__kab_temporal_instant_ctor".into(), Value::Bool(true));
    instant.insert("from".into(), Value::NativeFunction(instant_from_native));

    let mut now = HashMap::new();
    now.insert("instant".into(), Value::NativeFunction(instant_now_native));
    now.insert(
        "plainDateISO".into(),
        Value::NativeFunction(plain_date_now_iso_native),
    );

    let mut temporal = HashMap::new();
    temporal.insert("__kab_temporal".into(), Value::Bool(true));
    temporal.insert("PlainDate".into(), Value::Object(plain_date));
    temporal.insert("Instant".into(), Value::Object(instant));
    temporal.insert("Now".into(), Value::Object(now));
    Value::Object(temporal)
}

pub fn is_plain_date_ctor(v: &Value) -> bool {
    matches!(
        v,
        Value::Object(m) if matches!(m.get("__kab_temporal_plain_date_ctor"), Some(Value::Bool(true)))
    )
}

pub fn is_instant_ctor(v: &Value) -> bool {
    matches!(
        v,
        Value::Object(m) if matches!(m.get("__kab_temporal_instant_ctor"), Some(Value::Bool(true)))
    )
}

pub fn try_plain_date_ctor_call(
    callee: &Value,
    args: &[Value],
    env: &mut Environment,
) -> Option<Result<Value, String>> {
    if is_plain_date_ctor(callee) {
        if let Some(fields) = args.first() {
            Some(plain_date_from_native(&[fields.clone()], env))
        } else {
            Some(Err("Temporal.PlainDate() expects fields object".into()))
        }
    } else {
        None
    }
}

pub fn try_instant_ctor_call(
    callee: &Value,
    args: &[Value],
    env: &mut Environment,
) -> Option<Result<Value, String>> {
    if is_instant_ctor(callee) {
        Some(instant_from_native(args, env))
    } else {
        None
    }
}

pub fn register_temporal(env: &mut Environment) {
    env.set("Temporal".to_string(), build_temporal_namespace());
    let fns: &[(&str, fn(&[Value], &mut Environment) -> Result<Value, String>)] = &[
        ("temporal_plain_date_new", plain_date_new_native),
        ("temporal_plain_date_from", plain_date_from_native),
        ("temporal_plain_date_to_string", plain_date_to_string_native),
        ("temporal_plain_date_equals", plain_date_equals_native),
        ("temporal_plain_date_add", plain_date_add_native),
        ("temporal_instant_from", instant_from_native),
        ("temporal_instant_now", instant_now_native),
        ("temporal_instant_epoch_ms", instant_epoch_ms_native),
        ("temporal_instant_to_string", instant_to_string_native),
        ("temporal_now_plain_date_iso", plain_date_now_iso_native),
        ("is_temporal_plain_date", is_plain_date_native),
        ("is_temporal_instant", is_instant_native),
    ];
    for (name, func) in fns {
        env.set(name.to_string(), Value::NativeFunction(*func));
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn plain_date_roundtrip() {
        let p = PlainDateParts {
            year: 2024,
            month: 6,
            day: 20,
        };
        assert_eq!(plain_date_to_string(p), "2024-06-20");
        let added = add_days(p, 10);
        assert_eq!(added.day, 30);
    }
}
