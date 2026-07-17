//! Class registration and instantiation for bytecode programs.

use super::types::{BytecodeClassDef, BytecodeModule, Constant};
use super::vm::{run_bytecode_fn, run_expr_snippet};
use crate::class::{
    ClassDef, ClassInstance, FieldDef, InterfaceDef, MethodDef, MethodSignature, SharedClassInstance,
};
use crate::evaluator::{create_global_env, validate_class_interfaces};
use crate::value::{Environment, Value};
use std::collections::HashMap;
use std::cell::RefCell;
use std::rc::Rc;

fn const_to_value(c: &Constant) -> Value {
    match c {
        Constant::Number(n) => Value::Number(*n),
        Constant::BigInt(s) => Value::BigInt(
            crate::runtime::stdlib::bigint::parse_decimal(s)
                .unwrap_or_else(|_| num_bigint::BigInt::from(0)),
        ),
        Constant::Float(f) => Value::Float(*f),
        Constant::String(s) => Value::String(s.clone()),
        Constant::Bool(b) => Value::Bool(*b),
        Constant::Null => Value::Null,
        Constant::Undefined => Value::Undefined,
        Constant::Nan => Value::Float(f64::NAN),
    }
}

fn bytecode_class_to_def(class: &BytecodeClassDef) -> ClassDef {
    ClassDef {
        name: class.name.clone(),
        extends: class.extends.clone(),
        implements: class.implements.clone(),
        fields: class
            .fields
            .iter()
            .map(|f| FieldDef {
                name: f.name.clone(),
                type_name: f.type_name.clone(),
                default: None,
                private: crate::class::is_private_name(&f.name),
            })
            .collect(),
        methods: class
            .methods
            .iter()
            .map(|m| MethodDef {
                name: m.name.clone(),
                params: m.params.clone(),
                body: crate::ast::Expr::Literal(crate::ast::Literal::Null),
                bytecode: Some(std::rc::Rc::new(m.clone())),
                private: crate::class::is_private_name(&m.name),
                owner_class: Some(class.name.clone()),
            })
            .collect(),
        is_struct: class.is_struct,
    }
}

pub fn register_module_enums(module: &BytecodeModule, env: &mut Environment) {
    for en in &module.enums {
        let def = crate::class::EnumDef {
            name: en.name.clone(),
            variants: en
                .variants
                .iter()
                .map(|v| crate::class::EnumVariantDef {
                    name: v.name.clone(),
                    fields: v.fields.clone(),
                })
                .collect(),
        };
        crate::class::register_enum_in_env(&def, env);
    }
}

pub fn register_module_interfaces(module: &BytecodeModule, env: &mut Environment) {
    for iface in &module.interfaces {
        env.classes_mut().register_interface(InterfaceDef {
            name: iface.name.clone(),
            methods: iface
                .methods
                .iter()
                .map(|m| MethodSignature {
                    name: m.name.clone(),
                    params: m.params.clone(),
                })
                .collect(),
        });
    }
}

pub fn register_module_classes(module: &BytecodeModule, env: &mut Environment) -> Result<(), String> {
    for class in &module.classes {
        let def = bytecode_class_to_def(class);
        validate_class_interfaces(&def, env)?;
        env.classes_mut().register(def);
    }
    Ok(())
}

fn materialize_class_instance(
    class: &BytecodeClassDef,
    classes: &[BytecodeClassDef],
    env: &mut Environment,
) -> Result<SharedClassInstance, String> {
    let inst = if let Some(parent_name) = &class.extends {
        let parent = classes
            .iter()
            .find(|c| c.name == *parent_name)
            .ok_or_else(|| format!("Unknown base class: {parent_name}"))?;
        materialize_class_instance(parent, classes, env)?
    } else {
        Rc::new(RefCell::new(ClassInstance {
            class_name: class.name.clone(),
            super_class: None,
            interfaces: Vec::new(),
            fields: HashMap::new(),
            methods: HashMap::new(),
            private_fields: HashMap::new(),
            private_methods: HashMap::new(),
            is_struct: class.is_struct,
        }))
    };

    {
        let mut instance = inst
            .try_borrow_mut()
            .map_err(|e| format!("class instance borrow_mut: {e}"))?;
        for field in &class.fields {
            let val = if let Some(idx) = field.default_const {
                class
                    .constants
                    .get(idx as usize)
                    .map(const_to_value)
                    .unwrap_or(Value::Undefined)
            } else if !field.default_code.is_empty() {
                run_expr_snippet(
                    &field.default_code,
                    &class.constants,
                    &field.default_globals,
                    env,
                )?
            } else {
                instance
                    .fields
                    .get(&field.name)
                    .or_else(|| instance.private_fields.get(&field.name))
                    .cloned()
                    .unwrap_or(Value::Undefined)
            };
            if crate::class::is_private_name(&field.name) {
                instance.private_fields.insert(field.name.clone(), val);
            } else {
                instance.fields.insert(field.name.clone(), val);
            }
        }

        for method in &class.methods {
            let method_def = MethodDef {
                name: method.name.clone(),
                params: method.params.clone(),
                body: crate::ast::Expr::Literal(crate::ast::Literal::Null),
                bytecode: Some(std::rc::Rc::new(method.clone())),
                private: crate::class::is_private_name(&method.name),
                owner_class: Some(class.name.clone()),
            };
            if method_def.private {
                instance
                    .private_methods
                    .insert(method.name.clone(), method_def);
            } else {
                instance.methods.insert(method.name.clone(), method_def);
            }
        }

        instance.class_name = class.name.clone();
        instance.super_class = class.extends.clone();
        instance.interfaces = class.implements.clone();
        instance.is_struct = class.is_struct;
    }

    Ok(inst)
}

pub fn instantiate_class(
    class: &BytecodeClassDef,
    classes: &[BytecodeClassDef],
    arg_vals: Vec<Value>,
    env: &mut Environment,
) -> Result<Value, String> {
    let inst = materialize_class_instance(class, classes, env)?;

    if let Some(init) = inst
        .try_borrow()
        .ok()
        .and_then(|i| i.methods.get("init").cloned())
    {
        let Some(bc) = init.bytecode.as_ref() else {
            return Err(format!("Class {} init is missing bytecode", class.name));
        };
        if bc.params.len() != arg_vals.len() {
            return Err(format!(
                "Class {} init expects {} arguments, got {}",
                class.name,
                bc.params.len(),
                arg_vals.len()
            ));
        }
        let mut init_env = create_global_env();
        *init_env.classes_mut() = env.classes().clone();
        let (owner, recv) = {
            let inst_ref = inst
                .try_borrow()
                .map_err(|e| format!("class instance borrow: {e}"))?;
            let owner = crate::class::method_owner_class(&init, &inst_ref);
            let recv = crate::class::receiver_binding(inst_ref.is_struct);
            (owner, recv)
        };
        init_env.set_private_scope(Some(&owner));
        init_env.set(
            recv.to_string(),
            Value::ClassInstance(inst.clone()),
        );
        run_bytecode_fn(bc, arg_vals, &mut init_env)?;
    } else if !arg_vals.is_empty() {
        return Err(format!(
            "Class {} does not accept constructor arguments (define fn init)",
            class.name
        ));
    }

    Ok(Value::ClassInstance(inst))
}
