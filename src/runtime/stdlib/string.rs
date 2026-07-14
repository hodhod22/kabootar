//! String helpers.

use crate::value::{Environment, Value};
use unicode_normalization::UnicodeNormalization;
use std::cmp::Ordering;

fn str_arg(v: &Value) -> Result<&str, String> {
    match v {
        Value::String(s) => Ok(s.as_str()),
        _ => Err("expected string".into()),
    }
}

fn substring_native(args: &[Value], _env: &mut Environment) -> Result<Value, String> {
    let s = str_arg(args.first().ok_or("substring(s, start, end?)")?)?;
    let chars: Vec<char> = s.chars().collect();
    let len = chars.len() as i64;
    let start = match args.get(1) {
        Some(Value::Number(n)) => norm(*n, len),
        _ => 0,
    };
    let end = match args.get(2) {
        Some(Value::Number(n)) => norm(*n, len),
        _ => len,
    };
    let (a, b) = if start <= end {
        (start as usize, end as usize)
    } else {
        (end as usize, start as usize)
    };
    let out: String = chars
        .get(a..b.min(chars.len()))
        .unwrap_or(&[])
        .iter()
        .collect();
    Ok(Value::String(out))
}

fn norm(i: i64, len: i64) -> i64 {
    if i < 0 {
        (len + i).max(0)
    } else {
        i.min(len)
    }
}

fn trim_native(args: &[Value], _env: &mut Environment) -> Result<Value, String> {
    let s = str_arg(args.first().ok_or("trim(s)")?)?;
    Ok(Value::String(s.trim().to_string()))
}

fn split_native(args: &[Value], _env: &mut Environment) -> Result<Value, String> {
    let s = str_arg(args.first().ok_or("split(s, sep)")?)?;
    let sep = str_arg(args.get(1).ok_or("split(s, sep)")?)?;
    let parts: Vec<Value> = if sep.is_empty() {
        s.chars().map(|c| Value::String(c.to_string())).collect()
    } else {
        s.split(sep).map(|p| Value::String(p.to_string())).collect()
    };
    Ok(Value::Array(parts))
}

fn starts_with_native(args: &[Value], _env: &mut Environment) -> Result<Value, String> {
    let s = str_arg(args.first().ok_or("starts_with(s, prefix)")?)?;
    let prefix = str_arg(args.get(1).ok_or("starts_with(s, prefix)")?)?;
    Ok(Value::Bool(s.starts_with(prefix)))
}

fn ends_with_native(args: &[Value], _env: &mut Environment) -> Result<Value, String> {
    let s = str_arg(args.first().ok_or("ends_with(s, suffix)")?)?;
    let suffix = str_arg(args.get(1).ok_or("ends_with(s, suffix)")?)?;
    Ok(Value::Bool(s.ends_with(suffix)))
}

fn replace_native(args: &[Value], _env: &mut Environment) -> Result<Value, String> {
    let s = str_arg(args.first().ok_or("replace(s, from, to)")?)?;
    let from = str_arg(args.get(1).ok_or("replace(s, from, to)")?)?;
    let to = match args.get(2) {
        Some(Value::String(t)) => t.as_str(),
        _ => "",
    };
    Ok(Value::String(s.replacen(from, to, 1)))
}

fn replace_all_native(args: &[Value], _env: &mut Environment) -> Result<Value, String> {
    let s = str_arg(args.first().ok_or("replace_all(s, from, to)")?)?;
    let from = str_arg(args.get(1).ok_or("replace_all(s, from, to)")?)?;
    let to = match args.get(2) {
        Some(Value::String(t)) => t.as_str(),
        _ => "",
    };
    Ok(Value::String(s.replace(from, to)))
}

fn index_of_str_native(args: &[Value], _env: &mut Environment) -> Result<Value, String> {
    let s = str_arg(args.first().ok_or("str_index_of(s, needle, from?)")?)?;
    let needle = str_arg(args.get(1).ok_or("str_index_of(s, needle, from?)")?)?;
    let from = match args.get(2) {
        Some(Value::Number(n)) if *n >= 0 => *n as usize,
        _ => 0,
    };
    let hay = if from < s.len() { &s[from..] } else { "" };
    match hay.find(needle) {
        Some(i) => Ok(Value::Number((from + i) as i64)),
        None => Ok(Value::Number(-1)),
    }
}

fn to_upper_native(args: &[Value], _env: &mut Environment) -> Result<Value, String> {
    let s = str_arg(args.first().ok_or("to_upper(s)")?)?;
    Ok(Value::String(s.to_uppercase()))
}

fn to_lower_native(args: &[Value], _env: &mut Environment) -> Result<Value, String> {
    let s = str_arg(args.first().ok_or("to_lower(s)")?)?;
    Ok(Value::String(s.to_lowercase()))
}

