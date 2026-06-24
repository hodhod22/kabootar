//! Runtime field type checks for class fields.

use crate::class::ClassRegistry;
use crate::value::{format_value, Value};

pub fn check_field_type(
    type_name: &str,
    value: &Value,
    registry: &ClassRegistry,
) -> Result<(), String> {
    let normalized = type_name.trim();
    if normalized.is_empty() {
        return Ok(());
    }
    match normalized {
        "number" => match value {
            Value::Number(_) | Value::Float(_) => Ok(()),
            Value::Undefined => Ok(()),
            other => Err(format!(
                "expected number, got {}",
                format_value(other)
            )),
        },
        "float" => match value {
            Value::Float(_) => Ok(()),
            Value::Undefined => Ok(()),
            other => Err(format!("expected float, got {}", format_value(other))),
        },
        "string" => match value {
            Value::String(_) => Ok(()),
            Value::Undefined => Ok(()),
            other => Err(format!("expected string, got {}", format_value(other))),
        },
        "bool" => match value {
            Value::Bool(_) => Ok(()),
            Value::Undefined => Ok(()),
            other => Err(format!("expected bool, got {}", format_value(other))),
        },
        other => {
            if registry.get_enum(other).is_some() {
                match value {
                    Value::EnumValue { type_name, .. } if type_name == other => Ok(()),
                    Value::Undefined => Ok(()),
                    v => Err(format!(
                        "expected enum {other}, got {}",
                        format_value(v)
                    )),
                }
            } else if registry.get(other).is_some() {
                match value {
                    Value::ClassInstance(inst) => {
                        let name = inst
                            .try_borrow()
                            .map_err(|e| format!("class instance borrow: {e}"))?
                            .class_name
                            .clone();
                        if name == other {
                            Ok(())
                        } else {
                            Err(format!(
                                "expected class {other}, got {}",
                                format_value(value)
                            ))
                        }
                    }
                    Value::Undefined => Ok(()),
                    v => Err(format!(
                        "expected class {other}, got {}",
                        format_value(v)
                    )),
                }
            } else {
                Ok(())
            }
        }
    }
}

pub fn validate_class_field_write(
    inst: &crate::class::ClassInstance,
    field: &str,
    val: &Value,
    registry: &ClassRegistry,
) -> Result<(), String> {
    let field_def = registry.classes.values().find_map(|class_def| {
        class_def
            .fields
            .iter()
            .find(|f| f.name == field)
            .filter(|f| f.private == crate::class::is_private_name(field))
    });
    if let Some(field_def) = field_def {
        if let Some(type_name) = &field_def.type_name {
            if !type_name.is_empty() {
                check_field_type(type_name, val, registry)?;
            }
        }
    }
    let _ = inst;
    Ok(())
}
