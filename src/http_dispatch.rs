//! Dispatch HTTP requests to registered Kabootar route handlers.

use crate::evaluator::{call_function_value, create_global_env};
use crate::runtime::http::{HttpRequest, HttpResponse};
use crate::runtime::stdlib::deno::{dispatch_serve, SERVE_HANDLER_KEY};
use crate::value::{Environment, Value};

pub fn dispatch(env: &mut Environment, req: &HttpRequest) -> Result<HttpResponse, String> {
    if let Some(handler) = env.get(SERVE_HANDLER_KEY) {
        if !matches!(handler, Value::Null | Value::Undefined) {
            return dispatch_serve(env, req, &handler);
        }
    }

    let router = env
        .get("http_router")
        .ok_or("HTTP router not available")?;
    let Value::HttpRouter(router) = router else {
        return Err("HTTP router not available".into());
    };

    let Some(handler) = router.find_handler(&req.method, &req.path) else {
        return Ok(HttpResponse::not_found());
    };

    let mut call_env = create_global_env();
    *call_env.classes_mut() = env.classes().clone();
    if let Some(Value::HttpRouter(r)) = env.get("http_router") {
        call_env.set("http_router".to_string(), Value::HttpRouter(r));
    }
    call_env.set("req_method".to_string(), Value::String(req.method.clone()));
    call_env.set("req_path".to_string(), Value::String(req.path.clone()));
    call_env.set("req_body".to_string(), Value::String(req.body.clone()));

    let result = call_function_value(&handler, &mut call_env)?;
    coerce_to_response(result)
}

fn coerce_to_response(value: Value) -> Result<HttpResponse, String> {
    match value {
        Value::HttpResponse(res) => Ok(res),
        Value::String(body) => Ok(HttpResponse::new(200, body)),
        Value::Number(n) => Ok(HttpResponse::new(200, n.to_string())),
        Value::Null => Ok(HttpResponse::new(204, "")),
        other => Err(format!(
            "HTTP handler must return http_response(...), got {:?}",
            other
        )),
    }
}
