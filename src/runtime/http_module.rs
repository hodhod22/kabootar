//! `import "http"` — verb helpers registered as natives (avoids deep Kabootar closures).

use crate::http_dispatch;
use crate::runtime::http::{HttpRequest, HttpResponse, HttpRouter};
use crate::runtime::io_async::schedule_io_promise;
use crate::value::{Environment, IoOp, Value};
use std::collections::HashMap;

fn str_arg(args: &[Value], i: usize, name: &str) -> Result<String, String> {
    match args.get(i) {
        Some(Value::String(s)) => Ok(s.clone()),
        _ => Err(format!("{name} expects string")),
    }
}

fn body_arg(args: &[Value], i: usize) -> String {
    args.get(i)
        .map(|v| match v {
            Value::String(s) => s.clone(),
            other => crate::value::format_value(other),
        })
        .unwrap_or_default()
}

fn router(env: &Environment) -> Result<HttpRouter, String> {
    match env.get("http_router") {
        Some(Value::HttpRouter(r)) => Ok(r),
        _ => Err("HTTP router not available".into()),
    }
}

fn headers_arg(args: &[Value], i: usize, name: &str) -> Result<HashMap<String, String>, String> {
    let Some(v) = args.get(i) else {
        return Err(format!("{name} expects headers object"));
    };
    let Value::Object(map) = v else {
        return Err(format!("{name} expects headers object"));
    };
    let mut headers = HashMap::new();
    for (key, val) in map.iter() {
        headers.insert(
            key.clone(),
            match val {
                Value::String(s) => s.clone(),
                other => crate::value::format_value(other),
            },
        );
    }
    Ok(headers)
}

fn http_ok_native(args: &[Value], _env: &mut Environment) -> Result<Value, String> {
    Ok(Value::HttpResponse(HttpResponse::new(200, body_arg(args, 0))))
}

fn http_created_native(args: &[Value], _env: &mut Environment) -> Result<Value, String> {
    Ok(Value::HttpResponse(HttpResponse::new(201, body_arg(args, 0))))
}

fn http_no_content_native(_args: &[Value], _env: &mut Environment) -> Result<Value, String> {
    Ok(Value::HttpResponse(HttpResponse::new(204, "")))
}

fn http_not_found_native(_args: &[Value], _env: &mut Environment) -> Result<Value, String> {
    Ok(Value::HttpResponse(HttpResponse::not_found()))
}

fn http_method_not_allowed_native(_args: &[Value], _env: &mut Environment) -> Result<Value, String> {
    Ok(Value::HttpResponse(HttpResponse::new(405, "Method Not Allowed")))
}

macro_rules! method_native {
    ($fn_name:ident, $verb:literal) => {
        fn $fn_name(_args: &[Value], _env: &mut Environment) -> Result<Value, String> {
            Ok(Value::String($verb.into()))
        }
    };
}

method_native!(method_get_native, "GET");
method_native!(method_post_native, "POST");
method_native!(method_put_native, "PUT");
method_native!(method_patch_native, "PATCH");
method_native!(method_delete_native, "DELETE");
method_native!(method_head_native, "HEAD");
method_native!(method_options_native, "OPTIONS");

macro_rules! route_native {
    ($fn_name:ident, $verb:literal) => {
        fn $fn_name(args: &[Value], env: &mut Environment) -> Result<Value, String> {
            let path = str_arg(args, 0, stringify!($fn_name))?;
            let handler = args.get(1).cloned().ok_or(concat!(stringify!($fn_name), " expects handler"))?;
            router(env)?.add_route($verb.into(), path, handler)?;
            Ok(Value::Null)
        }
    };
}

route_native!(route_get_native, "GET");
route_native!(route_post_native, "POST");
route_native!(route_put_native, "PUT");
route_native!(route_patch_native, "PATCH");
route_native!(route_delete_native, "DELETE");
route_native!(route_head_native, "HEAD");
route_native!(route_options_native, "OPTIONS");

macro_rules! request_native {
    ($fn_name:ident, $verb:literal) => {
        fn $fn_name(args: &[Value], env: &mut Environment) -> Result<Value, String> {
            let path = str_arg(args, 0, stringify!($fn_name))?;
            let body = body_arg(args, 1);
            let res = http_dispatch::dispatch(env, &HttpRequest::new($verb, path, body))?;
            Ok(Value::HttpResponse(res))
        }
    };
}

request_native!(request_get_native, "GET");
request_native!(request_post_native, "POST");
request_native!(request_put_native, "PUT");
request_native!(request_patch_native, "PATCH");
request_native!(request_delete_native, "DELETE");
request_native!(request_head_native, "HEAD");
request_native!(request_options_native, "OPTIONS");

