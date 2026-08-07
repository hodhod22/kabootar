//! Training DX — progress logging for REPL/notebook (SC2j).

use super::helpers::{float_out, int_out, num, num_at};
use crate::value::{Environment, Value};
use std::collections::HashMap;

/// ml_train_log(epoch, loss, metrics?, opts?) → train_progress object for rich_display.
fn ml_train_log(args: &[Value], _env: &mut Environment) -> Result<Value, String> {
    let epoch = num_at(args, 0, "ml_train_log")? as i64;
    let loss = num_at(args, 1, "ml_train_log")?;
    let metrics = args.get(2).cloned().unwrap_or(Value::Null);
    let opts = args.get(3).cloned().unwrap_or(Value::Null);

    let verbose = match &opts {
        Value::Object(m) => match m.get("verbose") {
            Some(Value::Bool(b)) => *b,
            _ => true,
        },
        _ => true,
    };
    if verbose {
        let mut line = format!("epoch {} loss={}", epoch, loss);
        if let Value::Object(m) = &metrics {
            for (k, v) in m.iter() {
                if let Ok(f) = num(v) {
                    line.push_str(&format!(" {}={}", k, f));
                }
            }
        }
        eprintln!("{}", line);
    }

    let mut out = HashMap::new();
    out.insert("kind".into(), Value::String("train_progress".into()));
    out.insert("epoch".into(), int_out(epoch));
    out.insert("loss".into(), float_out(loss));
    if !matches!(metrics, Value::Null | Value::Undefined) {
        out.insert("metrics".into(), metrics);
    }
    if !matches!(opts, Value::Null | Value::Undefined) {
        out.insert("opts".into(), opts);
    }

    Ok(Value::from_object(out))
}

pub fn register(bind: &mut dyn FnMut(&[&str], fn(&[Value], &mut Environment) -> Result<Value, String>)) {
    bind(&["science_ml_train_log", "ml_train_log"], ml_train_log);
}
