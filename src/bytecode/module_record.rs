//! SH17: `BytecodeModule` → Kab VM module record.
//!
//! Produces the exact record shape `self_host/deserialize_kbcb.kab`'s
//! `deserializeKbcb` returns, but built directly in Rust — the Kab VM
//! session's S-resident import path (`vTryBindImportSK`) can therefore load a
//! module file without paying the kbcb byte-parse per shard.
//!
//! Faithfully replicates the kbcb record: globals/immutable/capture flags,
//! class field defaults/constants, interfaces and memory_mode are dropped the
//! same way `deserializeKbcb` drops them. `exports` is carried as an extra
//! field (the kbcb record has none) so `vTryBindImportSK` can bind exactly the
//! module's declared exports — pub fns are not necessarily in `globals`.

use std::collections::HashMap;

use crate::bytecode::types::{
    BytecodeClassDef, BytecodeEnumDef, BytecodeFnDef, BytecodeModule, Constant,
    GeneratorTryRegion, Opcode,
};
use crate::value::Value;

/// Mirror of `KBCB_OP_NAMES` in `self_host/deserialize_kbcb.kab` — the tag is
/// the `Opcode` enum declaration order.
const OP_NAMES: [&str; 96] = [
    "const", "load_local", "store_local", "load_global", "store_global", "pop",
    "add", "sub", "mul", "div", "mod", "pow", "eq", "ne", "lt", "le", "gt", "ge",
    "and", "or", "in", "bit_and", "bit_or", "bit_xor", "shl", "shr", "ushr",
    "jump_if_not_nullish", "not", "neg", "bit_not", "jump", "jump_if_false",
    "call", "dup", "make_array", "make_object", "index_get", "index_set",
    "get_length", "array_push", "take_local", "take_global",
    "array_push_local", "array_push_global", "array_pop_local",
    "array_pop_global", "acc_add_local", "acc_add_global", "len_local",
    "len_global", "index_get_local", "index_get_global", "get_member",
    "member_set", "swap", "concat_array", "merge_object", "call_from_array",
    "make_ok", "make_err", "make_some", "make_none", "jump_if_result_err",
    "array_slice_from", "make_arrow_fn", "jump_unless_result_ok",
    "unwrap_result_ok", "jump_unless_result_err", "unwrap_result_err",
    "jump_unless_option_some", "unwrap_option_some", "jump_unless_option_none",
    "jump_unless_enum_variant", "unpack_enum_fields", "jump_unless_const_eq",
    "jump_unless_array", "jump_unless_object", "jump_unless_object_empty",
    "jump_unless_has_member", "index_peek_from_end", "array_slice_rest",
    "object_rest", "await", "yield", "yield_star", "iterator_step_in_place",
    "async_iterator_step_in_place", "new_instance", "new_instance_from_array",
    "get_super_method", "result_question", "match_fail", "throw", "return",
    "halt",
];

