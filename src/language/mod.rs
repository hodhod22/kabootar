mod generics_lsp;
mod symbols;

pub use symbols::{DefinitionSite, Symbol, SymbolKind};

use crate::lexer::tokenize;
use crate::parser::Parser;
pub use generics_lsp::{
    class_signature, collect_generic_templates, demangle_name, fn_signature, generic_word_at,
    hover_for_symbol, hover_from_ast, hover_member_at, in_type_arg_context, parse_for_lsp,
    symbol_covering,
    TYPE_NAMES,
};
pub use symbols::{definition_before, resolve_definition};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Severity {
    Error,
    Warning,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Diagnostic {
    pub line: u32,
    pub column: u32,
    pub len: u32,
    pub message: String,
    pub severity: Severity,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GotoTarget {
    pub line: u32,
    pub column: u32,
    pub len: u32,
    pub module: Option<String>,
}

pub fn analyze(source: &str) -> Vec<Diagnostic> {
    let tokens = match tokenize(source) {
        Ok(tokens) => tokens,
        Err(err) => {
            return vec![Diagnostic {
                line: err.line,
                column: err.column,
                len: 1,
                message: err.message,
                severity: Severity::Error,
            }];
        }
    };

    let mut parser = Parser::with_eof(tokens);
    match parser.parse_program() {
        Ok(_) => Vec::new(),
        Err(err) => vec![Diagnostic {
            line: err.span.line,
            column: err.span.column,
            len: err.span.len,
            message: err.message,
            severity: Severity::Error,
        }],
    }
}

pub fn goto_definition(source: &str, line: u32, column: u32) -> Option<GotoTarget> {
    let word = generic_word_at(line, column, source);
    if word.is_empty() {
        return None;
    }

    let (_, symbols) = parse_for_lsp(source)?;

    let use_line = line + 1;
    let use_column = column + 1;
    let sym = definition_before(&symbols, &word, use_line, use_column)?;
    let site = resolve_definition(sym);
    Some(GotoTarget {
        line: site.line,
        column: site.column,
        len: site.len,
        module: sym.module_name.clone(),
    })
}

pub fn word_at_position(line: u32, character: u32, text: &str) -> String {
    let line_text = text.lines().nth(line as usize).unwrap_or("");
    let col = (character as usize).min(line_text.len());
    let chars: Vec<char> = line_text.chars().collect();
    let mut start = col;
    while start > 0 && is_ident_char(chars[start - 1]) {
        start -= 1;
    }
    let mut end = col;
    while end < chars.len() && is_ident_char(chars[end]) {
        end += 1;
    }
    chars[start..end].iter().collect()
}

fn is_ident_char(c: char) -> bool {
    c.is_alphanumeric() || c == '_'
}

pub fn hover_at(source: &str, line: u32, column: u32) -> Option<String> {
    let word = generic_word_at(line, column, source);
    if word.is_empty() {
        return None;
    }

    if let Some(doc) = hover_word(&word) {
        return Some(doc);
    }

    if TYPE_NAMES.iter().any(|t| *t == word) {
        return Some(format!("Concrete type `{word}`"));
    }

    if let Some(text) = hover_member_at(source, line, column, &word) {
        return Some(text);
    }

    let (stmts, symbols) = parse_for_lsp(source)?;
    let use_line = line + 1;
    let use_column = column + 1;

    if let Some(sym) = symbol_covering(&symbols, &word, line, column) {
        if let Some(text) = hover_for_symbol(&word, sym, &stmts) {
            return Some(text);
        }
    }

    if let Some(sym) = definition_before(&symbols, &word, use_line, use_column) {
        if let Some(text) = hover_for_symbol(&word, sym, &stmts) {
            return Some(text);
        }
    }

    hover_from_ast(&word, &stmts)
}

pub fn completions_at(source: &str, line: u32, column: u32, prefix: &str) -> Vec<CompletionItem> {
    if in_type_arg_context(source, line, column) {
        let lower = prefix.to_lowercase();
        return TYPE_NAMES
            .iter()
            .filter(|t| prefix.is_empty() || t.starts_with(prefix) || t.to_lowercase().starts_with(&lower))
            .map(|t| CompletionItem {
                label: (*t).to_string(),
                detail: Some("concrete type".into()),
                kind: CompletionKind::Type,
            })
            .collect();
    }

    let mut items = completions(prefix);

    if let Some((stmts, _)) = parse_for_lsp(source) {
        for (name, detail) in collect_generic_templates(&stmts) {
            if prefix.is_empty()
                || name.starts_with(prefix)
                || name.to_lowercase().starts_with(&prefix.to_lowercase())
            {
                if !items.iter().any(|i| i.label == name) {
                    items.push(CompletionItem {
                        label: name,
                        detail: Some(detail),
                        kind: CompletionKind::Generic,
                    });
                }
            }
        }
    }

    items
}

pub fn completions(prefix: &str) -> Vec<CompletionItem> {
    let mut items: Vec<CompletionItem> = KEYWORDS
        .iter()
        .chain(BUILTINS.iter())
        .map(|label| CompletionItem {
            label: (*label).to_string(),
            detail: None,
            kind: CompletionKind::Keyword,
        })
        .collect();

    if prefix.is_empty() {
        return items;
    }

    let lower = prefix.to_lowercase();
    items.retain(|item| item.label.starts_with(prefix) || item.label.to_lowercase().starts_with(&lower));
    items
}

pub fn hover_word(word: &str) -> Option<String> {
    HOVER_DOCS
        .iter()
        .find(|(key, _)| *key == word)
        .map(|(_, text)| (*text).to_string())
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CompletionKind {
    Keyword,
    Function,
    Module,
    Type,
    Generic,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CompletionItem {
    pub label: String,
    pub detail: Option<String>,
    pub kind: CompletionKind,
}

const KEYWORDS: &[&str] = &[
    "async", "await", "yield", "break", "catch", "class", "const", "continue", "else", "extends", "false",
    "fn", "for", "if", "implements", "import", "in", "interface", "trait", "let", "match", "null", "return",
    "super", "this", "true", "try", "finally",
    "undefined", "while", "do", "switch", "case", "default", "of", "NaN", "Some", "None", "Ok", "Err",
    "pass", "raise", "assert", "with", "as", "is", "not",
];

const BUILTINS: &[&str] = &[
    "import_meta",
    "dynamic_import",
    "log",
    "is_null",
    "is_undefined",
    "is_nan",
    "len",
    "push",
    "object_array_push",
    "object_array_pop",
    "object_array_clear",
    "object_array_truncate",
    "pop",
    "map",
    "filter",
    "reduce",
    "reduce_right",
    "sort",
    "to_sorted",
    "reverse",
    "to_reversed",
    "join",
    "concat",
    "includes",
    "array_includes",
    "find",
    "find_index",
    "array_find_index",
    "slice",
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
    "json_parse",
    "json_stringify",
    "assign",
    "has_key",
    "delete_prop",
    "trim",
    "split",
    "repeat",
    "pad_start",
    "pad_end",
    "flat",
    "array_flat",
    "flat_map",
    "array_flat_map",
    "array_filter",
    "array_find",
    "array_some",
    "array_every",
    "array_of",
    "fetch",
    "response_text",
    "response_json",
    "response_ok",
    "queue_microtask",
    "code_point_at",
    "from_code_point",
    "set_symmetric_difference",
    "date_new",
    "date_get_time",
    "date_get_full_year",
    "date_set_time",
    "date_to_iso_string",
    "object_freeze",
    "object_seal",
    "object_prevent_extensions",
    "object_is_frozen",
    "object_is_sealed",
    "object_define_property",
    "object_get_own_property_descriptor",
    "object_get_own_property_symbols",
    "object_create",
    "object_get_parent",
    "object_set_parent",
    "object_get_own_property_names",
    "object_is_extensible",
    "log1p",
    "expm1",
    "string_trim",
    "string_char_at",
    "array_flat_map",
    "array_is_array",
    "encode_uri",
    "decode_uri",
    "encode_uri_component",
    "decode_uri_component",
    "date_now",
    "date_parse",
    "date_format",
    "date_iso",
    "trim_end",
    "char_at",
    "str_includes",
    "string_includes",
    "str_slice",
    "str_normalize",
    "str_last_index_of",
    "trim_start",
    "last_index_of",
    "find_last",
    "find_last_index",
    "at",
    "fill",
    "copy_within",
    "to_spliced",
    "to_fixed",
    "typeof",
    "keys",
    "object_keys",
    "object_values",
    "object_entries",
    "object_assign",
    "object_has_own",
    "object_has",
    "object_has_key",
    "object_delete",
    "object_delete_prop",
    "object_is",
    "object_clone_shallow",
    "regex_search",
    "regex_replace_all",
    "array_from",
    "array_with",
    "object_from_entries",
    "structured_clone",
    "is_integer",
    "number_is_integer",
    "to_exponential",
    "to_precision",
    "char_code_at",
    "from_char_code",
    "string_index_of",
    "string_slice",
    "string_starts_with",
    "string_ends_with",
    "string_split",
    "string_to_lower",
    "string_to_upper",
    "string_replace",
    "string_replace_all",
    "pi",
    "log",
    "log2",
    "log10",
    "exp",
    "sin",
    "cos",
    "tan",
    "hypot",
    "cbrt",
    "asin",
    "acos",
    "atan",
    "atan2",
    "map_new",
    "map_set",
    "map_get",
    "map_get_or_insert",
    "map_get_or_insert_computed",
    "map_has",
    "map_delete",
    "map_clear",
    "map_size",
    "map_keys",
    "map_values",
    "map_entries",
    "map_from_entries",
    "set_new",
    "set_add",
    "set_has",
    "set_delete",
    "set_clear",
    "set_size",
    "set_values",
    "set_for_each",
    "set_union",
    "set_intersection",
    "set_difference",
    "set_is_subset",
    "set_is_superset",
    "set_is_disjoint",
    "instanceof",
    "console_log",
    "console_warn",
    "console_error",
    "sleep_ticks",
    "sleep_ms",
    "set_timeout",
    "clear_timeout",
    "set_interval",
    "clear_interval",
    "map_for_each",
    "promise_resolve",
    "promise_reject",
    "is_promise",
    "promise_all_settled",
    "promise_any",
    "promise_then",
    "promise_catch",
    "promise_finally",
    "promise_new",
    "promise",
    "is_safe_integer",
    "number_is_finite",
    "number_is_nan",
    "number_to_string",
    "fmod",
    "imul",
    "clz32",
    "fround",
    "await_all",
    "promise_all",
    "promise_with_resolvers",
    "object_group_by",
    "group_by",
    "map_group_by",
    "for_of_items",
    "for_await_of_items",
    "array_from_async",
    "iterator_from",
    "iterator_from_async",
    "iterator_begin",
    "iterator_step",
    "iterator_close",
    "async_iterator_begin",
    "async_iterator_step",
    "async_iterator_close",
    "promise_try",
    "error_new",
    "error",
    "type_error",
    "is_error",
    "error_message",
    "url_new",
    "url_search_params_new",
    "usp_get",
    "usp_set",
    "reference_error",
    "range_error",
    "text_encode",
    "text_decode",
    "text_decoder_decode",
    "btoa",
    "atob",
    "BigInt",
    "performance",
    "crypto",
    "global_this",
    "globalThis",
    "iterator_map",
    "iterator_filter",
    "iterator_take",
    "iterator_skip",
    "iterator_zip",
    "iterator_enumerate",
    "iterator_chain",
    "iterator_flat_map",
    "iterator_drop_while",
    "iterator_take_while",
    "iterator_to_array",
    "iterator_reduce",
    "iterator_for_each",
    "iterator_find",
    "iterator_find_index",
    "iterator_includes",
    "iterator_accumulate",
    "iterator_pairwise",
    "range",
    "counter_new",
    "counter_inc",
    "counter_get",
    "counter_items",
    "defaultdict_new",
    "defaultdict_get",
    "defaultdict_set",
    "regex_escape",
    "regexp_new",
    "regexp_test",
    "regexp_exec",
    "is_regexp",
    "RegExp_escape",
    "env_get",
    "env_set",
    "env_has",
    "env_delete",
    "env_to_object",
    "Deno_env_get",
    "Deno_env_set",
    "stream_from_array",
    "stream_read",
    "stream_read_all",
    "stream_new",
    "stream_from_string",
    "stream_cancel",
    "stream_tee",
    "stream_pipe_to",
    "stream_locked",
    "stream_lock",
    "stream_unlock",
    "stream_desired_size",
    "stream_get_reader",
    "reader_read",
    "reader_release_lock",
    "reader_cancel",
    "stream_abort",
    "stream_state",
    "stream_enqueue",
    "stream_close_readable",
    "transform_stream_new",
    "byte_stream_new",
    "byte_stream_from_bytes",
    "byte_stream_read",
    "byte_stream_byob_read",
    "stream_transfer",
    "stream_from_transfer",
    "writable_stream_new",
    "writable_write",
    "writable_close",
    "writable_abort",
    "writable_read_all",
    "writable_get_writer",
    "writer_write",
    "writer_close",
    "writer_abort",
    "writer_release_lock",
    "writable_locked",
    "writable_desired_size",
    "cwd",
    "Deno_cwd",
    "chdir",
    "Deno_chdir",
    "read_text_file",
    "Deno_readTextFile",
    "write_text_file",
    "Deno_writeTextFile",
    "read_file",
    "Deno_readFile",
    "write_file",
    "Deno_writeFile",
    "read_dir",
    "Deno_readDir",
    "mkdir",
    "Deno_mkdir",
    "stat",
    "Deno_stat",
    "remove",
    "Deno_remove",
    "exists",
    "Deno_exists",
    "tcp_connect",
    "Deno_connect",
    "tcp_listen",
    "Deno_listen",
    "tcp_accept",
    "tcp_read",
    "tcp_read_bytes",
    "tcp_write",
    "tcp_write_bytes",
    "tcp_close",
    "tcp_start_tls",
    "Deno_startTls",
    "deno_run",
    "Deno_run",
    "run_command",
    "Deno_command",
    "resolve_dns",
    "Deno_resolveDns",
    "udp_bind",
    "udp_local_addr",
    "udp_send",
    "udp_recv",
    "udp_close",
    "ws_channel_pair",
    "ws_connect",
    "ws_link",
    "ws_send",
    "ws_recv",
    "request_method",
    "request_url",
    "request_body",
    "response_new",
    "serve_handler",
    "serve",
    "Deno_serve",
    "stream_read_async",
    "stream_read_all_async",
    "stream_pipe_to_async",
    "reader_read_async",
    "open_kv",
    "Deno_openKv",
    "open_kv_db",
    "kv_get",
    "kv_get_entry",
    "kv_get_version",
    "kv_set",
    "kv_delete",
    "kv_list",
    "kv_close",
    "kv_watch",
    "kv_listen",
    "kv_listen_recv",
    "kv_listen_close",
    "kv_listen_async",
    "kv_watch_async",
    "kv_atomic",
    "kv_list_entries",
    "kv_enqueue",
    "kv_dequeue",
    "unix_connect",
    "unix_listen",
    "unix_accept",
    "unix_read",
    "unix_write",
    "unix_close",
    "worker_new",
    "worker_start",
    "worker_post_message",
    "worker_recv",
    "worker_recv_async",
    "worker_onmessage",
    "worker_poll_async",
    "worker_start_file",
    "worker_join",
    "worker_terminate",
    "importScripts",
    "ffi_load",
    "ffi_call",
    "ffi_close",
    "npm_install",
    "npm_fetch",
    "jsr_fetch",
    "npm_resolve",
    "npm_parse_spec",
    "npm_list_cache",
    "npm_import",
    "ts_transpile",
    "ts_compile",
    "ts_compile_file",
    "Deno_emit",
    "ts_strip_types",
    "node_resolve",
    "node_list",
    "node_import",
    "node_path_join",
    "node_path_dirname",
    "node_path_basename",
    "node_path_extname",
    "sab_new",
    "shared_array_buffer_new",
    "sab_byte_length",
    "sab_transfer",
    "sab_from_transfer",
    "sab_is_shared",
    "uint8_array_new",
    "uint8_array_get",
    "uint8_array_set",
    "int32_array_new",
    "int32_array_get",
    "int32_array_set",
    "array_buffer_new",
    "float64_array_new",
    "float64_array_get",
    "float64_array_set",
    "float32_array_new",
    "float32_array_get",
    "float32_array_set",
    "data_view_new",
    "data_view_get_float64",
    "data_view_set_float64",
    "atomics_load",
    "atomics_store",
    "atomics_add",
    "atomics_sub",
    "atomics_and",
    "atomics_or",
    "atomics_xor",
    "atomics_exchange",
    "atomics_compare_exchange",
    "atomics_wait",
    "atomics_notify",
    "abort_controller_new",
    "abort_controller_abort",
    "abort_signal_aborted",
    "is_impl",
    "os_read_async",
    "os_write_async",
    "http_request_async",
    "sql_async",
    "http_fetch_async",
    "tls_add_ca",
    "tls_ca_only",
    "tls_pin",
    "tls_reset",
    "tls_cert_sha256",
    "sql",
    "db_open",
    "kml",
    "kdom_render",
    "os_read",
    "os_write",
    "os_exists",
    "os_list",
    "os_delete",
    "os_mkdir",
    "os_stat",
    "os_caps",
    "http_route",
    "http_request",
    "http_response",
    "http_status",
    "http_body",
    "http_headers",
    "http_header",
    "http_process",
    "http_serve_once",
    // http (efter `import "http"`)
    "ok",
    "created",
    "no_content",
    "not_found",
    "method_not_allowed",
    "method_get",
    "method_post",
    "method_put",
    "method_patch",
    "method_delete",
    "method_head",
    "method_options",
    "route_get",
    "route_post",
    "route_put",
    "route_patch",
    "route_delete",
    "route_head",
    "route_options",
    "request_get",
    "request_post",
    "request_put",
    "request_patch",
    "request_delete",
    "request_head",
    "request_options",
    "request_get_async",
    "request_post_async",
    "request_put_async",
    "request_patch_async",
    "request_delete_async",
    "request_head_async",
    "request_options_async",
    "fetch_get",
    "fetch_post",
    "fetch_put",
    "fetch_patch",
    "fetch_delete",
    "fetch_head",
    "fetch_options",
    "fetch_get_headers",
    "fetch_post_headers",
    "fetch_put_headers",
    "fetch_patch_headers",
    "fetch_delete_headers",
    "crypto_random",
    "crypto_sha3_256",
    "crypto_sha3_512",
    "crypto_argon2",
    "crypto_aes256_encrypt",
    "crypto_aes256_decrypt",
    "crypto_chacha20_encrypt",
    "crypto_chacha20_decrypt",
    "crypto_rsa_generate",
    "crypto_rsa_encrypt",
    "crypto_rsa_decrypt",
    "crypto_ecc_generate",
    "crypto_ecc_sign",
    "crypto_ecc_verify",
    "crypto_secure",
    "crypto_wipe",
    "crypto_is_secure",
    "security_list_providers",
    "security_use_provider",
    "security_provider",
    "security_capabilities",
    "device_list",
    "device_open",
    "device_close",
    "device_read",
    "device_write",
    // science (efter `import "science"`)
    "cplx",
    "c_add",
    "c_sub",
    "c_mul",
    "c_div",
    "c_conj",
    "c_abs",
    "c_arg",
    "c_exp",
    "c_sqrt",
    "c_polar",
    "sqrt",
    "pow",
    "fact",
    "gcd",
    "lcm",
    "sin",
    "cos",
    "tan",
    "ln",
    "log10",
    "deg2rad",
    "rad2deg",
    "quadratic",
    "kinetic_energy",
    "potential_energy",
    "force",
    "ohms_v",
    "ohms_p",
    "wavelength",
    "photon_energy",
    "relativity_e",
    "ph",
    "h_plus",
    "molarity",
    "ideal_gas_p",
    "dilution",
    "compound",
    "present_value",
    "break_even",
    "roi",
    "margin",
    "bit_and",
    "bit_or",
    "bit_xor",
    "bit_not",
    "shl",
    "shr",
    "hex",
    "bin",
    "hamming_weight",
    "doc_ask",
    "doc_search",
    "doc_sources",
    "doc_topics",
    // game assets / hot reload (GP2a, GP2b, GP4a)
    "gltf_load_json",
    "image_decode_png",
    "asset_watch",
    "asset_poll",
    "gpu3d_load_wgsl",
    "gpu3d_load_wgsl_from_file",
    "gpu3d_shader_info",
    "host_read_bytes",
];

const HOVER_DOCS: &[(&str, &str)] = &[
    ("let", "Deklarera en variabel. `let x;` ger `undefined` tills den tilldelas."),
    ("class", "Definiera en C#-inspirerad klass med fält och metoder."),
    ("import", "Ladda en inbyggd modul, t.ex. `import \"math\"`, `import \"science\"` eller `import \"docai\"`."),
    ("null", "Explicit tomhet (SQL NULL). Skiljer sig från `undefined`."),
    ("undefined", "Variabeln finns men har inget värde än."),
    ("sql", "Kör SQL mot den inbyggda databasen: `sql(\"SELECT 1\")`."),
    ("kml", "Parsa Kabootar Markup Language till en DOM-trädstruktur."),
    ("println", "Skriv ut ett värde till stdout."),
];

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reports_lexer_error_with_position() {
        let diags = analyze("let x = @;");
        assert_eq!(diags.len(), 1);
        assert!(
            diags[0].message.contains("Unknown character")
                || diags[0].message.contains("Unexpected character")
        );
        assert_eq!(diags[0].line, 1);
    }

    #[test]
    fn reports_parse_error_at_token() {
        let diags = analyze("let = 1;");
        assert_eq!(diags.len(), 1);
        assert!(diags[0].message.contains("identifier") || diags[0].message.contains("Expected"));
        assert!(diags[0].column >= 5);
    }

    #[test]
    fn reports_parse_error_on_unexpected_semicolon() {
        let diags = analyze("1 + ;");
        assert_eq!(diags.len(), 1);
        assert!(diags[0].message.contains("Unexpected"));
        assert!(diags[0].column >= 5);
    }

    #[test]
    fn clean_source_has_no_diagnostics() {
        assert!(analyze("let x = 1;").is_empty());
    }

    #[test]
    fn completions_include_keywords() {
        let items = completions("le");
        assert!(items.iter().any(|i| i.label == "let"));
    }

    #[test]
    fn goto_let_binding() {
        let source = "let answer = 42;\nanswer";
        let target = goto_definition(source, 1, 0).expect("definition");
        assert_eq!(target.line, 1);
        assert_eq!(target.column, 5);
        assert_eq!(target.module, None);
    }

    #[test]
    fn goto_class_name() {
        let source = "class Person { }\nlet p = Person()";
        let target = goto_definition(source, 1, 8).expect("definition");
        assert_eq!(target.line, 1);
        assert!(target.column >= 7);
    }

    #[test]
    fn goto_imported_function() {
        let source = "import \"math\"\nadd(1, 2)";
        let target = goto_definition(source, 1, 0).expect("definition");
        assert_eq!(target.module.as_deref(), Some("math"));
        assert_eq!(target.line, 2);
    }

    #[test]
    fn goto_imported_science_function() {
        let source = "import \"science\"\nc_abs(cplx(1, 1))";
        let target = goto_definition(source, 1, 0).expect("definition");
        assert_eq!(target.module.as_deref(), Some("science"));
    }

    #[test]
    fn completions_include_science() {
        let items = completions("cpl");
        assert!(items.iter().any(|i| i.label == "cplx"));
    }

    #[test]
    fn goto_module_name_in_import() {
        let source = "import \"math\"";
        let target = goto_definition(source, 0, 9).expect("definition");
        assert_eq!(target.line, 1);
        assert_eq!(target.module, None);
    }

    #[test]
    fn goto_type_param_on_generic_fn() {
        let source = "fn id<T>(x) { return x }\nreturn T";
        let target = goto_definition(source, 1, 7).expect("type param def");
        assert_eq!(target.line, 1);
        assert!(target.column >= 6);
    }

    #[test]
    fn goto_type_param_on_generic_class() {
        let source = "class Box<T> { }\nlet x = T";
        let target = goto_definition(source, 1, 8).expect("type param def");
        assert_eq!(target.line, 1);
    }

    #[test]
    fn hover_generic_fn_signature() {
        let source = "fn id<T>(x: T) -> T { return x }";
        let text = hover_at(source, 0, 3).expect("hover fn");
        assert!(text.contains("fn id<T>"));
    }

    #[test]
    fn hover_type_param() {
        let source = "fn id<T>(x) { return x }";
        let text = hover_at(source, 0, 6).expect("hover T");
        assert!(text.contains("type parameter"));
    }

    #[test]
    fn hover_concrete_type_in_type_arg() {
        let source = "fn id<T>(x) { return x }\nid<Number>(1)";
        let text = hover_at(source, 1, 5).expect("hover Number");
        assert!(text.contains("Number"));
    }

    #[test]
    fn completions_after_lt_suggest_types() {
        let source = "fn id<T>(x) { return x }\nid<";
        let items = completions_at(source, 1, 3, "");
        assert!(items.iter().any(|i| i.label == "Number"));
        assert!(items.iter().any(|i| i.label == "String"));
    }

    #[test]
    fn completions_include_generic_class_template() {
        let source = "class Box<T> { }";
        let items = completions_at(source, 0, 0, "Bo");
        assert!(items.iter().any(|i| i.label == "Box"));
    }

    #[test]
    fn hover_generic_class_signature() {
        let source = "class Box<T> { fn get<U>(x) { return x } }";
        let text = hover_at(source, 0, 7).expect("hover Box");
        assert!(text.contains("class Box<T>"));
    }

    #[test]
    fn hover_member_method_with_inferred_receiver() {
        let source = "class Box<T> { fn echo<U>(x) { return x } }\nlet b = Box(42)\nb.echo";
        let text = hover_at(source, 2, 6).expect("hover echo on Box$Number receiver");
        assert!(text.contains("Box$Number"));
        assert!(text.contains("echo"));
    }

    #[test]
    fn hover_member_method_on_non_generic_class() {
        let source = "class Holder { fn echo<T>(x) { return x } }\nlet h = Holder()\nh.echo";
        let text = hover_at(source, 2, 6).expect("hover echo on Holder receiver");
        assert!(text.contains("Holder"));
        assert!(text.contains("echo"));
    }
}