macro_rules! request_async_native {
    ($fn_name:ident, $verb:literal) => {
        fn $fn_name(args: &[Value], env: &mut Environment) -> Result<Value, String> {
            let path = str_arg(args, 0, stringify!($fn_name))?;
            let body = body_arg(args, 1);
            Ok(schedule_io_promise(
                IoOp::HttpRequest {
                    method: $verb.into(),
                    path,
                    body,
                },
                env,
                None,
            ))
        }
    };
}

request_async_native!(request_get_async_native, "GET");
request_async_native!(request_post_async_native, "POST");
request_async_native!(request_put_async_native, "PUT");
request_async_native!(request_patch_async_native, "PATCH");
request_async_native!(request_delete_async_native, "DELETE");
request_async_native!(request_head_async_native, "HEAD");
request_async_native!(request_options_async_native, "OPTIONS");

macro_rules! fetch_native {
    ($fn_name:ident, $verb:literal) => {
        fn $fn_name(args: &[Value], env: &mut Environment) -> Result<Value, String> {
            let url = str_arg(args, 0, stringify!($fn_name))?;
            let body = body_arg(args, 1);
            Ok(schedule_io_promise(
                IoOp::HttpFetch {
                    method: $verb.into(),
                    url,
                    body,
                    headers: HashMap::new(),
                    timeout_ms: env.http_fetch_timeout_ms(),
                },
                env,
                None,
            ))
        }
    };
}

fetch_native!(fetch_get_native, "GET");
fetch_native!(fetch_post_native, "POST");
fetch_native!(fetch_put_native, "PUT");
fetch_native!(fetch_patch_native, "PATCH");
fetch_native!(fetch_delete_native, "DELETE");
fetch_native!(fetch_head_native, "HEAD");
fetch_native!(fetch_options_native, "OPTIONS");

macro_rules! fetch_headers_native {
    ($fn_name:ident, $verb:literal) => {
        fn $fn_name(args: &[Value], env: &mut Environment) -> Result<Value, String> {
            let url = str_arg(args, 0, stringify!($fn_name))?;
            let body = body_arg(args, 1);
            let headers = headers_arg(args, 2, stringify!($fn_name))?;
            Ok(schedule_io_promise(
                IoOp::HttpFetch {
                    method: $verb.into(),
                    url,
                    body,
                    headers,
                    timeout_ms: env.http_fetch_timeout_ms(),
                },
                env,
                None,
            ))
        }
    };
}

fetch_headers_native!(fetch_get_headers_native, "GET");
fetch_headers_native!(fetch_post_headers_native, "POST");
fetch_headers_native!(fetch_put_headers_native, "PUT");
fetch_headers_native!(fetch_patch_headers_native, "PATCH");
fetch_headers_native!(fetch_delete_headers_native, "DELETE");

pub fn register(env: &mut Environment) {
    let fns: &[(&str, fn(&[Value], &mut Environment) -> Result<Value, String>)] = &[
        ("ok", http_ok_native),
        ("created", http_created_native),
        ("no_content", http_no_content_native),
        ("not_found", http_not_found_native),
        ("method_not_allowed", http_method_not_allowed_native),
        ("method_get", method_get_native),
        ("method_post", method_post_native),
        ("method_put", method_put_native),
        ("method_patch", method_patch_native),
        ("method_delete", method_delete_native),
        ("method_head", method_head_native),
        ("method_options", method_options_native),
        ("route_get", route_get_native),
        ("route_post", route_post_native),
        ("route_put", route_put_native),
        ("route_patch", route_patch_native),
        ("route_delete", route_delete_native),
        ("route_head", route_head_native),
        ("route_options", route_options_native),
        ("request_get", request_get_native),
        ("request_post", request_post_native),
        ("request_put", request_put_native),
        ("request_patch", request_patch_native),
        ("request_delete", request_delete_native),
        ("request_head", request_head_native),
        ("request_options", request_options_native),
        ("request_get_async", request_get_async_native),
        ("request_post_async", request_post_async_native),
        ("request_put_async", request_put_async_native),
        ("request_patch_async", request_patch_async_native),
        ("request_delete_async", request_delete_async_native),
        ("request_head_async", request_head_async_native),
        ("request_options_async", request_options_async_native),
        ("fetch_get", fetch_get_native),
        ("fetch_post", fetch_post_native),
        ("fetch_put", fetch_put_native),
        ("fetch_patch", fetch_patch_native),
        ("fetch_delete", fetch_delete_native),
        ("fetch_head", fetch_head_native),
        ("fetch_options", fetch_options_native),
        ("fetch_get_headers", fetch_get_headers_native),
        ("fetch_post_headers", fetch_post_headers_native),
        ("fetch_put_headers", fetch_put_headers_native),
        ("fetch_patch_headers", fetch_patch_headers_native),
        ("fetch_delete_headers", fetch_delete_headers_native),
    ];
    for (name, func) in fns {
        env.set(name.to_string(), Value::NativeFunction(*func));
    }
}
