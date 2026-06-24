//! C#-inspired class system for Kabootar.
//!
//! Classes have explicit fields, constructors (`fn init`), methods, and inheritance.

pub mod type_check;

use crate::ast::Expr;
use crate::value::Value;
use std::cell::RefCell;
use std::collections::HashMap;
use std::rc::Rc;

pub type SharedClassInstance = Rc<RefCell<ClassInstance>>;

#[derive(Debug, Clone)]
pub struct FieldDef {
    pub name: String,
    pub type_name: Option<String>,
    pub default: Option<Expr>,
    pub private: bool,
}

#[derive(Debug, Clone)]
pub struct EnumVariantDef {
    pub name: String,
    pub fields: Vec<String>,
}

#[derive(Debug, Clone)]
pub struct EnumDef {
    pub name: String,
    pub variants: Vec<EnumVariantDef>,
}

#[derive(Debug, Clone)]
pub struct MethodDef {
    pub name: String,
    pub params: Vec<String>,
    pub body: Expr,
    pub bytecode: Option<std::rc::Rc<crate::bytecode::BytecodeFnDef>>,
    pub private: bool,
    /// Class that declared this method (for private field access).
    pub owner_class: Option<String>,
}

#[derive(Debug, Clone)]
pub struct MethodSignature {
    pub name: String,
    pub params: Vec<String>,
}

#[derive(Debug, Clone)]
pub struct InterfaceDef {
    pub name: String,
    pub methods: Vec<MethodSignature>,
}

/// Static class definition registered at compile/eval time.
#[derive(Debug, Clone)]
pub struct ClassDef {
    pub name: String,
    pub extends: Option<String>,
    pub implements: Vec<String>,
    pub fields: Vec<FieldDef>,
    pub methods: Vec<MethodDef>,
}

#[derive(Debug, Clone)]
pub struct ClassInstance {
    pub class_name: String,
    pub super_class: Option<String>,
    pub interfaces: Vec<String>,
    pub fields: HashMap<String, Value>,
    pub methods: HashMap<String, MethodDef>,
    pub private_fields: HashMap<String, Value>,
    pub private_methods: HashMap<String, MethodDef>,
}

pub fn is_private_name(name: &str) -> bool {
    name.starts_with('#')
}

pub fn class_declares_private_member(
    registry: &ClassRegistry,
    class_name: &str,
    member: &str,
) -> bool {
    let Some(def) = registry.get(class_name) else {
        return false;
    };
    def.fields
        .iter()
        .any(|f| f.private && f.name == member)
        || def
            .methods
            .iter()
            .any(|m| m.private && m.name == member)
}

pub fn can_access_private_member(
    member: &str,
    scope_class: &str,
    registry: &ClassRegistry,
) -> bool {
    class_declares_private_member(registry, scope_class, member)
}

pub fn method_owner_class(method: &MethodDef, inst: &ClassInstance) -> String {
    method
        .owner_class
        .clone()
        .unwrap_or_else(|| inst.class_name.clone())
}

pub fn borrow_class_instance(v: &Value) -> Result<std::cell::Ref<'_, ClassInstance>, String> {
    match v {
        Value::ClassInstance(rc) => rc
            .try_borrow()
            .map_err(|e| format!("class instance borrow: {e}")),
        _ => Err("expected class instance".into()),
    }
}

pub fn borrow_class_instance_mut(
    v: &mut Value,
) -> Result<std::cell::RefMut<'_, ClassInstance>, String> {
    match v {
        Value::ClassInstance(rc) => Rc::get_mut(rc)
            .ok_or_else(|| "class instance is shared".to_string())?
            .try_borrow_mut()
            .map_err(|e| format!("class instance borrow_mut: {e}")),
        _ => Err("expected class instance".into()),
    }
}

