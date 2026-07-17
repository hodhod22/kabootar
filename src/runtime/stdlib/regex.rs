//! RegExp — `regexp_new` / `regexp_test` / `regexp_exec` and legacy `regex_*` helpers.

use crate::value::{Environment, Value};
use fancy_regex::{Captures, Regex};
use std::cell::RefCell;
use std::collections::HashMap;
use std::sync::atomic::{AtomicU64, Ordering};

const REGEXP_MARKER: &str = "__kab_regexp";
const REGEXP_ID: &str = "__kab_id";

static NEXT_REGEXP_ID: AtomicU64 = AtomicU64::new(1);

thread_local! {
    static REGEXP_STORE: RefCell<HashMap<u64, RegExpRecord>> = RefCell::new(HashMap::new());
}

struct RegExpRecord {
    compiled: Regex,
    flags: String,
    last_index: usize,
}

pub fn is_regexp_value(v: &Value) -> bool {
    match v {
        Value::Object(map) => matches!(map.get(REGEXP_MARKER), Some(Value::Bool(true))),
        _ => false,
    }
}

fn regexp_id(v: &Value) -> Result<u64, String> {
    let Value::Object(map) = v else {
        return Err("expected RegExp".into());
    };
    if !is_regexp_value(v) {
        return Err("expected RegExp".into());
    }
    match map.get(REGEXP_ID) {
        Some(Value::Number(n)) if *n > 0 => Ok(*n as u64),
        _ => Err("invalid RegExp handle".into()),
    }
}

fn with_record<R>(id: u64, f: impl FnOnce(&mut RegExpRecord) -> R) -> Result<R, String> {
    REGEXP_STORE.with(|store| {
        let mut map = store.borrow_mut();
        let record = map
            .get_mut(&id)
            .ok_or_else(|| format!("invalid RegExp id {id}"))?;
        Ok(f(record))
    })
}

fn normalize_flags(flags: &str) -> String {
    let mut out = String::new();
    for c in flags.chars() {
        if "gimsuyd".contains(c) && !out.contains(c) {
            out.push(c);
        }
    }
    out
}

fn wrap_pattern(pattern: &str, flags: &str) -> String {
    let flags = normalize_flags(flags);
    let mut inline = String::new();
    if flags.contains('i') {
        inline.push('i');
    }
    if flags.contains('m') {
        inline.push('m');
    }
    if flags.contains('s') {
        inline.push('s');
    }
    if flags.contains('u') {
        inline.push('u');
    }
    if inline.is_empty() {
        pattern.to_string()
    } else {
        format!("(?{inline}){pattern}")
    }
}

pub fn compile_regex(pattern: &str, flags: &str) -> Result<Regex, String> {
    let wrapped = wrap_pattern(pattern, flags);
    Regex::new(&wrapped).map_err(|e| format!("Invalid RegExp: {e}"))
}

fn regex_is_match(re: &Regex, text: &str) -> Result<bool, String> {
    re.is_match(text).map_err(|e| format!("RegExp match error: {e}"))
}

fn regex_captures<'t>(re: &Regex, text: &'t str) -> Result<Option<Captures<'t>>, String> {
    re.captures(text)
        .map_err(|e| format!("RegExp match error: {e}"))
}

fn regex_find_at<'t>(
    re: &Regex,
    text: &'t str,
    start: usize,
) -> Result<Option<(usize, usize, Vec<String>)>, String> {
    if start > text.len() {
        return Ok(None);
    }
    let haystack = &text[start..];
    let Some(caps) = regex_captures(re, haystack)? else {
        return Ok(None);
    };
    let m = caps.get(0).ok_or("RegExp match missing group 0")?;
    let abs_start = start + m.start();
    let abs_end = start + m.end();
    let mut groups = vec![m.as_str().to_string()];
    for i in 1..caps.len() {
        groups.push(
            caps.get(i)
                .map(|g| g.as_str().to_string())
                .unwrap_or_default(),
        );
    }
    Ok(Some((abs_start, abs_end, groups)))
}

