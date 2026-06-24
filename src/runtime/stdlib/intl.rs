//! Minimal ECMAScript `Intl` — `NumberFormat` and `DateTimeFormat`.

use crate::runtime::stdlib::date::{date_epoch_ms, is_date_value};
use crate::runtime::stdlib::iterator::attach_bound_method;
use crate::value::{Environment, Value};
use std::collections::HashMap;

const NF_MARKER: &str = "__kab_intl_nf";
const DTF_MARKER: &str = "__kab_intl_dtf";

#[derive(Clone, Debug)]
struct NumberFormatOptions {
    style: String,
    currency: String,
    minimum_fraction_digits: Option<usize>,
    maximum_fraction_digits: Option<usize>,
    use_grouping: bool,
}

impl Default for NumberFormatOptions {
    fn default() -> Self {
        Self {
            style: "decimal".into(),
            currency: "USD".into(),
            minimum_fraction_digits: None,
            maximum_fraction_digits: None,
            use_grouping: true,
        }
    }
}

#[derive(Clone, Debug)]
struct DateTimeFormatOptions {
    year: Option<String>,
    month: Option<String>,
    day: Option<String>,
    hour: Option<String>,
    minute: Option<String>,
    second: Option<String>,
    date_style: Option<String>,
    time_style: Option<String>,
    hour12: bool,
}

impl Default for DateTimeFormatOptions {
    fn default() -> Self {
        Self {
            year: None,
            month: None,
            day: None,
            hour: None,
            minute: None,
            second: None,
            date_style: None,
            time_style: None,
            hour12: false,
        }
    }
}

fn string_field(map: &HashMap<String, Value>, key: &str) -> Option<String> {
    match map.get(key) {
        Some(Value::String(s)) => Some(s.clone()),
        _ => None,
    }
}

fn bool_field(map: &HashMap<String, Value>, key: &str, default: bool) -> bool {
    match map.get(key) {
        Some(Value::Bool(b)) => *b,
        _ => default,
    }
}

fn usize_field(map: &HashMap<String, Value>, key: &str) -> Option<usize> {
    match map.get(key) {
        Some(Value::Number(n)) if *n >= 0 => Some(*n as usize),
        Some(Value::Float(f)) if *f >= 0.0 => Some(*f as usize),
        _ => None,
    }
}

fn parse_number_format_options(v: &Value) -> NumberFormatOptions {
    let mut opts = NumberFormatOptions::default();
    let Value::Object(map) = v else {
        return opts;
    };
    if let Some(s) = string_field(map, "style") {
        opts.style = s;
    }
    if let Some(s) = string_field(map, "currency") {
        opts.currency = s;
    }
    opts.minimum_fraction_digits = usize_field(map, "minimumFractionDigits");
    opts.maximum_fraction_digits = usize_field(map, "maximumFractionDigits");
    opts.use_grouping = bool_field(map, "useGrouping", true);
    opts
}

fn parse_date_time_format_options(v: &Value) -> DateTimeFormatOptions {
    let mut opts = DateTimeFormatOptions::default();
    let Value::Object(map) = v else {
        return opts;
    };
    opts.year = string_field(map, "year");
    opts.month = string_field(map, "month");
    opts.day = string_field(map, "day");
    opts.hour = string_field(map, "hour");
    opts.minute = string_field(map, "minute");
    opts.second = string_field(map, "second");
    opts.date_style = string_field(map, "dateStyle");
    opts.time_style = string_field(map, "timeStyle");
    opts.hour12 = bool_field(map, "hour12", false);
    opts
}

fn locale_decimal(locale: &str) -> (char, char) {
    if locale.starts_with("sv") || locale.starts_with("de") || locale.starts_with("fr") {
        (',', ' ')
    } else {
        ('.', ',')
    }
}

fn format_fixed(mut n: f64, min: usize, max: usize) -> String {
    if !n.is_finite() {
        return "NaN".into();
    }
    let negative = n < 0.0;
    if negative {
        n = -n;
    }
    let rounded = format!("{:.max$}", n, max = max);
    let mut parts = rounded.split('.');
    let int_part = parts.next().unwrap_or("0");
    let frac_part = parts.next().unwrap_or("");
    let frac_padded = if frac_part.len() < min {
        format!("{frac_part}{:0<min$}", "", min = min - frac_part.len())
    } else {
        frac_part.to_string()
    };
    let body = if min == 0 && max == 0 {
        int_part.to_string()
    } else {
        format!("{int_part}.{frac_padded}")
    };
    if negative {
        format!("-{body}")
    } else {
        body
    }
}