fn obj(fields: Vec<(&'static str, Value)>) -> Value {
    let mut m = HashMap::with_capacity(fields.len());
    for (k, v) in fields {
        m.insert(k.to_string(), v);
    }
    Value::from_object(m)
}

fn str_array(items: &[String]) -> Value {
    Value::from_array(items.iter().map(|s| Value::String(s.clone())).collect())
}

/// kbcb const tags: 0 i64, 1 f64, 2 bigint-as-string, 3 string, 4 bool,
/// 5 null, 6 `KAB_UNDEF_CONST` sentinel, 7 nan.
fn const_value(c: &Constant) -> Value {
    match c {
        Constant::Number(n) => Value::Number(*n),
        Constant::Float(f) => Value::Float(*f),
        Constant::BigInt(s) | Constant::String(s) => Value::String(s.clone()),
        Constant::Bool(b) => Value::Bool(*b),
        Constant::Null => Value::Null,
        Constant::Undefined => Value::String("__kab_const_undefined__".to_string()),
        Constant::Nan => Value::Float(f64::NAN),
    }
}

fn const_array(cs: &[Constant]) -> Value {
    Value::from_array(cs.iter().map(const_value).collect())
}

fn try_regions(rs: &[GeneratorTryRegion]) -> Value {
    Value::from_array(
        rs.iter()
            .map(|r| {
                obj(vec![
                    ("bodyStart", Value::Number(r.body_start as i64)),
                    ("bodyEnd", Value::Number(r.body_end as i64)),
                    ("catchStart", Value::Number(r.catch_start as i64)),
                    ("errLocal", Value::Number(r.err_local as i64)),
                ])
            })
            .collect(),
    )
}

/// `{op, arg, tag}` plus `arg2`/`arg3` for the multi-operand kbcb kinds
/// (4/5/6/7). Tag order mirrors `KBCB_OP_NAMES`.
fn op_record(op: &Opcode) -> Value {
    use Opcode::*;
    let (tag, arg, arg2, arg3): (i64, i64, Option<i64>, Option<i64>) = match *op {
        Const(a) => (0, a as i64, None, None),
        LoadLocal(a) => (1, a as i64, None, None),
        StoreLocal(a) => (2, a as i64, None, None),
        LoadGlobal(a) => (3, a as i64, None, None),
        StoreGlobal(a) => (4, a as i64, None, None),
        Pop => (5, 0, None, None),
        Add => (6, 0, None, None),
        Sub => (7, 0, None, None),
        Mul => (8, 0, None, None),
        Div => (9, 0, None, None),
        Mod => (10, 0, None, None),
        Pow => (11, 0, None, None),
        Eq => (12, 0, None, None),
        Ne => (13, 0, None, None),
        Lt => (14, 0, None, None),
        Le => (15, 0, None, None),
        Gt => (16, 0, None, None),
        Ge => (17, 0, None, None),
        And => (18, 0, None, None),
        Or => (19, 0, None, None),
        In => (20, 0, None, None),
        BitAnd => (21, 0, None, None),
        BitOr => (22, 0, None, None),
        BitXor => (23, 0, None, None),
        Shl => (24, 0, None, None),
        Shr => (25, 0, None, None),
        Ushr => (26, 0, None, None),
        JumpIfNotNullish(a) => (27, a as i64, None, None),
        Not => (28, 0, None, None),
        Neg => (29, 0, None, None),
        BitNot => (30, 0, None, None),
        Jump(a) => (31, a as i64, None, None),
        JumpIfFalse(a) => (32, a as i64, None, None),
        Call(a) => (33, a as i64, None, None),
        Dup => (34, 0, None, None),
        MakeArray(a) => (35, a as i64, None, None),
        MakeObject(a) => (36, a as i64, None, None),
        IndexGet => (37, 0, None, None),
        IndexSet => (38, 0, None, None),
        GetLength => (39, 0, None, None),
        ArrayPush => (40, 0, None, None),
        TakeLocal(a) => (41, a as i64, None, None),
        TakeGlobal(a) => (42, a as i64, None, None),
        ArrayPushLocal(a) => (43, a as i64, None, None),
        ArrayPushGlobal(a) => (44, a as i64, None, None),
        ArrayPopLocal(a) => (45, a as i64, None, None),
        ArrayPopGlobal(a) => (46, a as i64, None, None),
        AccAddLocal(a) => (47, a as i64, None, None),
        AccAddGlobal(a) => (48, a as i64, None, None),
        LenLocal(a) => (49, a as i64, None, None),
        LenGlobal(a) => (50, a as i64, None, None),
        IndexGetLocal(a) => (51, a as i64, None, None),
        IndexGetGlobal(a) => (52, a as i64, None, None),
        GetMember(a) => (53, a as i64, None, None),
        MemberSet(a) => (54, a as i64, None, None),
        Swap => (55, 0, None, None),
        ConcatArray => (56, 0, None, None),
        MergeObject => (57, 0, None, None),
        CallFromArray => (58, 0, None, None),
        MakeOk => (59, 0, None, None),
        MakeErr => (60, 0, None, None),
        MakeSome => (61, 0, None, None),
        MakeNone => (62, 0, None, None),
        JumpIfResultErr(a) => (63, a as i64, None, None),
        ArraySliceFrom(a) => (64, a as i64, None, None),
        MakeArrowFn(a) => (65, a as i64, None, None),
        JumpUnlessResultOk(a) => (66, a as i64, None, None),
        UnwrapResultOk => (67, 0, None, None),
        JumpUnlessResultErr(a) => (68, a as i64, None, None),
        UnwrapResultErr => (69, 0, None, None),
        JumpUnlessOptionSome(a) => (70, a as i64, None, None),
        UnwrapOptionSome => (71, 0, None, None),
        JumpUnlessOptionNone(a) => (72, a as i64, None, None),
        JumpUnlessEnumVariant(a, b, c) => {
            (73, a as i64, Some(b as i64), Some(c as i64))
        }
        UnpackEnumFields(a) => (74, a as i64, None, None),
        JumpUnlessConstEq(a, b) => (75, a as i64, Some(b as i64), None),
        JumpUnlessArray(a) => (76, a as i64, None, None),
        JumpUnlessObject(a) => (77, a as i64, None, None),
        JumpUnlessObjectEmpty(a) => (78, a as i64, None, None),
        JumpUnlessHasMember(a, b) => (79, a as i64, Some(b as i64), None),
        IndexPeekFromEnd(a) => (80, a as i64, None, None),
        ArraySliceRest(a, b) => (81, a as i64, Some(b as i64), None),
        ObjectRest(a) => (82, a as i64, None, None),
        Await => (83, 0, None, None),
        Yield => (84, 0, None, None),
        YieldStar => (85, 0, None, None),
        IteratorStepInPlace => (86, 0, None, None),
        AsyncIteratorStepInPlace => (87, 0, None, None),
        NewInstance(a, b) => (88, a as i64, Some(b as i64), None),
        NewInstanceFromArray(a) => (89, a as i64, None, None),
        GetSuperMethod(a) => (90, a as i64, None, None),
        ResultQuestion => (91, 0, None, None),
        MatchFail => (92, 0, None, None),
        Throw => (93, 0, None, None),
        Return => (94, 0, None, None),
        Halt => (95, 0, None, None),
    };
    let mut fields = vec![
        ("op", Value::String(OP_NAMES[tag as usize].to_string())),
        ("arg", Value::Number(arg)),
        ("tag", Value::Number(tag)),
    ];
    if let Some(a2) = arg2 {
        fields.push(("arg2", Value::Number(a2)));
    }
    if let Some(a3) = arg3 {
        fields.push(("arg3", Value::Number(a3)));
    }
    obj(fields)
}

fn op_array(code: &[Opcode]) -> Value {
    Value::from_array(code.iter().map(op_record).collect())
}

/// kbcb fn record — globals/immutable/capture flags are dropped like
/// `kbcbFn` drops them.
fn fn_record(f: &BytecodeFnDef) -> Value {
    obj(vec![
        ("sym", Value::String(f.name.clone())),
        ("params", str_array(&f.params)),
        ("locals", str_array(&f.locals)),
        ("ops", op_array(&f.code)),
        ("constants", const_array(&f.constants)),
        ("tryRegions", try_regions(&f.try_regions)),
        (
            "arrows",
            Value::from_array(f.arrow_functions.iter().map(fn_record).collect()),
        ),
        ("isAsync", Value::Bool(f.async_fn)),
        ("isGen", Value::Bool(f.generator_fn)),
    ])
}

/// kbcb class record — implements/assoc-types/constants/field defaults are
/// dropped like `kbcbClass`/`kbcbField` drop them.
fn class_record(c: &BytecodeClassDef) -> Value {
    let fields = c
        .fields
        .iter()
        .map(|f| {
            obj(vec![
                ("name", Value::String(f.name.clone())),
                (
                    "type",
                    Value::String(f.type_name.clone().unwrap_or_default()),
                ),
            ])
        })
        .collect();
    let mut rec = vec![
        ("sym", Value::String(c.name.clone())),
        (
            "methods",
            Value::from_array(c.methods.iter().map(fn_record).collect()),
        ),
        ("fields", Value::from_array(fields)),
        ("isStruct", Value::Number(c.is_struct as i64)),
    ];
    if let Some(ext) = &c.extends {
        rec.push(("extends", Value::String(ext.clone())));
    }
    obj(rec)
}

fn enum_record(e: &BytecodeEnumDef) -> Value {
    let variants = e
        .variants
        .iter()
        .map(|v| {
            obj(vec![
                ("name", Value::String(v.name.clone())),
                ("fields", str_array(&v.fields)),
            ])
        })
        .collect();
    obj(vec![
        ("sym", Value::String(e.name.clone())),
        ("variants", Value::from_array(variants)),
    ])
}

/// The module record `deserializeKbcb` would return for this module.
pub fn module_record(m: &BytecodeModule) -> Value {
    obj(vec![
        ("constants", const_array(&m.constants)),
        ("globals", str_array(&m.globals)),
        ("locals", str_array(&m.main_locals)),
        ("tryRegions", try_regions(&m.main_try_regions)),
        ("ops", op_array(&m.main_code)),
        (
            "functions",
            Value::from_array(m.functions.iter().map(fn_record).collect()),
        ),
        (
            "arrows",
            Value::from_array(m.arrow_functions.iter().map(fn_record).collect()),
        ),
        (
            "classes",
            Value::from_array(m.classes.iter().map(class_record).collect()),
        ),
        (
            "enums",
            Value::from_array(m.enums.iter().map(enum_record).collect()),
        ),
        ("imports", str_array(&m.imports)),
        ("pub_imports", str_array(&m.pub_imports)),
        ("exports", str_array(&m.exports)),
    ])
}