fn parse_pattern_and_flags(raw: &str) -> (String, String) {
    let s = raw.trim();
    if s.starts_with('/') && s.len() > 1 {
        if let Some(end) = s[1..].find('/') {
            let pattern = s[1..1 + end].to_string();
            let flags = s[1 + end + 1..].to_string();
            return (pattern, normalize_flags(&flags));
        }
    }
    (s.to_string(), String::new())
}

fn make_regexp_object(id: u64, source: &str, flags: &str) -> Value {
    let flags = normalize_flags(flags);
    let mut map = HashMap::new();
    map.insert(REGEXP_MARKER.into(), Value::Bool(true));
    map.insert(REGEXP_ID.into(), Value::Number(id as i64));
    map.insert("source".into(), Value::String(source.to_string()));
    map.insert("flags".into(), Value::String(flags.clone()));
    map.insert("lastIndex".into(), Value::Number(0));
    map.insert("global".into(), Value::Bool(flags.contains('g')));
    map.insert("dotAll".into(), Value::Bool(flags.contains('s')));
    map.insert("unicode".into(), Value::Bool(flags.contains('u')));
    map.insert("ignoreCase".into(), Value::Bool(flags.contains('i')));
    map.insert("multiline".into(), Value::Bool(flags.contains('m')));
    map.insert("sticky".into(), Value::Bool(flags.contains('y')));
    Value::Object(map)
}

fn regexp_new_native(args: &[Value], _env: &mut Environment) -> Result<Value, String> {
    let (pattern, flags) = match args.first() {
        Some(Value::String(s)) => {
            if s.starts_with('/') {
                parse_pattern_and_flags(s)
            } else {
                (s.clone(), String::new())
            }
        }
        _ => return Err("regexp_new(pattern, flags?)".into()),
    };
    let flags = match args.get(1) {
        Some(Value::String(s)) => normalize_flags(s),
        None => flags,
        _ => return Err("regexp_new(pattern, flags?) flags must be string".into()),
    };
    let compiled = compile_regex(&pattern, &flags)?;
    let id = NEXT_REGEXP_ID.fetch_add(1, Ordering::Relaxed);
    REGEXP_STORE.with(|store| {
        store.borrow_mut().insert(
            id,
            RegExpRecord {
                compiled,
                flags: flags.clone(),
                last_index: 0,
            },
        );
    });
    Ok(make_regexp_object(id, &pattern, &flags))
}

fn exec_on_text(record: &mut RegExpRecord, text: &str) -> Result<Option<(usize, usize, Vec<String>)>, String> {
    let global = record.flags.contains('g');
    let sticky = record.flags.contains('y');
    let start = if global || sticky {
        record.last_index
    } else {
        0
    };
    let matched = regex_find_at(&record.compiled, text, start)?;
    if let Some((_, abs_end, _)) = &matched {
        if global {
            let mlen = abs_end.saturating_sub(start);
            let next = if mlen == 0 { abs_end + 1 } else { *abs_end };
            record.last_index = next.min(text.len());
        } else if sticky {
            record.last_index = *abs_end;
        }
    } else if global && start > text.len() {
        record.last_index = 0;
    }
    Ok(matched)
}

fn regexp_test_native(args: &[Value], _env: &mut Environment) -> Result<Value, String> {
    let re = args.first().ok_or("regexp_test(re, text)")?;
    let text = match args.get(1) {
        Some(Value::String(s)) => s.as_str(),
        _ => return Err("regexp_test(re, text)".into()),
    };
    let id = regexp_id(re)?;
    let matched = with_record(id, |record| exec_on_text(record, text))??.is_some();
    Ok(Value::Bool(matched))
}

fn regexp_exec_native(args: &[Value], _env: &mut Environment) -> Result<Value, String> {
    let re = args.first().ok_or("regexp_exec(re, text)")?;
    let text = match args.get(1) {
        Some(Value::String(s)) => s.as_str(),
        _ => return Err("regexp_exec(re, text)".into()),
    };
    let id = regexp_id(re)?;
    let groups = with_record(id, |record| exec_on_text(record, text))??;
    if let Some((_, _, groups)) = groups {
        Ok(Value::Array(
            groups.into_iter().map(Value::String).collect(),
        ))
    } else {
        Ok(Value::Null)
    }
}

