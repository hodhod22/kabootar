//! JSON/JSONB operators for Kabootar SQL.

use crate::value::Value;
use std::collections::HashMap;

pub fn json_get_text(val: &Value, path: &str) -> Option<Value> {
    let mut current = val;
    for key in path.split('.') {
        current = match current {
            Value::Object(map) => map.get(key)?,
            _ => return None,
        };
    }
    match current {
        Value::String(s) => Some(Value::String(s.clone())),
        Value::Number(n) => Some(Value::String(n.to_string())),
        Value::Float(f) => Some(Value::String(f.to_string())),
        Value::Bool(b) => Some(Value::String(b.to_string())),
        Value::Null => Some(Value::Null),
        _ => None,
    }
}

pub fn json_contains(left: &Value, right: &Value) -> bool {
    match (left, right) {
        (Value::Object(a), Value::Object(b)) => b.iter().all(|(k, v)| {
            a.get(k)
                .map(|lv| values_json_equal(lv, v))
                .unwrap_or(false)
        }),
        (Value::Array(a), Value::Array(b)) => {
            b.iter().all(|bv| a.iter().any(|av| values_json_equal(av, bv)))
        }
        _ => values_json_equal(left, right),
    }
}

fn values_json_equal(a: &Value, b: &Value) -> bool {
    match (a, b) {
        (Value::Number(x), Value::Number(y)) => x == y,
        (Value::Float(x), Value::Float(y)) => x == y,
        (Value::String(x), Value::String(y)) => x == y,
        (Value::Bool(x), Value::Bool(y)) => x == y,
        (Value::Null, Value::Null) => true,
        (Value::Object(x), Value::Object(y)) => {
            x.len() == y.len()
                && x.iter().all(|(k, v)| {
                    y.get(k)
                        .map(|yv| values_json_equal(v, yv))
                        .unwrap_or(false)
                })
        }
        (Value::Array(x), Value::Array(y)) => {
            x.len() == y.len() && x.iter().zip(y.iter()).all(|(l, r)| values_json_equal(l, r))
        }
        _ => false,
    }
}

pub fn object_from_pairs(pairs: &[(String, Value)]) -> Value {
    let mut map = HashMap::new();
    for (k, v) in pairs {
        map.insert(k.clone(), v.clone());
    }
    Value::from_object(map)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;

    #[test]
    fn contains_does_not_loop() {
        let mut body = HashMap::new();
        body.insert("title".into(), Value::String("hi".into()));
        body.insert("plan".into(), Value::String("pro".into()));
        let body = Value::from_object(body);
        let mut probe = HashMap::new();
        probe.insert("plan".into(), Value::String("pro".into()));
        let probe = Value::from_object(probe);
        assert!(json_contains(&body, &probe));
    }
}