pub fn with_class_instance_mut<R>(
    v: &Value,
    f: impl FnOnce(&mut ClassInstance) -> R,
) -> Result<R, String> {
    let Value::ClassInstance(rc) = v else {
        return Err("expected class instance".into());
    };
    let mut inst = rc
        .try_borrow_mut()
        .map_err(|e| format!("class instance borrow_mut: {e}"))?;
    Ok(f(&mut inst))
}

impl ClassDef {
    pub fn instantiate_shell(&self) -> SharedClassInstance {
        let mut fields = HashMap::new();
        for field in &self.fields {
            fields.insert(field.name.clone(), Value::Undefined);
        }
        let methods = self
            .methods
            .iter()
            .map(|m| {
                (
                    m.name.clone(),
                    MethodDef {
                        owner_class: Some(self.name.clone()),
                        private: m.private,
                        ..m.clone()
                    },
                )
            })
            .collect();
        Rc::new(RefCell::new(ClassInstance {
            class_name: self.name.clone(),
            super_class: self.extends.clone(),
            interfaces: self.implements.clone(),
            fields,
            methods,
            private_fields: HashMap::new(),
            private_methods: HashMap::new(),
        }))
    }

    pub fn init_method(&self) -> Option<&MethodDef> {
        self.methods.iter().find(|m| m.name == "init")
    }
}

impl ClassInstance {
    pub fn into_shared(self) -> SharedClassInstance {
        Rc::new(RefCell::new(self))
    }
}

/// Registry of defined classes, interfaces, and enums in the global environment.
#[derive(Debug, Clone, Default)]
pub struct ClassRegistry {
    pub classes: HashMap<String, ClassDef>,
    pub interfaces: HashMap<String, InterfaceDef>,
    pub enums: HashMap<String, EnumDef>,
}

impl ClassRegistry {
    pub fn register(&mut self, def: ClassDef) {
        self.classes.insert(def.name.clone(), def);
    }

    pub fn register_interface(&mut self, def: InterfaceDef) {
        self.interfaces.insert(def.name.clone(), def);
    }

    pub fn register_enum(&mut self, def: EnumDef) {
        self.enums.insert(def.name.clone(), def);
    }

    pub fn get(&self, name: &str) -> Option<&ClassDef> {
        self.classes.get(name)
    }

    pub fn get_interface(&self, name: &str) -> Option<&InterfaceDef> {
        self.interfaces.get(name)
    }

    pub fn get_enum(&self, name: &str) -> Option<&EnumDef> {
        self.enums.get(name)
    }

    pub fn enum_variant(&self, enum_name: &str, variant: &str) -> Option<&EnumVariantDef> {
        self.enums
            .get(enum_name)?
            .variants
            .iter()
            .find(|v| v.name == variant)
    }
}

pub fn register_enum_in_env(def: &EnumDef, env: &mut crate::value::Environment) {
    env.classes_mut().register_enum(def.clone());
    env.set(
        def.name.clone(),
        Value::EnumNamespace(def.name.clone()),
    );
}

pub fn resolve_enum_member(
    type_name: &str,
    variant: &str,
    env: &crate::value::Environment,
) -> Result<Value, String> {
    let Some(vdef) = env.classes().enum_variant(type_name, variant) else {
        return Err(format!("Unknown variant {type_name}.{variant}"));
    };
    if vdef.fields.is_empty() {
        Ok(Value::EnumValue {
            type_name: type_name.to_string(),
            variant: variant.to_string(),
            fields: Vec::new(),
        })
    } else {
        Ok(Value::EnumCtor {
            type_name: type_name.to_string(),
            variant: variant.to_string(),
            arity: vdef.fields.len(),
        })
    }
}

pub fn invoke_enum_ctor(
    type_name: &str,
    variant: &str,
    arity: usize,
    args: Vec<Value>,
) -> Result<Value, String> {
    if args.len() != arity {
        return Err(format!(
            "{type_name}.{variant} expects {arity} arguments, got {}",
            args.len()
        ));
    }
    Ok(Value::EnumValue {
        type_name: type_name.to_string(),
        variant: variant.to_string(),
        fields: args,
    })
}