fn add_grouping(int_part: &str, sep: char) -> String {
    let digits: Vec<char> = int_part.chars().filter(|c| c.is_ascii_digit()).collect();
    if digits.is_empty() {
        return int_part.to_string();
    }
    let negative = int_part.starts_with('-');
    let mut out = String::new();
    let len = digits.len();
    for (i, d) in digits.iter().enumerate() {
        if i > 0 && (len - i) % 3 == 0 {
            out.push(sep);
        }
        out.push(*d);
    }
    if negative {
        format!("-{out}")
    } else {
        out
    }
}

fn currency_symbol(code: &str) -> &str {
    match code {
        "USD" => "$",
        "EUR" => "€",
        "GBP" => "£",
        "SEK" => "kr",
        "JPY" => "¥",
        _ => code,
    }
}

fn format_number_value(locale: &str, opts: &NumberFormatOptions, value: &Value) -> Result<String, String> {
    let mut n = match value {
        Value::Number(i) => *i as f64,
        Value::Float(f) => *f,
        Value::BigInt(b) => b.to_string().parse::<f64>().unwrap_or(0.0),
        other => return Err(format!("Intl.NumberFormat.format() expects number, got {:?}", other)),
    };
    if opts.style == "percent" {
        n *= 100.0;
    }
    let min = opts.minimum_fraction_digits.unwrap_or(if opts.style == "currency" { 2 } else { 0 });
    let max = opts.maximum_fraction_digits.unwrap_or(min);
    let (dec_sep, group_sep) = locale_decimal(locale);
    let mut body = format_fixed(n, min, max);
    if let Some((int_part, frac)) = body.split_once('.') {
        let grouped = if opts.use_grouping {
            add_grouping(int_part, group_sep)
        } else {
            int_part.to_string()
        };
        body = format!("{grouped}.{frac}");
    } else if opts.use_grouping {
        body = add_grouping(&body, group_sep);
    }
    if dec_sep != '.' {
        body = body.replace('.', &dec_sep.to_string());
    }
    Ok(match opts.style.as_str() {
        "percent" => format!("{body}%"),
        "currency" => {
            let sym = currency_symbol(&opts.currency);
            format!("{sym}{body}")
        }
        _ => body,
    })
}

fn pad2(n: i64) -> String {
    format!("{n:02}")
}

fn format_date_time_value(
    locale: &str,
    opts: &DateTimeFormatOptions,
    date: &Value,
) -> Result<String, String> {
    if !is_date_value(date) {
        return Err("Intl.DateTimeFormat.format() expects Date".into());
    }
    let ms = date_epoch_ms(date)?;
    let utc_secs = ms.div_euclid(1000);
    let rem_ms = ms.rem_euclid(1000);
    let days = utc_secs.div_euclid(86_400);
    let day_secs = utc_secs.rem_euclid(86_400);
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

    let date_style = opts.date_style.as_deref().unwrap_or("medium");
    let time_style = opts.time_style.as_deref().unwrap_or("medium");

    let date_part = if opts.year.is_some() || opts.month.is_some() || opts.day.is_some() {
        let y = if opts.year.as_deref() == Some("numeric") || opts.year.is_some() {
            format!("{year:04}")
        } else {
            format!("{:02}", year % 100)
        };
        let m = pad2(month);
        let d = pad2(day);
        if locale.starts_with("en") {
            format!("{m}/{d}/{y}")
        } else {
            format!("{y}-{m}-{d}")
        }
    } else {
        let m = pad2(month);
        let d = pad2(day);
        match date_style {
            "full" | "long" => format!("{year:04}-{m}-{d}"),
            "short" => format!("{}/{}/{}", year % 100, month, day),
            _ => format!("{m}/{d}/{year}"),
        }
    };

    let time_part = if opts.hour.is_some() || opts.minute.is_some() || opts.second.is_some() {
        let ph = pad2(hour);
        let pm = pad2(minute);
        let ps = pad2(second);
        if opts.hour12 {
            let h12 = hour % 12;
            let h = if h12 == 0 { 12 } else { h12 };
            let ampm = if hour < 12 { "AM" } else { "PM" };
            format!("{h}:{pm}:{ps} {ampm}")
        } else {
            format!("{ph}:{pm}:{ps}")
        }
    } else {
        let ph = pad2(hour);
        let pm = pad2(minute);
        let ps = pad2(second);
        match time_style {
            "full" | "long" => format!("{ph}:{pm}:{ps}.{rem_ms:03}"),
            "short" => format!("{ph}:{pm}"),
            _ => format!("{ph}:{pm}:{ps}"),
        }
    };

    if opts.date_style.is_some() && opts.time_style.is_some() {
        Ok(format!("{date_part}, {time_part}"))
    } else if opts.time_style.is_some() || opts.hour.is_some() {
        Ok(time_part)
    } else {
        Ok(date_part)
    }
}