fn repeat_native(args: &[Value], _env: &mut Environment) -> Result<Value, String> {
    let s = str_arg(args.first().ok_or("repeat(s, count)")?)?;
    let count = match args.get(1) {
        Some(Value::Number(n)) if *n >= 0 => *n as usize,
        _ => return Err("repeat(s, count) expects non-negative count".into()),
    };
    Ok(Value::String(s.repeat(count)))
}

fn pad_start_native(args: &[Value], _env: &mut Environment) -> Result<Value, String> {
    let s = str_arg(args.first().ok_or("pad_start(s, len, fill?)")?)?;
    let target = match args.get(1) {
        Some(Value::Number(n)) if *n >= 0 => *n as usize,
        _ => return Err("pad_start(s, len, fill?) expects non-negative length".into()),
    };
    let fill = match args.get(2) {
        Some(Value::String(f)) if !f.is_empty() => f.as_str(),
        _ => " ",
    };
    if s.chars().count() >= target {
        return Ok(Value::String(s.to_string()));
    }
    let pad_len = target - s.chars().count();
    let mut pad = String::new();
    while pad.chars().count() < pad_len {
        pad.push_str(fill);
    }
    let pad: String = pad.chars().take(pad_len).collect();
    Ok(Value::String(format!("{pad}{s}")))
}

fn pad_end_native(args: &[Value], _env: &mut Environment) -> Result<Value, String> {
    let s = str_arg(args.first().ok_or("pad_end(s, len, fill?)")?)?;
    let target = match args.get(1) {
        Some(Value::Number(n)) if *n >= 0 => *n as usize,
        _ => return Err("pad_end(s, len, fill?) expects non-negative length".into()),
    };
    let fill = match args.get(2) {
        Some(Value::String(f)) if !f.is_empty() => f.as_str(),
        _ => " ",
    };
    if s.chars().count() >= target {
        return Ok(Value::String(s.to_string()));
    }
    let pad_len = target - s.chars().count();
    let mut pad = String::new();
    while pad.chars().count() < pad_len {
        pad.push_str(fill);
    }
    let pad: String = pad.chars().take(pad_len).collect();
    Ok(Value::String(format!("{s}{pad}")))
}

fn char_at_native(args: &[Value], _env: &mut Environment) -> Result<Value, String> {
    let s = str_arg(args.first().ok_or("char_at(s, index)")?)?;
    let chars: Vec<char> = s.chars().collect();
    let idx = match args.get(1) {
        Some(Value::Number(n)) => norm(*n, chars.len() as i64) as usize,
        _ => 0,
    };
    Ok(chars
        .get(idx)
        .map(|c| Value::String(c.to_string()))
        .unwrap_or(Value::String(String::new())))
}

fn str_includes_native(args: &[Value], _env: &mut Environment) -> Result<Value, String> {
    let s = str_arg(args.first().ok_or("str_includes(s, needle)")?)?;
    let needle = str_arg(args.get(1).ok_or("str_includes(s, needle)")?)?;
    Ok(Value::Bool(s.contains(needle)))
}

fn str_last_index_of_native(args: &[Value], _env: &mut Environment) -> Result<Value, String> {
    let s = str_arg(args.first().ok_or("str_last_index_of(s, needle, from?)")?)?;
    let needle = str_arg(args.get(1).ok_or("str_last_index_of(s, needle, from?)")?)?;
    let from = match args.get(2) {
        Some(Value::Number(n)) if *n >= 0 => (*n as usize).min(s.len()),
        _ => s.len(),
    };
    let hay = &s[..from.min(s.len())];
    Ok(Value::Number(
        hay
            .rfind(needle)
            .map(|i| i as i64)
            .unwrap_or(-1),
    ))
}

fn trim_start_native(args: &[Value], _env: &mut Environment) -> Result<Value, String> {
    let s = str_arg(args.first().ok_or("trim_start(s)")?)?;
    Ok(Value::String(s.trim_start().to_string()))
}

fn trim_end_native(args: &[Value], _env: &mut Environment) -> Result<Value, String> {
    let s = str_arg(args.first().ok_or("trim_end(s)")?)?;
    Ok(Value::String(s.trim_end().to_string()))
}

fn str_slice_native(args: &[Value], env: &mut Environment) -> Result<Value, String> {
    substring_native(args, env)
}

fn str_normalize_native(args: &[Value], _env: &mut Environment) -> Result<Value, String> {
    let s = str_arg(args.first().ok_or("str_normalize(s, form?)")?)?;
    let form = match args.get(1) {
        Some(Value::String(f)) => f.as_str(),
        _ => "NFC",
    };
    match form.to_ascii_uppercase().as_str() {
        "NFC" => Ok(Value::String(s.nfc().collect::<String>())),
        "NFD" => Ok(Value::String(s.nfd().collect::<String>())),
        _ => Err(format!("str_normalize supports NFC/NFD, got {:?}", form)),
    }
}