fn is_regexp_native(args: &[Value], _env: &mut Environment) -> Result<Value, String> {
    let v = args.first().ok_or("is_regexp(v)")?;
    Ok(Value::Bool(is_regexp_value(v)))
}

fn regex_test_native(args: &[Value], _env: &mut Environment) -> Result<Value, String> {
    let (pattern, flags) = match args.first() {
        Some(Value::String(s)) => parse_pattern_and_flags(s),
        _ => return Err("regex_test(pattern, text)".into()),
    };
    let text = match args.get(1) {
        Some(Value::String(s)) => s.as_str(),
        _ => return Err("regex_test(pattern, text)".into()),
    };
    let re = compile_regex(&pattern, &flags)?;
    Ok(Value::Bool(regex_is_match(&re, text)?))
}

fn regex_match_native(args: &[Value], _env: &mut Environment) -> Result<Value, String> {
    let (pattern, flags) = match args.first() {
        Some(Value::String(s)) => parse_pattern_and_flags(s),
        _ => return Err("regex_match(pattern, text)".into()),
    };
    let text = match args.get(1) {
        Some(Value::String(s)) => s.as_str(),
        _ => return Err("regex_match(pattern, text)".into()),
    };
    let re = compile_regex(&pattern, &flags)?;
    if let Some(caps) = regex_captures(&re, text)? {
        let mut groups = Vec::new();
        groups.push(
            caps.get(0)
                .map(|m| m.as_str().to_string())
                .unwrap_or_default(),
        );
        for i in 1..caps.len() {
            groups.push(
                caps.get(i)
                    .map(|g| g.as_str().to_string())
                    .unwrap_or_default(),
            );
        }
        Ok(Value::Array(groups.into_iter().map(Value::String).collect()))
    } else {
        Ok(Value::Null)
    }
}

fn regex_replace_native(args: &[Value], _env: &mut Environment) -> Result<Value, String> {
    let (pattern, flags) = match args.first() {
        Some(Value::String(s)) => parse_pattern_and_flags(s),
        _ => return Err("regex_replace(pattern, text, repl)".into()),
    };
    let text = match args.get(1) {
        Some(Value::String(s)) => s.clone(),
        _ => return Err("regex_replace(pattern, text, repl)".into()),
    };
    let repl = match args.get(2) {
        Some(Value::String(s)) => s.clone(),
        _ => String::new(),
    };
    let re = compile_regex(&pattern, &flags)?;
    if let Some((start, end, _)) = regex_find_at(&re, &text, 0)? {
        let mut out = String::with_capacity(text.len());
        out.push_str(&text[..start]);
        out.push_str(&repl);
        out.push_str(&text[end..]);
        Ok(Value::String(out))
    } else {
        Ok(Value::String(text))
    }
}

fn regex_search_native(args: &[Value], _env: &mut Environment) -> Result<Value, String> {
    let (pattern, flags) = match args.first() {
        Some(Value::String(s)) => parse_pattern_and_flags(s),
        _ => return Err("regex_search(pattern, text)".into()),
    };
    let text = match args.get(1) {
        Some(Value::String(s)) => s.as_str(),
        _ => return Err("regex_search(pattern, text)".into()),
    };
    let re = compile_regex(&pattern, &flags)?;
    if let Some((start, _, _)) = regex_find_at(&re, text, 0)? {
        Ok(Value::Number(start as i64))
    } else {
        Ok(Value::Number(-1))
    }
}