fn nf_from_object(v: &Value) -> Result<(&str, NumberFormatOptions), String> {
    let Value::Object(map) = v else {
        return Err("expected Intl.NumberFormat instance".into());
    };
    if !matches!(map.get(NF_MARKER), Some(Value::Bool(true))) {
        return Err("expected Intl.NumberFormat instance".into());
    }
    let locale = match map.get("locale") {
        Some(Value::String(s)) => s.as_str(),
        _ => "en-US",
    };
    let opts = match map.get("options") {
        Some(v) => parse_number_format_options(v),
        None => NumberFormatOptions::default(),
    };
    Ok((locale, opts))
}

fn dtf_from_object(v: &Value) -> Result<(&str, DateTimeFormatOptions), String> {
    let Value::Object(map) = v else {
        return Err("expected Intl.DateTimeFormat instance".into());
    };
    if !matches!(map.get(DTF_MARKER), Some(Value::Bool(true))) {
        return Err("expected Intl.DateTimeFormat instance".into());
    }
    let locale = match map.get("locale") {
        Some(Value::String(s)) => s.as_str(),
        _ => "en-US",
    };
    let opts = match map.get("options") {
        Some(v) => parse_date_time_format_options(v),
        None => DateTimeFormatOptions::default(),
    };
    Ok((locale, opts))
}

fn create_number_format(locale: String, options: Value) -> Value {
    let mut map = HashMap::new();
    map.insert(NF_MARKER.into(), Value::Bool(true));
    map.insert("locale".into(), Value::String(locale));
    map.insert("options".into(), options);
    attach_bound_method(&mut map, "format", nf_format_method_native);
    attach_bound_method(&mut map, "formatToParts", nf_format_parts_native);
    Value::Object(map)
}

fn create_date_time_format(locale: String, options: Value) -> Value {
    let mut map = HashMap::new();
    map.insert(DTF_MARKER.into(), Value::Bool(true));
    map.insert("locale".into(), Value::String(locale));
    map.insert("options".into(), options);
    attach_bound_method(&mut map, "format", dtf_format_method_native);
    Value::Object(map)
}

fn nf_format_method_native(args: &[Value], _env: &mut Environment) -> Result<Value, String> {
    let fmt = args.first().ok_or("NumberFormat.format(value)")?;
    let value = args.get(1).ok_or("NumberFormat.format(value)")?;
    let (locale, opts) = nf_from_object(fmt)?;
    Ok(Value::String(format_number_value(locale, &opts, value)?))
}

fn nf_format_parts_native(args: &[Value], _env: &mut Environment) -> Result<Value, String> {
    let fmt = args.first().ok_or("NumberFormat.formatToParts(value)")?;
    let value = args.get(1).ok_or("NumberFormat.formatToParts(value)")?;
    let (locale, opts) = nf_from_object(fmt)?;
    let text = format_number_value(locale, &opts, value)?;
    Ok(Value::Array(vec![Value::Object(HashMap::from([
        ("type".into(), Value::String("literal".into())),
        ("value".into(), Value::String(text)),
    ]))]))
}

fn dtf_format_method_native(args: &[Value], _env: &mut Environment) -> Result<Value, String> {
    let fmt = args.first().ok_or("DateTimeFormat.format(date)")?;
    let value = args.get(1).ok_or("DateTimeFormat.format(date)")?;
    let (locale, opts) = dtf_from_object(fmt)?;
    Ok(Value::String(format_date_time_value(locale, &opts, value)?))
}

fn number_format_ctor_native(args: &[Value], _env: &mut Environment) -> Result<Value, String> {
    let locale = match args.first() {
        Some(Value::String(s)) => s.clone(),
        None => "en-US".into(),
        other => return Err(format!("Intl.NumberFormat locale must be string, got {:?}", other)),
    };
    let options = args.get(1).cloned().unwrap_or(Value::Object(HashMap::new()));
    Ok(create_number_format(locale, options))
}

