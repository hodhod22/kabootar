//! Kabootar standard library — JSON, collections, arrays, strings, regex, types.

mod array;
mod date;
pub mod disposable;
pub mod deno_permissions;
pub mod deno_testing;
pub mod deno_wave_b;
pub mod intl;
pub mod temporal;
mod encoding;
pub mod error;
pub mod json;
pub mod map;
mod math;
mod number;
pub mod abort;
pub mod object;
pub mod symbol;
pub mod descriptor;
mod fetch;
pub mod opt;
pub mod promise;
pub mod proxy;
pub mod reflect;
pub mod weak;
pub mod weak_collections;
pub mod async_iterator;
pub mod generator;
pub mod iterator;
pub mod iterator_class;
mod regex;
mod string;
mod types;
mod text_encoding;
pub mod bigint;
mod base64;
mod web_api;
mod url;
pub mod deno;
mod fs;

use crate::value::{Environment, Value};

pub use array::{array_to_locale_string_method, entries_native, values_native};
pub(crate) use string::{
    str_locale_compare_method, str_match_all_method, str_to_locale_string_method,
};
pub use map::{is_map_value, is_set_value};
pub use types::typeof_name;

fn json_parse_native(args: &[Value], _env: &mut Environment) -> Result<Value, String> {
    let s = match args.first() {
        Some(Value::String(text)) => text.as_str(),
        _ => return Err("json_parse(text)".into()),
    };
    json::parse(s)
}

fn json_stringify_native(args: &[Value], _env: &mut Environment) -> Result<Value, String> {
    let v = args.first().ok_or("json_stringify(value)")?;
    let indent = match args.get(1) {
        Some(Value::Number(n)) if (0..=8).contains(n) => Some(*n as usize),
        None => None,
        _ => return Err("json_stringify(value, indent?) indent must be 0..8".into()),
    };
    Ok(Value::String(json::stringify_pretty(v, indent)))
}

fn std_info_native(_args: &[Value], _env: &mut Environment) -> Result<Value, String> {
    let caps = vec![
        "json_parse",
        "json_stringify",
        "reduce",
        "for_each",
        "find",
        "slice",
        "sort",
        "reverse",
        "join",
        "shift",
        "unshift",
        "splice",
        "map_new",
        "set_new",
        "regex_test",
        "type_assert",
        "entries",
        "values",
        "concat",
        "flat",
        "floor",
        "ceil",
        "round",
        "abs",
        "min",
        "max",
        "sqrt",
        "pow",
        "random",
        "parse_int",
        "parse_float",
        "assign",
        "has_key",
        "delete_prop",
        "clone_shallow",
        "repeat",
        "pad_start",
        "pad_end",
        "flat_map",
        "array_is_array",
        "encode_uri",
        "decode_uri",
        "encode_uri_component",
        "decode_uri_component",
        "date_now",
        "date_parse",
        "date_format",
        "date_iso",
        "sleep_ms",
        "set_timeout",
        "clear_timeout",
        "object_keys",
        "char_at",
        "str_includes",
        "str_last_index_of",
        "trim_start",
        "trim_end",
        "last_index_of",
        "find_last",
        "find_last_index",
        "object_values",
        "object_entries",
        "set_interval",
        "clear_interval",
        "at",
        "fill",
        "copy_within",
        "to_spliced",
        "str_slice",
        "string_includes",
        "str_normalize",
        "to_fixed",
        "object_assign",
        "object_has_own",
        "regex_search",
        "regex_replace_all",
        "array_from",
        "array_with",
        "object_from_entries",
        "structured_clone",
        "is_integer",
        "to_exponential",
        "to_precision",
        "char_code_at",
        "from_char_code",
        "pi",
        "log",
        "log2",
        "exp",
        "sin",
        "cos",
        "hypot",
        "sinh",
        "cosh",
        "tanh",
        "str_match",
        "str_search",
        "str_match_all",
        "str_to_locale_string",
        "str_locale_compare",
    ];
    let items: Vec<Value> = caps.into_iter().map(|s| Value::String(s.into())).collect();
    let mut info = std::collections::HashMap::new();
    info.insert("version".into(), Value::String("1.0".into()));
    info.insert("capabilities".into(), Value::Array(items));
    Ok(Value::Object(info))
}

/// Register all stdlib natives on the global environment.
pub fn register_globals(env: &mut Environment) {
    env.set("json_parse".to_string(), Value::NativeFunction(json_parse_native));
    env.set(
        "json_stringify".to_string(),
        Value::NativeFunction(json_stringify_native),
    );
    env.set("std_info".to_string(), Value::NativeFunction(std_info_native));
    array::register_array(env);
    string::register_string(env);
    map::register_map_set(env);
    iterator_class::register_iterator_classes(env);
    regex::register_regex(env);
    types::register_types(env);
    math::register_math(env);
    number::register_number(env);
    object::register_object(env);
    symbol::register_symbol(env);
    proxy::register_proxy(env);
    reflect::register_reflect(env);
    weak::register_weak(env);
    weak_collections::register_weak_collections(env);
    encoding::register_encoding(env);
    date::register_date(env);
    promise::register_promise(env);
    fetch::register_fetch(env);
    opt::register_opt(env);
    abort::register_abort(env);
    error::register_error(env);
    text_encoding::register_text_encoding(env);
    web_api::register_web_api(env);
    bigint::register_bigint(env);
    disposable::register_disposable(env);
    intl::register_intl(env);
    temporal::register_temporal(env);
    deno::register_deno(env);
    fs::register_fs(env);
    url::register_url(env);
    crate::runtime::node_compat::register_node_globals(env);
    crate::runtime::shared_memory::register_shared_memory(env);
}

