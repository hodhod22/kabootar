//! Kabootar bytecode format (v2.18+).

use std::fmt::Write as _;

pub const FORMAT_HEADER: &str = "kabootar-bytecode/1";

#[derive(Debug, Clone, PartialEq)]
pub enum Constant {
    Number(i64),
    Float(f64),
    BigInt(String),
    String(String),
    Bool(bool),
    Null,
    Undefined,
    Nan,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Opcode {
    Const(u16),
    LoadLocal(u16),
    StoreLocal(u16),
    LoadGlobal(u16),
    StoreGlobal(u16),
    Pop,
    Add,
    Sub,
    Mul,
    Div,
    Mod,
    Pow,
    Eq,
    Ne,
    Lt,
    Le,
    Gt,
    Ge,
    And,
    Or,
    In,
    BitAnd,
    BitOr,
    BitXor,
    Shl,
    Shr,
    Ushr,
    JumpIfNotNullish(i32),
    Not,
    Neg,
    BitNot,
    Jump(i32),
    JumpIfFalse(i32),
    Call(u8),
    Dup,
    MakeArray(u8),
    MakeObject(u8),
    IndexGet,
    IndexSet,
    GetLength,
    GetMember(u16),
    MemberSet(u16),
    Swap,
    ConcatArray,
    MergeObject,
    CallFromArray,
    MakeOk,
    MakeErr,
    MakeSome,
    MakeNone,
    JumpIfResultErr(i32),
    ArraySliceFrom(u8),
    MakeArrowFn(u16),
    JumpUnlessResultOk(i32),
    UnwrapResultOk,
    JumpUnlessResultErr(i32),
    UnwrapResultErr,
    JumpUnlessOptionSome(i32),
    UnwrapOptionSome,
    JumpUnlessOptionNone(i32),
    JumpUnlessConstEq(u16, i32),
    JumpUnlessArray(i32),
    JumpUnlessObject(i32),
    JumpUnlessObjectEmpty(i32),
    JumpUnlessHasMember(u16, i32),
    IndexPeekFromEnd(u8),
    ArraySliceRest(u8, u8),
    ObjectRest(u8),
    Await,
    Yield,
    YieldStar,
    IteratorStepInPlace,
    AsyncIteratorStepInPlace,
    NewInstance(u16, u8),
    NewInstanceFromArray(u16),
    GetSuperMethod(u16),
    ResultQuestion,
    MatchFail,
    Throw,
    Return,
    Halt,
}

/// `try { … } catch (e) { … }` region inside a generator (for `.throw()` resume).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GeneratorTryRegion {
    pub body_start: usize,
    pub body_end: usize,
    pub catch_start: usize,
    pub err_local: u16,
}