fn date_time_format_ctor_native(args: &[Value], _env: &mut Environment) -> Result<Value, String> {
    let locale = match args.first() {
        Some(Value::String(s)) => s.clone(),
        None => "en-US".into(),
        other => return Err(format!("Intl.DateTimeFormat locale must be string, got {:?}", other)),
    };
    let options = args.get(1).cloned().unwrap_or(Value::Object(HashMap::new()));
    Ok(create_date_time_format(locale, options))
}

pub fn is_number_format_ctor(v: &Value) -> bool {
    matches!(
        v,
        Value::Object(m) if matches!(m.get("__kab_intl_nf_ctor"), Some(Value::Bool(true)))
    )
}

pub fn is_date_time_format_ctor(v: &Value) -> bool {
    matches!(
        v,
        Value::Object(m) if matches!(m.get("__kab_intl_dtf_ctor"), Some(Value::Bool(true)))
    )
}

pub fn try_number_format_ctor_call(
    callee: &Value,
    args: &[Value],
    env: &mut Environment,
) -> Option<Result<Value, String>> {
    if is_number_format_ctor(callee) {
        Some(number_format_ctor_native(args, env))
    } else {
        None
    }
}

pub fn try_date_time_format_ctor_call(
    callee: &Value,
    args: &[Value],
    env: &mut Environment,
) -> Option<Result<Value, String>> {
    if is_date_time_format_ctor(callee) {
        Some(date_time_format_ctor_native(args, env))
    } else {
        None
    }
}

fn intl_number_format_new_native(args: &[Value], env: &mut Environment) -> Result<Value, String> {
    number_format_ctor_native(args, env)
}

fn intl_number_format_format_native(args: &[Value], env: &mut Environment) -> Result<Value, String> {
    let fmt = args.first().ok_or("intl_number_format_format(fmt, value)")?;
    let value = args.get(1).ok_or("intl_number_format_format(fmt, value)")?;
    nf_format_method_native(&[fmt.clone(), value.clone()], env)
}

fn intl_date_time_format_new_native(args: &[Value], env: &mut Environment) -> Result<Value, String> {
    date_time_format_ctor_native(args, env)
}

fn intl_date_time_format_format_native(args: &[Value], env: &mut Environment) -> Result<Value, String> {
    let fmt = args.first().ok_or("intl_date_time_format_format(fmt, date)")?;
    let value = args.get(1).ok_or("intl_date_time_format_format(fmt, date)")?;
    dtf_format_method_native(&[fmt.clone(), value.clone()], env)
}

pub fn build_intl_namespace() -> Value {
    let mut nf_ctor = HashMap::new();
    nf_ctor.insert("__kab_intl_nf_ctor".into(), Value::Bool(true));
    nf_ctor.insert(
        "supportedLocalesOf".into(),
        Value::NativeFunction(intl_supported_locales_native),
    );

    let mut dtf_ctor = HashMap::new();
    dtf_ctor.insert("__kab_intl_dtf_ctor".into(), Value::Bool(true));
    dtf_ctor.insert(
        "supportedLocalesOf".into(),
        Value::NativeFunction(intl_supported_locales_native),
    );

    let mut intl = HashMap::new();
    intl.insert("__kab_intl".into(), Value::Bool(true));
    intl.insert("NumberFormat".into(), Value::Object(nf_ctor));
    intl.insert("DateTimeFormat".into(), Value::Object(dtf_ctor));
    Value::Object(intl)
}

fn intl_supported_locales_native(args: &[Value], _env: &mut Environment) -> Result<Value, String> {
    let locales = match args.first() {
        Some(Value::Array(items)) => items
            .iter()
            .filter_map(|v| match v {
                Value::String(s) => Some(Value::String(s.clone())),
                _ => None,
            })
            .collect(),
        Some(Value::String(s)) => vec![Value::String(s.clone())],
        None => vec![Value::String("en-US".into())],
        _ => return Err("supportedLocalesOf expects locale list".into()),
    };
    Ok(Value::Array(locales))
}

pub fn register_intl(env: &mut Environment) {
    env.set("Intl".to_string(), build_intl_namespace());
    let fns: &[(&str, fn(&[Value], &mut Environment) -> Result<Value, String>)] = &[
        ("intl_number_format_new", intl_number_format_new_native),
        ("intl_number_format_format", intl_number_format_format_native),
        ("intl_date_time_format_new", intl_date_time_format_new_native),
        (
            "intl_date_time_format_format",
            intl_date_time_format_format_native,
        ),
    ];
    for (name, func) in fns {
        env.set(name.to_string(), Value::NativeFunction(*func));
    }
}