fn char_code_at_native(args: &[Value], _env: &mut Environment) -> Result<Value, String> {
    let s = str_arg(args.first().ok_or("char_code_at(s, index)")?)?;
    let chars: Vec<char> = s.chars().collect();
    let idx = match args.get(1) {
        Some(Value::Number(n)) => norm(*n, chars.len() as i64) as usize,
        _ => 0,
    };
    Ok(chars
        .get(idx)
        .map(|c| Value::Number(*c as u32 as i64))
        .unwrap_or(Value::Float(f64::NAN)))
}

fn from_char_code_native(args: &[Value], _env: &mut Environment) -> Result<Value, String> {
    let mut out = String::new();
    for arg in args {
        let code = match arg {
            Value::Number(n) if (0..=0x10FFFF).contains(n) => *n as u32,
            _ => return Err("from_char_code(...codes) expects code points".into()),
        };
        let ch = char::from_u32(code).ok_or("from_char_code() invalid code point")?;
        out.push(ch);
    }
    Ok(Value::String(out))
}

fn code_point_at_native(args: &[Value], _env: &mut Environment) -> Result<Value, String> {
    let s = str_arg(args.first().ok_or("code_point_at(s, index)")?)?;
    let chars: Vec<char> = s.chars().collect();
    let idx = match args.get(1) {
        Some(Value::Number(n)) => norm(*n, chars.len() as i64) as usize,
        _ => 0,
    };
    Ok(chars
        .get(idx)
        .map(|c| Value::Number(*c as u32 as i64))
        .unwrap_or(Value::Undefined))
}

fn from_code_point_native(args: &[Value], _env: &mut Environment) -> Result<Value, String> {
    from_char_code_native(args, _env)
}

fn str_search_native(args: &[Value], _env: &mut Environment) -> Result<Value, String> {
    let text = str_arg(args.first().ok_or("str_search(text, pattern)")?)?;
    let pattern = str_arg(args.get(1).ok_or("str_search(text, pattern)")?)?;
    Ok(Value::Number(
        crate::runtime::stdlib::regex::text_search_regex(pattern, text)?,
    ))
}

fn str_match_native(args: &[Value], _env: &mut Environment) -> Result<Value, String> {
    let text = str_arg(args.first().ok_or("str_match(text, pattern)")?)?;
    let pattern = str_arg(args.get(1).ok_or("str_match(text, pattern)")?)?;
    match crate::runtime::stdlib::regex::text_match_regex(pattern, text)? {
        Some(v) => Ok(v),
        None => Ok(Value::Null),
    }
}

fn str_locale_compare_native(args: &[Value], _env: &mut Environment) -> Result<Value, String> {
    let a = str_arg(args.first().ok_or("str_locale_compare(a, b)")?)?;
    let b = str_arg(args.get(1).ok_or("str_locale_compare(a, b)")?)?;
    let ord = a.cmp(b);
    Ok(Value::Number(match ord {
        Ordering::Less => -1,
        Ordering::Equal => 0,
        Ordering::Greater => 1,
    }))
}

pub fn register_string(env: &mut Environment) {
    let fns: &[(&str, fn(&[Value], &mut Environment) -> Result<Value, String>)] = &[
        ("substring", substring_native),
        ("trim", trim_native),
        ("split", split_native),
        ("starts_with", starts_with_native),
        ("ends_with", ends_with_native),
        ("replace", replace_native),
        ("replace_all", replace_all_native),
        ("str_index_of", index_of_str_native),
        ("to_upper", to_upper_native),
        ("to_lower", to_lower_native),
        ("repeat", repeat_native),
        ("pad_start", pad_start_native),
        ("pad_end", pad_end_native),
        ("char_at", char_at_native),
        ("str_includes", str_includes_native),
        ("str_last_index_of", str_last_index_of_native),
        ("trim_start", trim_start_native),
        ("trim_end", trim_end_native),
        ("str_slice", str_slice_native),
        ("string_includes", str_includes_native),
        ("str_normalize", str_normalize_native),
        ("char_code_at", char_code_at_native),
        ("from_char_code", from_char_code_native),
        ("string_index_of", index_of_str_native),
        ("string_slice", str_slice_native),
        ("string_starts_with", starts_with_native),
        ("string_ends_with", ends_with_native),
        ("string_split", split_native),
        ("string_to_lower", to_lower_native),
        ("string_to_upper", to_upper_native),
        ("string_replace", replace_native),
        ("string_replace_all", replace_all_native),
        ("code_point_at", code_point_at_native),
        ("from_code_point", from_code_point_native),
        ("string_trim", trim_native),
        ("string_char_at", char_at_native),
        ("str_search", str_search_native),
        ("string_search", str_search_native),
        ("str_match", str_match_native),
        ("string_match", str_match_native),
        ("str_locale_compare", str_locale_compare_native),
        ("string_locale_compare", str_locale_compare_native),
    ];
    for (name, func) in fns {
        env.set(name.to_string(), Value::NativeFunction(*func));
    }
}