#[derive(Debug, Clone, PartialEq)]
pub struct BytecodeFnDef {
    pub name: String,
    pub params: Vec<String>,
    pub locals: Vec<String>,
    pub globals: Vec<String>,
    pub constants: Vec<Constant>,
    pub code: Vec<Opcode>,
    pub immutable_locals: Vec<bool>,
    pub arrow_functions: Vec<BytecodeFnDef>,
    pub async_fn: bool,
    pub generator_fn: bool,
    pub try_regions: Vec<GeneratorTryRegion>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct BytecodeModule {
    pub constants: Vec<Constant>,
    pub globals: Vec<String>,
    pub main_locals: Vec<String>,
    pub main_immutable_locals: Vec<bool>,
    pub main_try_regions: Vec<GeneratorTryRegion>,
    pub main_code: Vec<Opcode>,
    pub functions: Vec<BytecodeFnDef>,
    pub arrow_functions: Vec<BytecodeFnDef>,
    pub classes: Vec<BytecodeClassDef>,
    pub interfaces: Vec<BytecodeInterfaceDef>,
    pub enums: Vec<BytecodeEnumDef>,
    pub imports: Vec<String>,
    pub pub_imports: Vec<String>,
    pub exports: Vec<String>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct BytecodeEnumVariantDef {
    pub name: String,
    pub fields: Vec<String>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct BytecodeEnumDef {
    pub name: String,
    pub variants: Vec<BytecodeEnumVariantDef>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct BytecodeInterfaceMethod {
    pub name: String,
    pub params: Vec<String>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct BytecodeInterfaceDef {
    pub name: String,
    pub methods: Vec<BytecodeInterfaceMethod>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct BytecodeClassField {
    pub name: String,
    pub type_name: Option<String>,
    pub default_const: Option<u16>,
    pub default_globals: Vec<String>,
    pub default_code: Vec<Opcode>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct BytecodeClassDef {
    pub name: String,
    pub extends: Option<String>,
    pub implements: Vec<String>,
    pub fields: Vec<BytecodeClassField>,
    pub constants: Vec<Constant>,
    pub methods: Vec<BytecodeFnDef>,
}

impl BytecodeModule {
    pub fn uses_bytecode(&self) -> bool {
        !self.main_code.is_empty()
            || !self.functions.is_empty()
            || !self.arrow_functions.is_empty()
            || !self.classes.is_empty()
            || !self.interfaces.is_empty()
            || !self.enums.is_empty()
            || !self.imports.is_empty()
    }
}

pub fn serialize(module: &BytecodeModule) -> String {
    let mut out = String::new();
    writeln!(out, "{FORMAT_HEADER}").unwrap();
    writeln!(out, "constants={}", module.constants.len()).unwrap();
    for (i, c) in module.constants.iter().enumerate() {
        match c {
            Constant::Number(n) => writeln!(out, "const {i} number {n}").unwrap(),
            Constant::BigInt(s) => writeln!(out, "const {i} bigint {}", escape(s)).unwrap(),
            Constant::Float(f) => writeln!(out, "const {i} float {f}").unwrap(),
            Constant::String(s) => writeln!(out, "const {i} string {}", escape(s)).unwrap(),
            Constant::Bool(b) => writeln!(out, "const {i} bool {b}").unwrap(),
            Constant::Null => writeln!(out, "const {i} null").unwrap(),
            Constant::Undefined => writeln!(out, "const {i} undefined").unwrap(),
            Constant::Nan => writeln!(out, "const {i} nan").unwrap(),
        }
    }
    for (i, g) in module.globals.iter().enumerate() {
        writeln!(out, "global {i} {}", escape(g)).unwrap();
    }
    for (i, l) in module.main_locals.iter().enumerate() {
        writeln!(out, "local {i} {}", escape(l)).unwrap();
    }
    for (i, imm) in module.main_immutable_locals.iter().enumerate() {
        if *imm {
            writeln!(out, "immutable_local {i}").unwrap();
        }
    }
    for (ri, region) in module.main_try_regions.iter().enumerate() {
        writeln!(
            out,
            "main_try_region {ri} {} {} {} {}",
            region.body_start, region.body_end, region.catch_start, region.err_local
        )
        .unwrap();
    }
    writeln!(out, "functions={}", module.functions.len()).unwrap();
    for (fi, f) in module.functions.iter().enumerate() {
        writeln!(out, "fn {fi} {}", escape(&f.name)).unwrap();
        writeln!(out, "fn_params {fi} {}", f.params.join(",")).unwrap();
        writeln!(out, "fn_locals {fi} {}", f.locals.join(",")).unwrap();
        for (li, imm) in f.immutable_locals.iter().enumerate() {
            if *imm {
                writeln!(out, "fn_immutable_local {fi} {li}").unwrap();
            }
        }
        if f.async_fn {
            writeln!(out, "fn_async {fi}").unwrap();
        }
        write_fn_try_regions(&mut out, "fn_try_region", &fi.to_string(), &f.try_regions);
        for op in &f.code {
            writeln!(out, "fn_op {fi} {}", encode_op(op)).unwrap();
        }
        writeln!(out, "fn_arrows {fi} {}", f.arrow_functions.len()).unwrap();
        for (ai, arrow) in f.arrow_functions.iter().enumerate() {
            writeln!(out, "fn_arrow {fi} {ai} {}", escape(&arrow.name)).unwrap();
            writeln!(out, "fn_arrow_params {fi} {ai} {}", arrow.params.join(",")).unwrap();
            writeln!(out, "fn_arrow_locals {fi} {ai} {}", arrow.locals.join(",")).unwrap();
            if arrow.async_fn {
                writeln!(out, "fn_arrow_async {fi} {ai}").unwrap();
            }
            for op in &arrow.code {
                writeln!(out, "fn_arrow_op {fi} {ai} {}", encode_op(op)).unwrap();
            }
        }
    }
    writeln!(out, "arrows={}", module.arrow_functions.len()).unwrap();
    for (ai, arrow) in module.arrow_functions.iter().enumerate() {
        writeln!(out, "arrow {ai} {}", escape(&arrow.name)).unwrap();
        writeln!(out, "arrow_params {ai} {}", arrow.params.join(",")).unwrap();
        writeln!(out, "arrow_locals {ai} {}", arrow.locals.join(",")).unwrap();
        if arrow.async_fn {
            writeln!(out, "arrow_async {ai}").unwrap();
        }
        for op in &arrow.code {
            writeln!(out, "arrow_op {ai} {}", encode_op(op)).unwrap();
        }
    }
    writeln!(out, "imports={}", module.imports.len()).unwrap();
    for (i, name) in module.imports.iter().enumerate() {
        writeln!(out, "import {i} {}", escape(name)).unwrap();
    }
    writeln!(out, "pub_imports={}", module.pub_imports.len()).unwrap();
    for (i, name) in module.pub_imports.iter().enumerate() {
        writeln!(out, "pub_import {i} {}", escape(name)).unwrap();
    }
    writeln!(out, "exports={}", module.exports.len()).unwrap();
    for (i, name) in module.exports.iter().enumerate() {
        writeln!(out, "export {i} {}", escape(name)).unwrap();
    }
    writeln!(out, "interfaces={}", module.interfaces.len()).unwrap();
    for (ii, iface) in module.interfaces.iter().enumerate() {
        writeln!(out, "interface {ii} {}", escape(&iface.name)).unwrap();
        for (mi, m) in iface.methods.iter().enumerate() {
            writeln!(out, "iface_method {ii} {mi} {}", escape(&m.name)).unwrap();
            writeln!(out, "iface_method_params {ii} {mi} {}", m.params.join(",")).unwrap();
        }
    }
    writeln!(out, "classes={}", module.classes.len()).unwrap();
    for (ci, class) in module.classes.iter().enumerate() {
        writeln!(out, "class {ci} {}", escape(&class.name)).unwrap();
        if let Some(ext) = &class.extends {
            writeln!(out, "class_extends {ci} {}", escape(ext)).unwrap();
        }
        if !class.implements.is_empty() {
            writeln!(
                out,
                "class_implements {ci} {}",
                class
                    .implements
                    .iter()
                    .map(|s| escape(s))
                    .collect::<Vec<_>>()
                    .join(",")
            )
            .unwrap();
        }
        for (idx, c) in class.constants.iter().enumerate() {
            write_const_line(&mut out, &format!("class_const {ci}"), idx, c);
        }
        for (fi, field) in class.fields.iter().enumerate() {
            writeln!(out, "class_field {ci} {fi} {}", escape(&field.name)).unwrap();
            if let Some(ref tn) = field.type_name {
                writeln!(out, "class_field_type {ci} {fi} {}", escape(tn)).unwrap();
            }
            if let Some(dc) = field.default_const {
                writeln!(out, "class_field_default {ci} {fi} {dc}").unwrap();
            }
            if !field.default_globals.is_empty() {
                writeln!(
                    out,
                    "class_field_default_globals {ci} {fi} {}",
                    field
                        .default_globals
                        .iter()
                        .map(|s| escape(s))
                        .collect::<Vec<_>>()
                        .join(",")
                )
                .unwrap();
            }
            for op in &field.default_code {
                writeln!(
                    out,
                    "class_field_default_op {ci} {fi} {}",
                    encode_op(op)
                )
                .unwrap();
            }
        }
        for (mi, method) in class.methods.iter().enumerate() {
            write_embedded_fn(&mut out, "class_method", &format!("{ci} {mi}"), method);
        }
    }
    writeln!(out, "code").unwrap();
    for op in &module.main_code {
        writeln!(out, "{}", encode_op(op)).unwrap();
    }
    out
}

pub fn deserialize(text: &str) -> Result<BytecodeModule, String> {
    let lines: Vec<&str> = text.lines().map(str::trim).filter(|l| !l.is_empty()).collect();
    let header = lines.first().ok_or("Empty bytecode file")?;
    if *header != FORMAT_HEADER {
        return Err(format!("Unsupported bytecode format: {header}"));
    }

    let mut constants = Vec::new();
    let mut globals = Vec::new();
    let mut main_locals = Vec::new();
    let mut main_immutable_locals = Vec::new();
    let mut main_try_regions = Vec::new();
    let mut main_code = Vec::new();
    let mut functions: Vec<BytecodeFnDef> = Vec::new();
    let mut arrow_functions: Vec<BytecodeFnDef> = Vec::new();
    let mut imports: Vec<String> = Vec::new();
    let mut pub_imports: Vec<String> = Vec::new();
    let mut exports: Vec<String> = Vec::new();
    let mut interfaces: Vec<BytecodeInterfaceDef> = Vec::new();
    let mut classes: Vec<BytecodeClassDef> = Vec::new();
    let mut in_code = false;

    for line in lines.iter().skip(1) {
        if *line == "code" {
            in_code = true;
            continue;
        }
        if in_code {
            if line.starts_with("source=") || line.starts_with("statements=") {
                break;
            }
            main_code.push(decode_op(line)?);
            continue;
        }
        if let Some(rest) = line.strip_prefix("const ") {
            constants.push(parse_const(rest)?);
            continue;
        }
        if let Some(rest) = line.strip_prefix("global ") {
            let (idx, name) = parse_index_name(rest)?;
            ensure_slot(&mut globals, idx, name);
            continue;
        }
        if let Some(rest) = line.strip_prefix("local ") {
            let (idx, name) = parse_index_name(rest)?;
            ensure_slot(&mut main_locals, idx, name);
            continue;
        }
        if let Some(rest) = line.strip_prefix("immutable_local ") {
            let idx: usize = rest
                .parse()
                .map_err(|_| format!("Invalid immutable local index: {line}"))?;
            ensure_bool_slot(&mut main_immutable_locals, idx, true);
            continue;
        }
        if let Some(rest) = line.strip_prefix("main_try_region ") {
            let (ri, region) = parse_try_region_line(rest)?;
            ensure_try_region(&mut main_try_regions, ri, region);
            continue;
        }
        if let Some(rest) = line.strip_prefix("fn_try_region ") {
            let (fi, ri, region) = parse_fn_try_region_line(rest)?;
            ensure_fn(&mut functions, fi, String::new(), Vec::new(), Vec::new());
            ensure_try_region(&mut functions[fi].try_regions, ri, region);
            continue;
        }
        if let Some(rest) = line.strip_prefix("class_method_try_region ") {
            let (ci, mi, ri, region) = parse_class_method_try_region_line(rest)?;
            ensure_class(&mut classes, ci, String::new());
            ensure_class_method(&mut classes[ci].methods, ci, mi, String::new());
            ensure_try_region(&mut classes[ci].methods[mi].try_regions, ri, region);
            continue;
        }
        if let Some(rest) = line.strip_prefix("fn ") {
            let (idx, name) = parse_index_name(rest)?;
            ensure_fn(&mut functions, idx, name, Vec::new(), Vec::new());
            continue;
        }
        if let Some(rest) = line.strip_prefix("fn_params ") {
            let (idx, params) = parse_index_list(rest)?;
            functions[idx].params = params;
            continue;
        }
        if let Some(rest) = line.strip_prefix("fn_locals ") {
            let (idx, locals) = parse_index_list(rest)?;
            functions[idx].locals = locals;
            continue;
        }
        if let Some(rest) = line.strip_prefix("fn_async ") {
            let fi: usize = rest
                .parse()
                .map_err(|_| format!("Invalid fn_async line: {line}"))?;
            ensure_fn(&mut functions, fi, String::new(), Vec::new(), Vec::new());
            functions[fi].async_fn = true;
            continue;
        }
        if let Some(rest) = line.strip_prefix("fn_immutable_local ") {
            let (fi, li) = rest
                .split_once(' ')
                .ok_or_else(|| format!("Invalid fn_immutable_local line: {line}"))?;
            let fi: usize = fi.parse().map_err(|_| format!("Invalid fn index: {line}"))?;
            let li: usize = li.parse().map_err(|_| format!("Invalid local index: {line}"))?;
            ensure_fn(&mut functions, fi, String::new(), Vec::new(), Vec::new());
            ensure_bool_slot(&mut functions[fi].immutable_locals, li, true);
            continue;
        }
        if let Some(rest) = line.strip_prefix("fn_op ") {
            let (idx, op_text) = rest
                .split_once(' ')
                .ok_or_else(|| format!("Invalid fn_op line: {line}"))?;
            let idx: usize = idx.parse().map_err(|_| format!("Invalid fn index: {line}"))?;
            ensure_fn(&mut functions, idx, String::new(), Vec::new(), Vec::new());
            functions[idx].code.push(decode_op(op_text)?);
            continue;
        }
        if let Some(rest) = line.strip_prefix("fn_arrow_op ") {
            let (fi, rest) = rest
                .split_once(' ')
                .ok_or_else(|| format!("Invalid fn_arrow_op line: {line}"))?;
            let (ai, op_text) = rest
                .split_once(' ')
                .ok_or_else(|| format!("Invalid fn_arrow_op line: {line}"))?;
            let fi: usize = fi.parse().map_err(|_| format!("Invalid fn index: {line}"))?;
            let ai: usize = ai.parse().map_err(|_| format!("Invalid arrow index: {line}"))?;
            ensure_fn(&mut functions, fi, String::new(), Vec::new(), Vec::new());
            ensure_fn_arrow(&mut functions[fi].arrow_functions, ai, String::new());
            functions[fi].arrow_functions[ai].code.push(decode_op(op_text)?);
            continue;
        }
        if let Some(rest) = line.strip_prefix("fn_arrow_params ") {
            let (fi, ai, params) = parse_fn_arrow_index_list(rest)?;
            ensure_fn(&mut functions, fi, String::new(), Vec::new(), Vec::new());
            ensure_fn_arrow(&mut functions[fi].arrow_functions, ai, String::new());
            functions[fi].arrow_functions[ai].params = params;
            continue;
        }
        if let Some(rest) = line.strip_prefix("fn_arrow_locals ") {
            let (fi, ai, locals) = parse_fn_arrow_index_list(rest)?;
            ensure_fn(&mut functions, fi, String::new(), Vec::new(), Vec::new());
            ensure_fn_arrow(&mut functions[fi].arrow_functions, ai, String::new());
            functions[fi].arrow_functions[ai].locals = locals;
            continue;
        }
        if let Some(rest) = line.strip_prefix("fn_arrow_async ") {
            let (fi, ai) = rest
                .split_once(' ')
                .ok_or_else(|| format!("Invalid fn_arrow_async line: {line}"))?;
            let fi: usize = fi.parse().map_err(|_| format!("Invalid fn index: {line}"))?;
            let ai: usize = ai.parse().map_err(|_| format!("Invalid arrow index: {line}"))?;
            ensure_fn(&mut functions, fi, String::new(), Vec::new(), Vec::new());
            ensure_fn_arrow(&mut functions[fi].arrow_functions, ai, String::new());
            functions[fi].arrow_functions[ai].async_fn = true;
            continue;
        }
        if let Some(rest) = line.strip_prefix("fn_arrow ") {
            let (fi, ai, name) = parse_fn_arrow_index_name(rest)?;
            ensure_fn(&mut functions, fi, String::new(), Vec::new(), Vec::new());
            ensure_fn_arrow(&mut functions[fi].arrow_functions, ai, name);
            continue;
        }
        if let Some(rest) = line.strip_prefix("arrow_op ") {
            let (ai, op_text) = rest
                .split_once(' ')
                .ok_or_else(|| format!("Invalid arrow_op line: {line}"))?;
            let ai: usize = ai.parse().map_err(|_| format!("Invalid arrow index: {line}"))?;
            ensure_fn_arrow(&mut arrow_functions, ai, String::new());
            arrow_functions[ai].code.push(decode_op(op_text)?);
            continue;
        }
        if let Some(rest) = line.strip_prefix("arrow_params ") {
            let (ai, params) = parse_index_list(rest)?;
            ensure_fn_arrow(&mut arrow_functions, ai, String::new());
            arrow_functions[ai].params = params;
            continue;
        }
        if let Some(rest) = line.strip_prefix("arrow_locals ") {
            let (ai, locals) = parse_index_list(rest)?;
            ensure_fn_arrow(&mut arrow_functions, ai, String::new());
            arrow_functions[ai].locals = locals;
            continue;
        }
        if let Some(rest) = line.strip_prefix("arrow_async ") {
            let ai: usize = rest
                .parse()
                .map_err(|_| format!("Invalid arrow_async line: {line}"))?;
            ensure_fn_arrow(&mut arrow_functions, ai, String::new());
            arrow_functions[ai].async_fn = true;
            continue;
        }
        if let Some(rest) = line.strip_prefix("arrow ") {
            let (ai, name) = parse_index_name(rest)?;
            ensure_fn_arrow(&mut arrow_functions, ai, name);
            continue;
        }
        if let Some(rest) = line.strip_prefix("import ") {
            let (idx, name) = parse_index_name(rest)?;
            ensure_slot(&mut imports, idx, name);
            continue;
        }
        if let Some(rest) = line.strip_prefix("pub_import ") {
            let (idx, name) = parse_index_name(rest)?;
            ensure_slot(&mut pub_imports, idx, name);
            continue;
        }
        if let Some(rest) = line.strip_prefix("export ") {
            let (idx, name) = parse_index_name(rest)?;
            ensure_slot(&mut exports, idx, name);
            continue;
        }
        if let Some(rest) = line.strip_prefix("interface ") {
            let (idx, name) = parse_index_name(rest)?;
            ensure_interface(&mut interfaces, idx, name);
            continue;
        }
        if let Some(rest) = line.strip_prefix("iface_method_params ") {
            let (ii, mi, params) = parse_class_method_index_list(rest)?;
            ensure_interface(&mut interfaces, ii, String::new());
            ensure_iface_method(&mut interfaces[ii].methods, mi, String::new());
            interfaces[ii].methods[mi].params = params;
            continue;
        }
        if let Some(rest) = line.strip_prefix("iface_method ") {
            let (ii, mi, name) = parse_class_method_index_name(rest)?;
            ensure_interface(&mut interfaces, ii, String::new());
            ensure_iface_method(&mut interfaces[ii].methods, mi, name);
            continue;
        }
        if let Some(rest) = line.strip_prefix("class_extends ") {
            let (ci, parent) = parse_index_name(rest)?;
            ensure_class(&mut classes, ci, String::new());
            classes[ci].extends = Some(parent);
            continue;
        }
        if let Some(rest) = line.strip_prefix("class_implements ") {
            let (ci, list) = parse_index_list(rest)?;
            ensure_class(&mut classes, ci, String::new());
            classes[ci].implements = list;
            continue;
        }
        if let Some(rest) = line.strip_prefix("class_const ") {
            let (ci, idx, c) = parse_class_const(rest)?;
            ensure_class(&mut classes, ci, String::new());
            ensure_const_slot(&mut classes[ci].constants, idx, c);
            continue;
        }
        if let Some(rest) = line.strip_prefix("class_field_default_globals ") {
            let (ci, fi, globals) = parse_class_field_default_globals(rest)?;
            ensure_class(&mut classes, ci, String::new());
            ensure_class_field(&mut classes[ci].fields, fi, String::new());
            classes[ci].fields[fi].default_globals = globals;
            continue;
        }
        if let Some(rest) = line.strip_prefix("class_field_default_op ") {
            let (ci, fi, op_text) = parse_class_field_default_op(rest)?;
            ensure_class(&mut classes, ci, String::new());
            ensure_class_field(&mut classes[ci].fields, fi, String::new());
            classes[ci].fields[fi]
                .default_code
                .push(decode_op(op_text)?);
            continue;
        }
        if let Some(rest) = line.strip_prefix("class_field_type ") {
            let (ci, fi, type_name) = parse_class_method_index_name(rest)?;
            ensure_class(&mut classes, ci, String::new());
            ensure_class_field(&mut classes[ci].fields, fi, String::new());
            classes[ci].fields[fi].type_name = Some(type_name);
            continue;
        }
        if let Some(rest) = line.strip_prefix("class_field_default ") {
            let (ci, fi, dc) = parse_class_field_default(rest)?;
            ensure_class(&mut classes, ci, String::new());
            ensure_class_field(&mut classes[ci].fields, fi, String::new());
            classes[ci].fields[fi].default_const = Some(dc);
            continue;
        }
        if let Some(rest) = line.strip_prefix("class_field ") {
            let (ci, fi, name) = parse_class_method_index_name(rest)?;
            ensure_class(&mut classes, ci, String::new());
            ensure_class_field(&mut classes[ci].fields, fi, name);
            continue;
        }
        if let Some(rest) = line.strip_prefix("class_method_const ") {
            let (ci, mi, idx, c) = parse_class_method_const(rest)?;
            ensure_class(&mut classes, ci, String::new());
            ensure_class_method(&mut classes[ci].methods, ci, mi, String::new());
            ensure_const_slot(&mut classes[ci].methods[mi].constants, idx, c);
            continue;
        }
        if let Some(rest) = line.strip_prefix("class_method_globals ") {
            let (ci, mi, globals_list) = parse_class_method_index_list(rest)?;
            ensure_class(&mut classes, ci, String::new());
            ensure_class_method(&mut classes[ci].methods, ci, mi, String::new());
            classes[ci].methods[mi].globals = globals_list;
            continue;
        }
        if let Some(rest) = line.strip_prefix("class_method_params ") {
            let (ci, mi, params) = parse_class_method_index_list(rest)?;
            ensure_class(&mut classes, ci, String::new());
            ensure_class_method(&mut classes[ci].methods, ci, mi, String::new());
            classes[ci].methods[mi].params = params;
            continue;
        }
        if let Some(rest) = line.strip_prefix("class_method_locals ") {
            let (ci, mi, locals) = parse_class_method_index_list(rest)?;
            ensure_class(&mut classes, ci, String::new());
            ensure_class_method(&mut classes[ci].methods, ci, mi, String::new());
            classes[ci].methods[mi].locals = locals;
            continue;
        }
        if let Some(rest) = line.strip_prefix("class_method_async ") {
            let (ci, mi) = parse_class_method_indices(rest)?;
            ensure_class(&mut classes, ci, String::new());
            ensure_class_method(&mut classes[ci].methods, ci, mi, String::new());
            classes[ci].methods[mi].async_fn = true;
            continue;
        }
        if let Some(rest) = line.strip_prefix("class_method_immutable_local ") {
            let (ci, mi, li) = parse_class_method_local_indices(rest)?;
            ensure_class(&mut classes, ci, String::new());
            ensure_class_method(&mut classes[ci].methods, ci, mi, String::new());
            ensure_bool_slot(&mut classes[ci].methods[mi].immutable_locals, li, true);
            continue;
        }
        if let Some(rest) = line.strip_prefix("class_method_op ") {
            let (ci, mi, op_text) = parse_class_method_op(rest)?;
            ensure_class(&mut classes, ci, String::new());
            ensure_class_method(&mut classes[ci].methods, ci, mi, String::new());
            classes[ci].methods[mi].code.push(decode_op(op_text)?);
            continue;
        }
        if let Some(rest) = line.strip_prefix("class_method_arrow_op ") {
            let (ci, mi, ai, op_text) = parse_class_method_arrow_op(rest)?;
            ensure_class(&mut classes, ci, String::new());
            ensure_class_method(&mut classes[ci].methods, ci, mi, String::new());
            ensure_fn_arrow(
                &mut classes[ci].methods[mi].arrow_functions,
                ai,
                String::new(),
            );
            classes[ci].methods[mi].arrow_functions[ai]
                .code
                .push(decode_op(op_text)?);
            continue;
        }
        if let Some(rest) = line.strip_prefix("class_method_arrow_params ") {
            let (ci, mi, ai, params) = parse_class_method_arrow_index_list(rest)?;
            ensure_class(&mut classes, ci, String::new());
            ensure_class_method(&mut classes[ci].methods, ci, mi, String::new());
            ensure_fn_arrow(
                &mut classes[ci].methods[mi].arrow_functions,
                ai,
                String::new(),
            );
            classes[ci].methods[mi].arrow_functions[ai].params = params;
            continue;
        }
        if let Some(rest) = line.strip_prefix("class_method_arrow_locals ") {
            let (ci, mi, ai, locals) = parse_class_method_arrow_index_list(rest)?;
            ensure_class(&mut classes, ci, String::new());
            ensure_class_method(&mut classes[ci].methods, ci, mi, String::new());
            ensure_fn_arrow(
                &mut classes[ci].methods[mi].arrow_functions,
                ai,
                String::new(),
            );
            classes[ci].methods[mi].arrow_functions[ai].locals = locals;
            continue;
        }
        if let Some(rest) = line.strip_prefix("class_method_arrow_async ") {
            let (ci, mi, ai) = parse_class_method_arrow_indices(rest)?;
            ensure_class(&mut classes, ci, String::new());
            ensure_class_method(&mut classes[ci].methods, ci, mi, String::new());
            ensure_fn_arrow(
                &mut classes[ci].methods[mi].arrow_functions,
                ai,
                String::new(),
            );
            classes[ci].methods[mi].arrow_functions[ai].async_fn = true;
            continue;
        }
        if let Some(rest) = line.strip_prefix("class_method_arrow ") {
            let (ci, mi, ai, name) = parse_class_method_arrow_index_name(rest)?;
            ensure_class(&mut classes, ci, String::new());
            ensure_class_method(&mut classes[ci].methods, ci, mi, String::new());
            ensure_fn_arrow(&mut classes[ci].methods[mi].arrow_functions, ai, name);
            continue;
        }
        if let Some(rest) = line.strip_prefix("class_method ") {
            let (ci, mi, name) = parse_class_method_index_name(rest)?;
            ensure_class(&mut classes, ci, String::new());
            ensure_class_method(&mut classes[ci].methods, ci, mi, name);
            continue;
        }
        if let Some(rest) = line.strip_prefix("class ") {
            let (idx, name) = parse_index_name(rest)?;
            ensure_class(&mut classes, idx, name);
            continue;
        }
        if line.starts_with("constants=")
            || line.starts_with("functions=")
            || line.starts_with("arrows=")
            || line.starts_with("fn_arrows ")
            || line.starts_with("imports=")
            || line.starts_with("pub_imports=")
            || line.starts_with("exports=")
            || line.starts_with("interfaces=")
            || line.starts_with("classes=")
            || line.starts_with("class_method_arrows ")
        {
            continue;
        }
        return Err(format!("Unknown bytecode line: {line}"));
    }

    pad_immutable_locals(&main_locals, &mut main_immutable_locals);
    for f in &mut functions {
        pad_immutable_locals(&f.locals, &mut f.immutable_locals);
        for arrow in &mut f.arrow_functions {
            pad_immutable_locals(&arrow.locals, &mut arrow.immutable_locals);
        }
    }
    for arrow in &mut arrow_functions {
        pad_immutable_locals(&arrow.locals, &mut arrow.immutable_locals);
    }
    for class in &mut classes {
        for method in &mut class.methods {
            pad_immutable_locals(&method.locals, &mut method.immutable_locals);
            for arrow in &mut method.arrow_functions {
                pad_immutable_locals(&arrow.locals, &mut arrow.immutable_locals);
            }
        }
    }

    Ok(BytecodeModule {
        constants,
        globals,
        main_locals,
        main_immutable_locals,
        main_try_regions,
        main_code,
        functions,
        arrow_functions,
        classes,
        interfaces,
        enums: Vec::new(),
        imports,
        pub_imports,
        exports,
    })
}

fn write_const_line(out: &mut String, prefix: &str, idx: usize, c: &Constant) {
    match c {
        Constant::Number(n) => writeln!(out, "{prefix} {idx} number {n}").unwrap(),
        Constant::BigInt(s) => writeln!(out, "{prefix} {idx} bigint {}", escape(s)).unwrap(),
        Constant::Float(f) => writeln!(out, "{prefix} {idx} float {f}").unwrap(),
        Constant::String(s) => writeln!(out, "{prefix} {idx} string {}", escape(s)).unwrap(),
        Constant::Bool(b) => writeln!(out, "{prefix} {idx} bool {b}").unwrap(),
        Constant::Null => writeln!(out, "{prefix} {idx} null").unwrap(),
        Constant::Undefined => writeln!(out, "{prefix} {idx} undefined").unwrap(),
        Constant::Nan => writeln!(out, "{prefix} {idx} nan").unwrap(),
    }
}

fn write_fn_try_regions(
    out: &mut String,
    prefix: &str,
    indices: &str,
    regions: &[GeneratorTryRegion],
) {
    for (ri, region) in regions.iter().enumerate() {
        writeln!(
            out,
            "{prefix} {indices} {ri} {} {} {} {}",
            region.body_start, region.body_end, region.catch_start, region.err_local
        )
        .unwrap();
    }
}

fn write_embedded_fn(out: &mut String, prefix: &str, indices: &str, f: &BytecodeFnDef) {
    writeln!(out, "{prefix} {indices} {}", escape(&f.name)).unwrap();
    writeln!(out, "{prefix}_params {indices} {}", f.params.join(",")).unwrap();
    writeln!(out, "{prefix}_locals {indices} {}", f.locals.join(",")).unwrap();
    writeln!(
        out,
        "{prefix}_globals {indices} {}",
        f.globals.join(",")
    )
    .unwrap();
    for (li, imm) in f.immutable_locals.iter().enumerate() {
        if *imm {
            writeln!(out, "{prefix}_immutable_local {indices} {li}").unwrap();
        }
    }
    if f.async_fn {
        writeln!(out, "{prefix}_async {indices}").unwrap();
    }
    write_fn_try_regions(
        out,
        &format!("{prefix}_try_region"),
        indices,
        &f.try_regions,
    );
    for (idx, c) in f.constants.iter().enumerate() {
        write_const_line(out, &format!("{prefix}_const {indices}"), idx, c);
    }
    for op in &f.code {
        writeln!(out, "{prefix}_op {indices} {}", encode_op(op)).unwrap();
    }
    writeln!(out, "{prefix}_arrows {indices} {}", f.arrow_functions.len()).unwrap();
    for (ai, arrow) in f.arrow_functions.iter().enumerate() {
        writeln!(
            out,
            "{prefix}_arrow {indices} {ai} {}",
            escape(&arrow.name)
        )
        .unwrap();
        writeln!(
            out,
            "{prefix}_arrow_params {indices} {ai} {}",
            arrow.params.join(",")
        )
        .unwrap();
        writeln!(
            out,
            "{prefix}_arrow_locals {indices} {ai} {}",
            arrow.locals.join(",")
        )
        .unwrap();
        if arrow.async_fn {
            writeln!(out, "{prefix}_arrow_async {indices} {ai}").unwrap();
        }
        for op in &arrow.code {
            writeln!(
                out,
                "{prefix}_arrow_op {indices} {ai} {}",
                encode_op(op)
            )
            .unwrap();
        }
    }
}

fn ensure_interface(interfaces: &mut Vec<BytecodeInterfaceDef>, idx: usize, name: String) {
    if interfaces.len() <= idx {
        interfaces.resize(
            idx + 1,
            BytecodeInterfaceDef {
                name: String::new(),
                methods: Vec::new(),
            },
        );
    }
    if !name.is_empty() {
        interfaces[idx].name = name;
    }
}

fn ensure_iface_method(methods: &mut Vec<BytecodeInterfaceMethod>, idx: usize, name: String) {
    if methods.len() <= idx {
        methods.resize(
            idx + 1,
            BytecodeInterfaceMethod {
                name: String::new(),
                params: Vec::new(),
            },
        );
    }
    if !name.is_empty() {
        methods[idx].name = name;
    }
}

fn ensure_class(classes: &mut Vec<BytecodeClassDef>, idx: usize, name: String) {
    if classes.len() <= idx {
        classes.resize(
            idx + 1,
            BytecodeClassDef {
                name: String::new(),
                extends: None,
                implements: Vec::new(),
                fields: Vec::new(),
                constants: Vec::new(),
                methods: Vec::new(),
            },
        );
    }
    if !name.is_empty() {
        classes[idx].name = name;
    }
}

fn ensure_class_field(fields: &mut Vec<BytecodeClassField>, idx: usize, name: String) {
    if fields.len() <= idx {
        fields.resize(
            idx + 1,
            BytecodeClassField {
                name: String::new(),
                type_name: None,
                default_const: None,
                default_globals: Vec::new(),
                default_code: Vec::new(),
            },
        );
    }
    if !name.is_empty() {
        fields[idx].name = name;
    }
}

fn ensure_class_method(
    methods: &mut Vec<BytecodeFnDef>,
    _ci: usize,
    mi: usize,
    name: String,
) {
    if methods.len() <= mi {
        methods.resize(
            mi + 1,
            BytecodeFnDef {
                name: String::new(),
                params: Vec::new(),
                locals: Vec::new(),
                globals: Vec::new(),
                constants: Vec::new(),
                code: Vec::new(),
                immutable_locals: Vec::new(),
                arrow_functions: Vec::new(),
                async_fn: false,
                generator_fn: false,
                try_regions: Vec::new(),
            },
        );
    }
    if !name.is_empty() {
        methods[mi].name = name;
    }
}

fn ensure_const_slot(constants: &mut Vec<Constant>, idx: usize, value: Constant) {
    if constants.len() <= idx {
        constants.resize(idx + 1, Constant::Null);
    }
    constants[idx] = value;
}

fn parse_class_method_index_name(rest: &str) -> Result<(usize, usize, String), String> {
    let (ci, rest) = rest
        .split_once(' ')
        .ok_or_else(|| format!("Invalid class/method indexed name: {rest}"))?;
    let (mi, name) = rest
        .split_once(' ')
        .ok_or_else(|| format!("Invalid class/method indexed name: {rest}"))?;
    Ok((
        ci.parse().map_err(|_| format!("Invalid class index: {rest}"))?,
        mi.parse().map_err(|_| format!("Invalid method index: {rest}"))?,
        unescape(name),
    ))
}

fn parse_class_method_index_list(rest: &str) -> Result<(usize, usize, Vec<String>), String> {
    let mut parts = rest.split_whitespace();
    let ci: usize = parts
        .next()
        .ok_or_else(|| format!("Invalid class/method indexed list: {rest}"))?
        .parse()
        .map_err(|_| format!("Invalid class index: {rest}"))?;
    let mi: usize = parts
        .next()
        .ok_or_else(|| format!("Invalid class/method indexed list: {rest}"))?
        .parse()
        .map_err(|_| format!("Invalid method index: {rest}"))?;
    let Some(list) = parts.next() else {
        return Ok((ci, mi, Vec::new()));
    };
    if list.is_empty() {
        return Ok((ci, mi, Vec::new()));
    }
    Ok((ci, mi, list.split(',').map(unescape).collect()))
}

fn parse_class_method_indices(rest: &str) -> Result<(usize, usize), String> {
    let (ci, mi) = rest
        .split_once(' ')
        .ok_or_else(|| format!("Invalid class/method indices: {rest}"))?;
    Ok((
        ci.parse().map_err(|_| format!("Invalid class index: {rest}"))?,
        mi.parse().map_err(|_| format!("Invalid method index: {rest}"))?,
    ))
}

fn parse_class_method_local_indices(rest: &str) -> Result<(usize, usize, usize), String> {
    let (ci, rest) = rest
        .split_once(' ')
        .ok_or_else(|| format!("Invalid class/method/local indices: {rest}"))?;
    let (mi, li) = rest
        .split_once(' ')
        .ok_or_else(|| format!("Invalid class/method/local indices: {rest}"))?;
    Ok((
        ci.parse().map_err(|_| format!("Invalid class index: {rest}"))?,
        mi.parse().map_err(|_| format!("Invalid method index: {rest}"))?,
        li.parse().map_err(|_| format!("Invalid local index: {rest}"))?,
    ))
}

fn parse_class_method_op(rest: &str) -> Result<(usize, usize, &str), String> {
    let (ci, rest) = rest
        .split_once(' ')
        .ok_or_else(|| format!("Invalid class_method_op line: {rest}"))?;
    let (mi, op_text) = rest
        .split_once(' ')
        .ok_or_else(|| format!("Invalid class_method_op line: {rest}"))?;
    Ok((
        ci.parse().map_err(|_| format!("Invalid class index: {rest}"))?,
        mi.parse().map_err(|_| format!("Invalid method index: {rest}"))?,
        op_text,
    ))
}

fn parse_class_method_arrow_indices(rest: &str) -> Result<(usize, usize, usize), String> {
    let (ci, rest) = rest
        .split_once(' ')
        .ok_or_else(|| format!("Invalid class/method/arrow indices: {rest}"))?;
    let (mi, ai) = rest
        .split_once(' ')
        .ok_or_else(|| format!("Invalid class/method/arrow indices: {rest}"))?;
    Ok((
        ci.parse().map_err(|_| format!("Invalid class index: {rest}"))?,
        mi.parse().map_err(|_| format!("Invalid method index: {rest}"))?,
        ai.parse().map_err(|_| format!("Invalid arrow index: {rest}"))?,
    ))
}

fn parse_class_method_arrow_index_list(
    rest: &str,
) -> Result<(usize, usize, usize, Vec<String>), String> {
    let (ci, rest) = rest
        .split_once(' ')
        .ok_or_else(|| format!("Invalid class/method/arrow indexed list: {rest}"))?;
    let (mi, rest) = rest
        .split_once(' ')
        .ok_or_else(|| format!("Invalid class/method/arrow indexed list: {rest}"))?;
    let (ai, list) = rest
        .split_once(' ')
        .ok_or_else(|| format!("Invalid class/method/arrow indexed list: {rest}"))?;
    let ci: usize = ci.parse().map_err(|_| format!("Invalid class index: {rest}"))?;
    let mi: usize = mi.parse().map_err(|_| format!("Invalid method index: {rest}"))?;
    let ai: usize = ai.parse().map_err(|_| format!("Invalid arrow index: {rest}"))?;
    if list.is_empty() {
        return Ok((ci, mi, ai, Vec::new()));
    }
    Ok((ci, mi, ai, list.split(',').map(unescape).collect()))
}

fn parse_class_method_arrow_index_name(
    rest: &str,
) -> Result<(usize, usize, usize, String), String> {
    let (ci, rest) = rest
        .split_once(' ')
        .ok_or_else(|| format!("Invalid class/method/arrow indexed name: {rest}"))?;
    let (mi, rest) = rest
        .split_once(' ')
        .ok_or_else(|| format!("Invalid class/method/arrow indexed name: {rest}"))?;
    let (ai, name) = rest
        .split_once(' ')
        .ok_or_else(|| format!("Invalid class/method/arrow indexed name: {rest}"))?;
    Ok((
        ci.parse().map_err(|_| format!("Invalid class index: {rest}"))?,
        mi.parse().map_err(|_| format!("Invalid method index: {rest}"))?,
        ai.parse().map_err(|_| format!("Invalid arrow index: {rest}"))?,
        unescape(name),
    ))
}

fn parse_class_method_arrow_op(rest: &str) -> Result<(usize, usize, usize, &str), String> {
    let (ci, rest) = rest
        .split_once(' ')
        .ok_or_else(|| format!("Invalid class_method_arrow_op line: {rest}"))?;
    let (mi, rest) = rest
        .split_once(' ')
        .ok_or_else(|| format!("Invalid class_method_arrow_op line: {rest}"))?;
    let (ai, op_text) = rest
        .split_once(' ')
        .ok_or_else(|| format!("Invalid class_method_arrow_op line: {rest}"))?;
    Ok((
        ci.parse().map_err(|_| format!("Invalid class index: {rest}"))?,
        mi.parse().map_err(|_| format!("Invalid method index: {rest}"))?,
        ai.parse().map_err(|_| format!("Invalid arrow index: {rest}"))?,
        op_text,
    ))
}

fn parse_class_const(rest: &str) -> Result<(usize, usize, Constant), String> {
    let (ci, rest) = rest
        .split_once(' ')
        .ok_or_else(|| format!("Invalid class_const line: {rest}"))?;
    let ci: usize = ci.parse().map_err(|_| format!("Invalid class index: {rest}"))?;
    let c = parse_const(rest)?;
    let idx = rest
        .split_whitespace()
        .next()
        .ok_or_else(|| format!("Invalid class_const line: {rest}"))?
        .parse()
        .map_err(|_| format!("Invalid class_const index: {rest}"))?;
    Ok((ci, idx, c))
}

fn parse_class_method_const(rest: &str) -> Result<(usize, usize, usize, Constant), String> {
    let (ci, rest) = rest
        .split_once(' ')
        .ok_or_else(|| format!("Invalid class_method_const line: {rest}"))?;
    let (mi, rest) = rest
        .split_once(' ')
        .ok_or_else(|| format!("Invalid class_method_const line: {rest}"))?;
    let ci: usize = ci.parse().map_err(|_| format!("Invalid class index: {rest}"))?;
    let mi: usize = mi.parse().map_err(|_| format!("Invalid method index: {rest}"))?;
    let idx = rest
        .split_whitespace()
        .next()
        .ok_or_else(|| format!("Invalid class_method_const line: {rest}"))?
        .parse()
        .map_err(|_| format!("Invalid class_method_const index: {rest}"))?;
    let c = parse_const(rest)?;
    Ok((ci, mi, idx, c))
}

fn parse_class_field_default(rest: &str) -> Result<(usize, usize, u16), String> {
    let (ci, rest) = rest
        .split_once(' ')
        .ok_or_else(|| format!("Invalid class_field_default line: {rest}"))?;
    let (fi, dc) = rest
        .split_once(' ')
        .ok_or_else(|| format!("Invalid class_field_default line: {rest}"))?;
    Ok((
        ci.parse().map_err(|_| format!("Invalid class index: {rest}"))?,
        fi.parse().map_err(|_| format!("Invalid field index: {rest}"))?,
        dc.parse().map_err(|_| format!("Invalid default const index: {rest}"))?,
    ))
}

fn parse_class_field_default_globals(rest: &str) -> Result<(usize, usize, Vec<String>), String> {
    let (ci, rest) = rest
        .split_once(' ')
        .ok_or_else(|| format!("Invalid class_field_default_globals line: {rest}"))?;
    let (fi, list) = rest
        .split_once(' ')
        .ok_or_else(|| format!("Invalid class_field_default_globals line: {rest}"))?;
    let globals = if list.is_empty() {
        Vec::new()
    } else {
        list.split(',').map(unescape).collect()
    };
    Ok((
        ci.parse().map_err(|_| format!("Invalid class index: {rest}"))?,
        fi.parse().map_err(|_| format!("Invalid field index: {rest}"))?,
        globals,
    ))
}

fn parse_class_field_default_op(rest: &str) -> Result<(usize, usize, &str), String> {
    let (ci, rest) = rest
        .split_once(' ')
        .ok_or_else(|| format!("Invalid class_field_default_op line: {rest}"))?;
    let (fi, op_text) = rest
        .split_once(' ')
        .ok_or_else(|| format!("Invalid class_field_default_op line: {rest}"))?;
    Ok((
        ci.parse().map_err(|_| format!("Invalid class index: {rest}"))?,
        fi.parse().map_err(|_| format!("Invalid field index: {rest}"))?,
        op_text,
    ))
}

fn escape(s: &str) -> String {
    s.replace('\\', "\\\\").replace(' ', "\\s")
}

fn unescape(s: &str) -> String {
    let mut out = String::new();
    let mut chars = s.chars().peekable();
    while let Some(c) = chars.next() {
        if c == '\\' {
            match chars.next() {
                Some('\\') => out.push('\\'),
                Some(' ') => out.push(' '),
                Some('s') => out.push(' '),
                Some(other) => {
                    out.push('\\');
                    out.push(other);
                }
                None => out.push('\\'),
            }
        } else {
            out.push(c);
        }
    }
    out
}

fn encode_op(op: &Opcode) -> String {
    match op {
        Opcode::Const(i) => format!("const {i}"),
        Opcode::LoadLocal(i) => format!("load_local {i}"),
        Opcode::StoreLocal(i) => format!("store_local {i}"),
        Opcode::LoadGlobal(i) => format!("load_global {i}"),
        Opcode::StoreGlobal(i) => format!("store_global {i}"),
        Opcode::Pop => "pop".into(),
        Opcode::Add => "add".into(),
        Opcode::Sub => "sub".into(),
        Opcode::Mul => "mul".into(),
        Opcode::Div => "div".into(),
        Opcode::Mod => "mod".into(),
        Opcode::Pow => "pow".into(),
        Opcode::Eq => "eq".into(),
        Opcode::Ne => "ne".into(),
        Opcode::Lt => "lt".into(),
        Opcode::Le => "le".into(),
        Opcode::Gt => "gt".into(),
        Opcode::Ge => "ge".into(),
        Opcode::And => "and".into(),
        Opcode::Or => "or".into(),
        Opcode::In => "in".into(),
        Opcode::BitAnd => "bit_and".into(),
        Opcode::BitOr => "bit_or".into(),
        Opcode::BitXor => "bit_xor".into(),
        Opcode::Shl => "shl".into(),
        Opcode::Shr => "shr".into(),
        Opcode::Ushr => "ushr".into(),
        Opcode::JumpIfNotNullish(off) => format!("jump_if_not_nullish {off}"),
        Opcode::Not => "not".into(),
        Opcode::Neg => "neg".into(),
        Opcode::BitNot => "bit_not".into(),
        Opcode::Jump(off) => format!("jump {off}"),
        Opcode::JumpIfFalse(off) => format!("jump_if_false {off}"),
        Opcode::Call(n) => format!("call {n}"),
        Opcode::Dup => "dup".into(),
        Opcode::MakeArray(n) => format!("make_array {n}"),
        Opcode::MakeObject(n) => format!("make_object {n}"),
        Opcode::IndexGet => "index_get".into(),
        Opcode::IndexSet => "index_set".into(),
        Opcode::GetLength => "get_length".into(),
        Opcode::GetMember(i) => format!("get_member {i}"),
        Opcode::MemberSet(i) => format!("member_set {i}"),
        Opcode::Swap => "swap".into(),
        Opcode::ConcatArray => "concat_array".into(),
        Opcode::MergeObject => "merge_object".into(),
        Opcode::CallFromArray => "call_from_array".into(),
        Opcode::MakeOk => "make_ok".into(),
        Opcode::MakeErr => "make_err".into(),
        Opcode::MakeSome => "make_some".into(),
        Opcode::MakeNone => "make_none".into(),
        Opcode::JumpIfResultErr(off) => format!("jump_if_result_err {off}"),
        Opcode::ArraySliceFrom(n) => format!("array_slice_from {n}"),
        Opcode::MakeArrowFn(i) => format!("make_arrow_fn {i}"),
        Opcode::JumpUnlessResultOk(off) => format!("jump_unless_result_ok {off}"),
        Opcode::UnwrapResultOk => "unwrap_result_ok".into(),
        Opcode::JumpUnlessResultErr(off) => format!("jump_unless_result_err {off}"),
        Opcode::UnwrapResultErr => "unwrap_result_err".into(),
        Opcode::JumpUnlessOptionSome(off) => format!("jump_unless_option_some {off}"),
        Opcode::UnwrapOptionSome => "unwrap_option_some".into(),
        Opcode::JumpUnlessOptionNone(off) => format!("jump_unless_option_none {off}"),
        Opcode::JumpUnlessConstEq(i, off) => format!("jump_unless_const_eq {i} {off}"),
        Opcode::JumpUnlessArray(off) => format!("jump_unless_array {off}"),
        Opcode::JumpUnlessObject(off) => format!("jump_unless_object {off}"),
        Opcode::JumpUnlessObjectEmpty(off) => format!("jump_unless_object_empty {off}"),
        Opcode::JumpUnlessHasMember(key, off) => format!("jump_unless_has_member {key} {off}"),
        Opcode::IndexPeekFromEnd(n) => format!("index_peek_from_end {n}"),
        Opcode::ArraySliceRest(start, end) => format!("array_slice_rest {start} {end}"),
        Opcode::ObjectRest(n) => format!("object_rest {n}"),
        Opcode::Await => "await".into(),
        Opcode::Yield => "yield".into(),
        Opcode::YieldStar => "yield_star".into(),
        Opcode::IteratorStepInPlace => "iterator_step_in_place".into(),
        Opcode::AsyncIteratorStepInPlace => "async_iterator_step_in_place".into(),
        Opcode::NewInstance(class, argc) => format!("new_instance {class} {argc}"),
        Opcode::NewInstanceFromArray(class) => format!("new_instance_from_array {class}"),
        Opcode::GetSuperMethod(i) => format!("get_super_method {i}"),
        Opcode::ResultQuestion => "result_question".into(),
        Opcode::MatchFail => "match_fail".into(),
        Opcode::Throw => "throw".into(),
        Opcode::Return => "return".into(),
        Opcode::Halt => "halt".into(),
    }
}

fn decode_op(line: &str) -> Result<Opcode, String> {
    let mut parts = line.split_whitespace();
    let head = parts.next().ok_or_else(|| format!("Empty opcode: {line}"))?;
    Ok(match head {
        "const" => Opcode::Const(parse_u16(parts.next(), line)?),
        "load_local" => Opcode::LoadLocal(parse_u16(parts.next(), line)?),
        "store_local" => Opcode::StoreLocal(parse_u16(parts.next(), line)?),
        "load_global" => Opcode::LoadGlobal(parse_u16(parts.next(), line)?),
        "store_global" => Opcode::StoreGlobal(parse_u16(parts.next(), line)?),
        "pop" => Opcode::Pop,
        "add" => Opcode::Add,
        "sub" => Opcode::Sub,
        "mul" => Opcode::Mul,
        "div" => Opcode::Div,
        "mod" => Opcode::Mod,
        "pow" => Opcode::Pow,
        "eq" => Opcode::Eq,
        "ne" => Opcode::Ne,
        "lt" => Opcode::Lt,
        "le" => Opcode::Le,
        "gt" => Opcode::Gt,
        "ge" => Opcode::Ge,
        "and" => Opcode::And,
        "or" => Opcode::Or,
        "in" => Opcode::In,
        "bit_and" => Opcode::BitAnd,
        "bit_or" => Opcode::BitOr,
        "bit_xor" => Opcode::BitXor,
        "shl" => Opcode::Shl,
        "shr" => Opcode::Shr,
        "ushr" => Opcode::Ushr,
        "jump_if_not_nullish" => Opcode::JumpIfNotNullish(parse_i32(parts.next(), line)?),
        "not" => Opcode::Not,
        "neg" => Opcode::Neg,
        "bit_not" => Opcode::BitNot,
        "jump" => Opcode::Jump(parse_i32(parts.next(), line)?),
        "jump_if_false" => Opcode::JumpIfFalse(parse_i32(parts.next(), line)?),
        "call" => Opcode::Call(parse_u8(parts.next(), line)?),
        "dup" => Opcode::Dup,
        "make_array" => Opcode::MakeArray(parse_u8(parts.next(), line)?),
        "make_object" => Opcode::MakeObject(parse_u8(parts.next(), line)?),
        "index_get" => Opcode::IndexGet,
        "index_set" => Opcode::IndexSet,
        "get_length" => Opcode::GetLength,
        "get_member" => Opcode::GetMember(parse_u16(parts.next(), line)?),
        "member_set" => Opcode::MemberSet(parse_u16(parts.next(), line)?),
        "swap" => Opcode::Swap,
        "concat_array" => Opcode::ConcatArray,
        "merge_object" => Opcode::MergeObject,
        "call_from_array" => Opcode::CallFromArray,
        "make_ok" => Opcode::MakeOk,
        "make_err" => Opcode::MakeErr,
        "make_some" => Opcode::MakeSome,
        "make_none" => Opcode::MakeNone,
        "jump_if_result_err" => Opcode::JumpIfResultErr(parse_i32(parts.next(), line)?),
        "array_slice_from" => Opcode::ArraySliceFrom(parse_u8(parts.next(), line)?),
        "make_arrow_fn" => Opcode::MakeArrowFn(parse_u16(parts.next(), line)?),
        "jump_unless_result_ok" => Opcode::JumpUnlessResultOk(parse_i32(parts.next(), line)?),
        "unwrap_result_ok" => Opcode::UnwrapResultOk,
        "jump_unless_result_err" => Opcode::JumpUnlessResultErr(parse_i32(parts.next(), line)?),
        "unwrap_result_err" => Opcode::UnwrapResultErr,
        "jump_unless_option_some" => Opcode::JumpUnlessOptionSome(parse_i32(parts.next(), line)?),
        "unwrap_option_some" => Opcode::UnwrapOptionSome,
        "jump_unless_option_none" => Opcode::JumpUnlessOptionNone(parse_i32(parts.next(), line)?),
        "jump_unless_const_eq" => {
            let i = parse_u16(parts.next(), line)?;
            Opcode::JumpUnlessConstEq(i, parse_i32(parts.next(), line)?)
        }
        "jump_unless_array" => Opcode::JumpUnlessArray(parse_i32(parts.next(), line)?),
        "jump_unless_object" => Opcode::JumpUnlessObject(parse_i32(parts.next(), line)?),
        "jump_unless_object_empty" => Opcode::JumpUnlessObjectEmpty(parse_i32(parts.next(), line)?),
        "jump_unless_has_member" => {
            let key = parse_u16(parts.next(), line)?;
            Opcode::JumpUnlessHasMember(key, parse_i32(parts.next(), line)?)
        }
        "index_peek_from_end" => Opcode::IndexPeekFromEnd(parse_u8(parts.next(), line)?),
        "array_slice_rest" => {
            let start = parse_u8(parts.next(), line)?;
            Opcode::ArraySliceRest(start, parse_u8(parts.next(), line)?)
        }
        "object_rest" => Opcode::ObjectRest(parse_u8(parts.next(), line)?),
        "await" => Opcode::Await,
        "yield" => Opcode::Yield,
        "yield_star" => Opcode::YieldStar,
        "iterator_step_in_place" => Opcode::IteratorStepInPlace,
        "async_iterator_step_in_place" => Opcode::AsyncIteratorStepInPlace,
        "new_instance" => {
            let class = parse_u16(parts.next(), line)?;
            Opcode::NewInstance(class, parse_u8(parts.next(), line)?)
        }
        "new_instance_from_array" => Opcode::NewInstanceFromArray(parse_u16(parts.next(), line)?),
        "get_super_method" => Opcode::GetSuperMethod(parse_u16(parts.next(), line)?),
        "result_question" => Opcode::ResultQuestion,
        "match_fail" => Opcode::MatchFail,
        "throw" => Opcode::Throw,
        "return" => Opcode::Return,
        "halt" => Opcode::Halt,
        _ => return Err(format!("Unknown opcode: {line}")),
    })
}

fn parse_u16(raw: Option<&str>, line: &str) -> Result<u16, String> {
    raw.ok_or_else(|| format!("Missing operand: {line}"))?
        .parse()
        .map_err(|_| format!("Invalid u16 in: {line}"))
}

fn parse_u8(raw: Option<&str>, line: &str) -> Result<u8, String> {
    raw.ok_or_else(|| format!("Missing operand: {line}"))?
        .parse()
        .map_err(|_| format!("Invalid u8 in: {line}"))
}

fn parse_i32(raw: Option<&str>, line: &str) -> Result<i32, String> {
    raw.ok_or_else(|| format!("Missing operand: {line}"))?
        .parse()
        .map_err(|_| format!("Invalid i32 in: {line}"))
}

fn pad_immutable_locals(locals: &[String], immutables: &mut Vec<bool>) {
    if immutables.len() < locals.len() {
        immutables.resize(locals.len(), false);
    }
}

fn parse_const(rest: &str) -> Result<Constant, String> {
    let mut parts = rest.splitn(3, ' ');
    let _idx = parts.next();
    let kind = parts.next().ok_or_else(|| format!("Invalid const: {rest}"))?;
    match kind {
        "number" => Ok(Constant::Number(
            parts
                .next()
                .ok_or_else(|| format!("Invalid number const: {rest}"))?
                .parse()
                .map_err(|_| format!("Invalid number const: {rest}"))?,
        )),
        "bigint" => {
            let value = rest
                .split_once(" bigint ")
                .map(|(_, v)| v)
                .ok_or_else(|| format!("Invalid bigint const: {rest}"))?;
            Ok(Constant::BigInt(unescape(value)))
        }
        "float" => Ok(Constant::Float(
            parts
                .next()
                .ok_or_else(|| format!("Invalid float const: {rest}"))?
                .parse()
                .map_err(|_| format!("Invalid float const: {rest}"))?,
        )),
        "string" => {
            let value = rest
                .split_once(" string ")
                .map(|(_, v)| v)
                .ok_or_else(|| format!("Invalid string const: {rest}"))?;
            Ok(Constant::String(unescape(value)))
        }
        "bool" => Ok(Constant::Bool(
            parts
                .next()
                .ok_or_else(|| format!("Invalid bool const: {rest}"))?
                .parse()
                .map_err(|_| format!("Invalid bool const: {rest}"))?,
        )),
        "null" => Ok(Constant::Null),
        "undefined" => Ok(Constant::Undefined),
        "nan" => Ok(Constant::Nan),
        _ => Err(format!("Unknown const kind: {rest}")),
    }
}

fn parse_index_name(rest: &str) -> Result<(usize, String), String> {
    let (idx, name) = rest
        .split_once(' ')
        .ok_or_else(|| format!("Invalid indexed name: {rest}"))?;
    Ok((idx.parse().map_err(|_| format!("Invalid index: {rest}"))?, unescape(name)))
}

fn parse_index_list(rest: &str) -> Result<(usize, Vec<String>), String> {
    let mut parts = rest.split_whitespace();
    let idx: usize = parts
        .next()
        .ok_or_else(|| format!("Invalid indexed list: {rest}"))?
        .parse()
        .map_err(|_| format!("Invalid index: {rest}"))?;
    let Some(list) = parts.next() else {
        return Ok((idx, Vec::new()));
    };
    if list.is_empty() {
        return Ok((idx, Vec::new()));
    }
    Ok((idx, list.split(',').map(unescape).collect()))
}

fn ensure_slot(slots: &mut Vec<String>, idx: usize, value: String) {
    if slots.len() <= idx {
        slots.resize(idx + 1, String::new());
    }
    slots[idx] = value;
}

fn ensure_bool_slot(slots: &mut Vec<bool>, idx: usize, value: bool) {
    if slots.len() <= idx {
        slots.resize(idx + 1, false);
    }
    slots[idx] = value;
}

fn ensure_try_region(regions: &mut Vec<GeneratorTryRegion>, idx: usize, region: GeneratorTryRegion) {
    if regions.len() <= idx {
        regions.resize(
            idx + 1,
            GeneratorTryRegion {
                body_start: 0,
                body_end: 0,
                catch_start: 0,
                err_local: 0,
            },
        );
    }
    regions[idx] = region;
}

fn parse_try_region_line(rest: &str) -> Result<(usize, GeneratorTryRegion), String> {
    let mut parts = rest.split_whitespace();
    let ri: usize = parts
        .next()
        .ok_or_else(|| format!("Invalid try_region line: {rest}"))?
        .parse()
        .map_err(|_| format!("Invalid try_region index: {rest}"))?;
    let body_start: usize = parts
        .next()
        .ok_or_else(|| format!("Invalid try_region line: {rest}"))?
        .parse()
        .map_err(|_| format!("Invalid try_region body_start: {rest}"))?;
    let body_end: usize = parts
        .next()
        .ok_or_else(|| format!("Invalid try_region line: {rest}"))?
        .parse()
        .map_err(|_| format!("Invalid try_region body_end: {rest}"))?;
    let catch_start: usize = parts
        .next()
        .ok_or_else(|| format!("Invalid try_region line: {rest}"))?
        .parse()
        .map_err(|_| format!("Invalid try_region catch_start: {rest}"))?;
    let err_local: u16 = parts
        .next()
        .ok_or_else(|| format!("Invalid try_region line: {rest}"))?
        .parse()
        .map_err(|_| format!("Invalid try_region err_local: {rest}"))?;
    Ok((
        ri,
        GeneratorTryRegion {
            body_start,
            body_end,
            catch_start,
            err_local,
        },
    ))
}

fn parse_fn_try_region_line(rest: &str) -> Result<(usize, usize, GeneratorTryRegion), String> {
    let (fi, rest) = rest
        .split_once(' ')
        .ok_or_else(|| format!("Invalid fn_try_region line: {rest}"))?;
    let fi: usize = fi.parse().map_err(|_| format!("Invalid fn index: {rest}"))?;
    let (ri, region) = parse_try_region_line(rest)?;
    Ok((fi, ri, region))
}

fn parse_class_method_try_region_line(
    rest: &str,
) -> Result<(usize, usize, usize, GeneratorTryRegion), String> {
    let (ci, rest) = rest
        .split_once(' ')
        .ok_or_else(|| format!("Invalid class_method_try_region line: {rest}"))?;
    let (mi, rest) = rest
        .split_once(' ')
        .ok_or_else(|| format!("Invalid class_method_try_region line: {rest}"))?;
    let ci: usize = ci.parse().map_err(|_| format!("Invalid class index: {rest}"))?;
    let mi: usize = mi.parse().map_err(|_| format!("Invalid method index: {rest}"))?;
    let (ri, region) = parse_try_region_line(rest)?;
    Ok((ci, mi, ri, region))
}

fn ensure_fn(
    fns: &mut Vec<BytecodeFnDef>,
    idx: usize,
    name: String,
    params: Vec<String>,
    locals: Vec<String>,
) {
    if fns.len() <= idx {
        fns.resize(
            idx + 1,
            BytecodeFnDef {
                name: String::new(),
                params: Vec::new(),
                locals: Vec::new(),
                globals: Vec::new(),
                constants: Vec::new(),
                code: Vec::new(),
                immutable_locals: Vec::new(),
                arrow_functions: Vec::new(),
                async_fn: false,
                generator_fn: false,
                try_regions: Vec::new(),
            },
        );
    }
    if !name.is_empty() {
        fns[idx].name = name;
    }
    if !params.is_empty() {
        fns[idx].params = params;
    }
    if !locals.is_empty() {
        fns[idx].locals = locals;
    }
}

fn ensure_fn_arrow(fns: &mut Vec<BytecodeFnDef>, idx: usize, name: String) {
    ensure_fn(fns, idx, name, Vec::new(), Vec::new());
}

fn parse_fn_arrow_index_list(rest: &str) -> Result<(usize, usize, Vec<String>), String> {
    let (fi, rest) = rest
        .split_once(' ')
        .ok_or_else(|| format!("Invalid fn/arrow indexed list: {rest}"))?;
    let (ai, list) = rest
        .split_once(' ')
        .ok_or_else(|| format!("Invalid fn/arrow indexed list: {rest}"))?;
    let fi: usize = fi.parse().map_err(|_| format!("Invalid fn index: {rest}"))?;
    let ai: usize = ai.parse().map_err(|_| format!("Invalid arrow index: {rest}"))?;
    if list.is_empty() {
        return Ok((fi, ai, Vec::new()));
    }
    Ok((fi, ai, list.split(',').map(unescape).collect()))
}

fn parse_fn_arrow_index_name(rest: &str) -> Result<(usize, usize, String), String> {
    let (fi, rest) = rest
        .split_once(' ')
        .ok_or_else(|| format!("Invalid fn/arrow indexed name: {rest}"))?;
    let (ai, name) = rest
        .split_once(' ')
        .ok_or_else(|| format!("Invalid fn/arrow indexed name: {rest}"))?;
    Ok((
        fi.parse().map_err(|_| format!("Invalid fn index: {rest}"))?,
        ai.parse().map_err(|_| format!("Invalid arrow index: {rest}"))?,
        unescape(name),
    ))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn roundtrip_main_try_regions() {
        let module = BytecodeModule {
            constants: vec![Constant::String("kab".into())],
            globals: Vec::new(),
            main_locals: vec!["msg".into(), "e".into()],
            main_immutable_locals: vec![false, false],
            main_try_regions: vec![GeneratorTryRegion {
                body_start: 2,
                body_end: 8,
                catch_start: 10,
                err_local: 1,
            }],
            main_code: vec![
                Opcode::Const(0),
                Opcode::Throw,
                Opcode::Halt,
            ],
            functions: Vec::new(),
            arrow_functions: Vec::new(),
            classes: Vec::new(),
            interfaces: Vec::new(),
            enums: Vec::new(),
            imports: Vec::new(),
            pub_imports: Vec::new(),
            exports: Vec::new(),
        };
        let restored = deserialize(&serialize(&module)).unwrap();
        assert_eq!(restored.main_try_regions, module.main_try_regions);
    }

    #[test]
    fn roundtrip_simple_module() {
        let module = BytecodeModule {
            constants: vec![Constant::Number(42)],
            globals: vec!["len".to_string()],
            main_locals: vec!["x".to_string()],
            main_immutable_locals: vec![true],
            main_try_regions: Vec::new(),
            main_code: vec![
                Opcode::Const(0),
                Opcode::StoreLocal(0),
                Opcode::LoadLocal(0),
                Opcode::Halt,
            ],
            functions: Vec::new(),
            arrow_functions: Vec::new(),
            classes: Vec::new(),
            interfaces: Vec::new(),
            enums: Vec::new(),
            imports: Vec::new(),
            pub_imports: Vec::new(),
            exports: Vec::new(),
        };
        let text = serialize(&module);
        let back = deserialize(&text).unwrap();
        assert_eq!(back, module);
    }

    #[test]
    fn roundtrip_class_module() {
        use crate::bytecode::compile_source;
        use crate::bytecode::compiler::try_compile;

        let p = compile_source(
            r#"
            interface Greeter { fn greet(); }
            class Animal {
                name: string;
                fn init(n) { self.name = n }
                fn greet() { return "hi " + self.name }
            }
            class Dog extends Animal implements Greeter {
                fn greet() { return super.greet() + "!" }
            }
            let d = Dog("Rex")
            d.greet()
        "#,
        )
        .unwrap();
        let module = try_compile(&p.stmts).expect("bytecode");
        let text = serialize(&module);
        let back = deserialize(&text).unwrap();
        assert_eq!(back, module);
    }
}