fn regex_replace_all_native(args: &[Value], _env: &mut Environment) -> Result<Value, String> {
    let (pattern, flags) = match args.first() {
        Some(Value::String(s)) => parse_pattern_and_flags(s),
        _ => return Err("regex_replace_all(pattern, text, repl)".into()),
    };
    let text = match args.get(1) {
        Some(Value::String(s)) => s.clone(),
        _ => return Err("regex_replace_all(pattern, text, repl)".into()),
    };
    let repl = match args.get(2) {
        Some(Value::String(s)) => s.clone(),
        _ => String::new(),
    };
    let re = compile_regex(&pattern, &flags)?;
    Ok(Value::String(
        re.replace_all(&text, repl.as_str()).into_owned(),
    ))
}

fn regex_escape_native(args: &[Value], _env: &mut Environment) -> Result<Value, String> {
    let s = match args.first() {
        Some(Value::String(text)) => text.as_str(),
        _ => return Err("regex_escape(text)".into()),
    };
    Ok(Value::String(regex::escape(s)))
}

/// JS `String.prototype.search` — index of first regex match, or -1.
pub fn text_search_regex(pattern: &str, text: &str) -> Result<i64, String> {
    let (pattern, flags) = parse_pattern_and_flags(pattern);
    let re = compile_regex(&pattern, &flags)?;
    if let Some((start, _, _)) = regex_find_at(&re, text, 0)? {
        Ok(start as i64)
    } else {
        Ok(-1)
    }
}

/// JS `String.prototype.matchAll` — all matches as array of capture arrays (global flag implied).
pub fn text_match_all_regex(pattern: &str, text: &str) -> Result<Vec<Vec<String>>, String> {
    let (pattern, flags) = parse_pattern_and_flags(pattern);
    let mut flags = normalize_flags(&flags);
    if !flags.contains('g') {
        flags.push('g');
    }
    let re = compile_regex(&pattern, &flags)?;
    let mut out = Vec::new();
    let mut start = 0usize;
    loop {
        let Some((abs_start, abs_end, groups)) = regex_find_at(&re, text, start)? else {
            break;
        };
        out.push(groups);
        if abs_end <= abs_start {
            start = abs_start.saturating_add(1);
        } else {
            start = abs_end;
        }
        if start > text.len() {
            break;
        }
    }
    Ok(out)
}

/// JS `String.prototype.match` — first match as array (index 0 = full match), or null.
pub fn text_match_regex(pattern: &str, text: &str) -> Result<Option<Value>, String> {
    let (pattern, flags) = parse_pattern_and_flags(pattern);
    let re = compile_regex(&pattern, &flags)?;
    if let Some((_, _, groups)) = regex_find_at(&re, text, 0)? {
        let items: Vec<Value> = groups.into_iter().map(Value::String).collect();
        Ok(Some(Value::Array(items)))
    } else {
        Ok(None)
    }
}

pub fn register_regex(env: &mut Environment) {
    let fns: &[(&str, fn(&[Value], &mut Environment) -> Result<Value, String>)] = &[
        ("regexp_new", regexp_new_native),
        ("regexp_test", regexp_test_native),
        ("regexp_exec", regexp_exec_native),
        ("is_regexp", is_regexp_native),
        ("regex_test", regex_test_native),
        ("regex_match", regex_match_native),
        ("regex_replace", regex_replace_native),
        ("regex_replace_all", regex_replace_all_native),
        ("regex_search", regex_search_native),
        ("regex_escape", regex_escape_native),
        ("RegExp_escape", regex_escape_native),
    ];
    for (name, func) in fns {
        env.set(name.to_string(), Value::NativeFunction(*func));
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn dot_all_matches_newline() {
        let re = compile_regex("a.b", "s").unwrap();
        assert!(regex_is_match(&re, "a\nb").unwrap());
    }

    #[test]
    fn lookbehind_matches() {
        let re = compile_regex(r"(?<=@)\w+", "").unwrap();
        let caps = regex_captures(&re, "user@host").unwrap().unwrap();
        assert_eq!(caps.get(0).unwrap().as_str(), "host");
    }

    #[test]
    fn match_all_finds_all_digits() {
        let all = text_match_all_regex(r"\d", "a1b22").unwrap();
        assert_eq!(all.len(), 3);
        assert_eq!(all[0][0], "1");
        assert_eq!(all[2][0], "2");
    }
}
