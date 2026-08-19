//! P10f v1: `KBCB` envelope around UTF-8 `.kbc` text.
//! SH4 v2: packed records (opcodes as tag+operands, no per-line string split).

use super::types::{
    deserialize, serialize, BytecodeClassDef, BytecodeClassField, BytecodeEnumDef,
    BytecodeEnumVariantDef, BytecodeFnDef, BytecodeInterfaceDef, BytecodeInterfaceMethod,
    BytecodeModule, Constant, GeneratorTryRegion, Opcode,
};
use crate::lang_preprocess::MemoryMode;

pub const KBCB_MAGIC: &[u8; 4] = b"KBCB";
pub const KBCB_VERSION: u8 = 2;
pub const KBCB_VERSION_TEXT: u8 = 1;

pub fn serialize_kbcb(module: &BytecodeModule) -> Vec<u8> {
    serialize_kbcb_v2(module)
}

pub fn serialize_kbcb_v1(module: &BytecodeModule) -> Vec<u8> {
    wrap(KBCB_VERSION_TEXT, serialize(module).into_bytes())
}

pub fn serialize_kbcb_v2(module: &BytecodeModule) -> Vec<u8> {
    wrap(KBCB_VERSION, encode_module(module))
}

fn wrap(version: u8, payload: Vec<u8>) -> Vec<u8> {
    let mut out = Vec::with_capacity(9 + payload.len());
    out.extend_from_slice(KBCB_MAGIC);
    out.push(version);
    out.extend_from_slice(&(payload.len() as u32).to_le_bytes());
    out.extend_from_slice(&payload);
    out
}

pub fn looks_like_kbcb(bytes: &[u8]) -> bool {
    bytes.len() >= 9 && bytes.starts_with(KBCB_MAGIC)
}

pub fn deserialize_kbcb(bytes: &[u8]) -> Result<BytecodeModule, String> {
    if !looks_like_kbcb(bytes) {
        return Err("not a .kbcb file".into());
    }
    let n = u32::from_le_bytes(bytes[5..9].try_into().unwrap()) as usize;
    let start = 9usize;
    let end = start
        .checked_add(n)
        .ok_or_else(|| "kbcb length overflow".to_string())?;
    if end > bytes.len() {
        return Err("kbcb truncated".into());
    }
    let payload = &bytes[start..end];
    match bytes[4] {
        KBCB_VERSION_TEXT => {
            let text = std::str::from_utf8(payload).map_err(|e| e.to_string())?;
            deserialize(text)
        }
        KBCB_VERSION => decode_module(payload),
        v => Err(format!("unsupported kbcb version {v}")),
    }
}

pub fn deserialize_kbcb_v2(bytes: &[u8]) -> Result<BytecodeModule, String> {
    deserialize_kbcb(bytes)
}

struct W(Vec<u8>);

impl W {
    fn u8(&mut self, v: u8) {
        self.0.push(v);
    }
    fn u16(&mut self, v: u16) {
        self.0.extend_from_slice(&v.to_le_bytes());
    }
    fn u32(&mut self, v: u32) {
        self.0.extend_from_slice(&v.to_le_bytes());
    }
    fn i32(&mut self, v: i32) {
        self.0.extend_from_slice(&v.to_le_bytes());
    }
    fn i64(&mut self, v: i64) {
        self.0.extend_from_slice(&v.to_le_bytes());
    }
    fn f64(&mut self, v: f64) {
        self.0.extend_from_slice(&v.to_le_bytes());
    }
    fn str(&mut self, s: &str) {
        let b = s.as_bytes();
        self.u32(b.len() as u32);
        self.0.extend_from_slice(b);
    }
    fn bool(&mut self, v: bool) {
        self.u8(if v { 1 } else { 0 });
    }
}

struct R<'a> {
    s: &'a [u8],
    i: usize,
}

impl<'a> R<'a> {
    fn rest(&self) -> usize {
        self.s.len().saturating_sub(self.i)
    }
    fn take(&mut self, n: usize) -> Result<&'a [u8], String> {
        if self.rest() < n {
            return Err("kbcb v2 truncated".into());
        }
        let b = &self.s[self.i..self.i + n];
        self.i += n;
        Ok(b)
    }
    fn u8(&mut self) -> Result<u8, String> {
        Ok(self.take(1)?[0])
    }
    fn u16(&mut self) -> Result<u16, String> {
        Ok(u16::from_le_bytes(self.take(2)?.try_into().unwrap()))
    }
    fn u32(&mut self) -> Result<u32, String> {
        Ok(u32::from_le_bytes(self.take(4)?.try_into().unwrap()))
    }
    fn i32(&mut self) -> Result<i32, String> {
        Ok(i32::from_le_bytes(self.take(4)?.try_into().unwrap()))
    }
    fn i64(&mut self) -> Result<i64, String> {
        Ok(i64::from_le_bytes(self.take(8)?.try_into().unwrap()))
    }
    fn f64(&mut self) -> Result<f64, String> {
        Ok(f64::from_le_bytes(self.take(8)?.try_into().unwrap()))
    }
    fn str(&mut self) -> Result<String, String> {
        let n = self.u32()? as usize;
        let b = self.take(n)?;
        std::str::from_utf8(b)
            .map(|s| s.to_string())
            .map_err(|e| e.to_string())
    }
    fn bool(&mut self) -> Result<bool, String> {
        Ok(self.u8()? != 0)
    }
}

fn encode_module(m: &BytecodeModule) -> Vec<u8> {
    let mut w = W(Vec::new());
    w.u8(match m.memory_mode {
        MemoryMode::Gc => 0,
        MemoryMode::Manual => 1,
    });
    encode_consts(&mut w, &m.constants);
    encode_strs(&mut w, &m.globals);
    encode_strs(&mut w, &m.main_locals);
    encode_bools(&mut w, &m.main_immutable_locals);
    encode_tries(&mut w, &m.main_try_regions);
    encode_ops(&mut w, &m.main_code);
    encode_fns(&mut w, &m.functions);
    encode_fns(&mut w, &m.arrow_functions);
    encode_classes(&mut w, &m.classes);
    encode_ifaces(&mut w, &m.interfaces);
    encode_enums(&mut w, &m.enums);
    encode_strs(&mut w, &m.imports);
    encode_strs(&mut w, &m.pub_imports);
    encode_strs(&mut w, &m.exports);
    w.0
}

fn decode_module(bytes: &[u8]) -> Result<BytecodeModule, String> {
    let mut r = R { s: bytes, i: 0 };
    let memory_mode = match r.u8()? {
        1 => MemoryMode::Manual,
        _ => MemoryMode::Gc,
    };
    Ok(BytecodeModule {
        constants: decode_consts(&mut r)?,
        globals: decode_strs(&mut r)?,
        main_locals: decode_strs(&mut r)?,
        main_immutable_locals: decode_bools(&mut r)?,
        main_try_regions: decode_tries(&mut r)?,
        main_code: decode_ops(&mut r)?,
        functions: decode_fns(&mut r)?,
        arrow_functions: decode_fns(&mut r)?,
        classes: decode_classes(&mut r)?,
        interfaces: decode_ifaces(&mut r)?,
        enums: decode_enums(&mut r)?,
        imports: decode_strs(&mut r)?,
        pub_imports: decode_strs(&mut r)?,
        exports: decode_strs(&mut r)?,
        memory_mode,
    })
}

fn encode_strs(w: &mut W, xs: &[String]) {
    w.u32(xs.len() as u32);
    for s in xs {
        w.str(s);
    }
}

fn decode_strs(r: &mut R) -> Result<Vec<String>, String> {
    let n = r.u32()? as usize;
    let mut out = Vec::with_capacity(n);
    for _ in 0..n {
        out.push(r.str()?);
    }
    Ok(out)
}

fn encode_bools(w: &mut W, xs: &[bool]) {
    w.u32(xs.len() as u32);
    for b in xs {
        w.bool(*b);
    }
}

fn decode_bools(r: &mut R) -> Result<Vec<bool>, String> {
    let n = r.u32()? as usize;
    let mut out = Vec::with_capacity(n);
    for _ in 0..n {
        out.push(r.bool()?);
    }
    Ok(out)
}

fn encode_consts(w: &mut W, xs: &[Constant]) {
    w.u32(xs.len() as u32);
    for c in xs {
        match c {
            Constant::Number(n) => {
                w.u8(0);
                w.i64(*n);
            }
            Constant::Float(f) => {
                w.u8(1);
                w.f64(*f);
            }
            Constant::BigInt(s) => {
                w.u8(2);
                w.str(s);
            }
            Constant::String(s) => {
                w.u8(3);
                w.str(s);
            }
            Constant::Bool(b) => {
                w.u8(4);
                w.bool(*b);
            }
            Constant::Null => w.u8(5),
            Constant::Undefined => w.u8(6),
            Constant::Nan => w.u8(7),
        }
    }
}

fn decode_consts(r: &mut R) -> Result<Vec<Constant>, String> {
    let n = r.u32()? as usize;
    let mut out = Vec::with_capacity(n);
    for _ in 0..n {
        out.push(match r.u8()? {
            0 => Constant::Number(r.i64()?),
            1 => Constant::Float(r.f64()?),
            2 => Constant::BigInt(r.str()?),
            3 => Constant::String(r.str()?),
            4 => Constant::Bool(r.bool()?),
            5 => Constant::Null,
            6 => Constant::Undefined,
            7 => Constant::Nan,
            t => return Err(format!("kbcb v2 bad const tag {t}")),
        });
    }
    Ok(out)
}

fn encode_tries(w: &mut W, xs: &[GeneratorTryRegion]) {
    w.u32(xs.len() as u32);
    for t in xs {
        w.u32(t.body_start as u32);
        w.u32(t.body_end as u32);
        w.u32(t.catch_start as u32);
        w.u16(t.err_local);
    }
}

fn decode_tries(r: &mut R) -> Result<Vec<GeneratorTryRegion>, String> {
    let n = r.u32()? as usize;
    let mut out = Vec::with_capacity(n);
    for _ in 0..n {
        out.push(GeneratorTryRegion {
            body_start: r.u32()? as usize,
            body_end: r.u32()? as usize,
            catch_start: r.u32()? as usize,
            err_local: r.u16()?,
        });
    }
    Ok(out)
}

fn encode_fns(w: &mut W, xs: &[BytecodeFnDef]) {
    w.u32(xs.len() as u32);
    for f in xs {
        encode_fn(w, f);
    }
}

fn decode_fns(r: &mut R) -> Result<Vec<BytecodeFnDef>, String> {
    let n = r.u32()? as usize;
    let mut out = Vec::with_capacity(n);
    for _ in 0..n {
        out.push(decode_fn(r)?);
    }
    Ok(out)
}

fn encode_fn(w: &mut W, f: &BytecodeFnDef) {
    w.str(&f.name);
    encode_strs(w, &f.params);
    encode_strs(w, &f.locals);
    encode_strs(w, &f.globals);
    encode_consts(w, &f.constants);
    encode_ops(w, &f.code);
    encode_bools(w, &f.immutable_locals);
    encode_bools(w, &f.local_captures);
    encode_fns(w, &f.arrow_functions);
    w.bool(f.async_fn);
    w.bool(f.generator_fn);
    encode_tries(w, &f.try_regions);
}

fn decode_fn(r: &mut R) -> Result<BytecodeFnDef, String> {
    Ok(BytecodeFnDef {
        name: r.str()?,
        params: decode_strs(r)?,
        locals: decode_strs(r)?,
        globals: decode_strs(r)?,
        constants: decode_consts(r)?,
        code: decode_ops(r)?,
        immutable_locals: decode_bools(r)?,
        local_captures: decode_bools(r)?,
        arrow_functions: decode_fns(r)?,
        async_fn: r.bool()?,
        generator_fn: r.bool()?,
        try_regions: decode_tries(r)?,
    })
}

fn encode_classes(w: &mut W, xs: &[BytecodeClassDef]) {
    w.u32(xs.len() as u32);
    for c in xs {
        w.str(&c.name);
        match &c.extends {
            Some(s) => {
                w.u8(1);
                w.str(s);
            }
            None => w.u8(0),
        }
        encode_strs(w, &c.implements);
        w.u32(c.associated_types.len() as u32);
        for (a, b) in &c.associated_types {
            w.str(a);
            w.str(b);
        }
        w.u32(c.fields.len() as u32);
        for f in &c.fields {
            encode_field(w, f);
        }
        encode_consts(w, &c.constants);
        encode_fns(w, &c.methods);
        w.bool(c.is_struct);
    }
}

fn encode_field(w: &mut W, f: &BytecodeClassField) {
    w.str(&f.name);
    match &f.type_name {
        Some(s) => {
            w.u8(1);
            w.str(s);
        }
        None => w.u8(0),
    }
    match f.default_const {
        Some(i) => {
            w.u8(1);
            w.u16(i);
        }
        None => w.u8(0),
    }
    encode_strs(w, &f.default_globals);
    encode_ops(w, &f.default_code);
}

fn decode_classes(r: &mut R) -> Result<Vec<BytecodeClassDef>, String> {
    let n = r.u32()? as usize;
    let mut out = Vec::with_capacity(n);
    for _ in 0..n {
        let name = r.str()?;
        let extends = if r.u8()? == 1 {
            Some(r.str()?)
        } else {
            None
        };
        let implements = decode_strs(r)?;
        let an = r.u32()? as usize;
        let mut associated_types = Vec::with_capacity(an);
        for _ in 0..an {
            associated_types.push((r.str()?, r.str()?));
        }
        let fn_ = r.u32()? as usize;
        let mut fields = Vec::with_capacity(fn_);
        for _ in 0..fn_ {
            fields.push(decode_field(r)?);
        }
        out.push(BytecodeClassDef {
            name,
            extends,
            implements,
            associated_types,
            fields,
            constants: decode_consts(r)?,
            methods: decode_fns(r)?,
            is_struct: r.bool()?,
        });
    }
    Ok(out)
}

fn decode_field(r: &mut R) -> Result<BytecodeClassField, String> {
    Ok(BytecodeClassField {
        name: r.str()?,
        type_name: if r.u8()? == 1 {
            Some(r.str()?)
        } else {
            None
        },
        default_const: if r.u8()? == 1 {
            Some(r.u16()?)
        } else {
            None
        },
        default_globals: decode_strs(r)?,
        default_code: decode_ops(r)?,
    })
}

fn encode_ifaces(w: &mut W, xs: &[BytecodeInterfaceDef]) {
    w.u32(xs.len() as u32);
    for i in xs {
        w.str(&i.name);
        encode_strs(w, &i.type_params);
        encode_strs(w, &i.associated_types);
        w.u32(i.methods.len() as u32);
        for m in &i.methods {
            w.str(&m.name);
            encode_strs(w, &m.params);
            match &m.default_fn {
                Some(f) => {
                    w.u8(1);
                    encode_fn(w, f);
                }
                None => w.u8(0),
            }
        }
    }
}

fn decode_ifaces(r: &mut R) -> Result<Vec<BytecodeInterfaceDef>, String> {
    let n = r.u32()? as usize;
    let mut out = Vec::with_capacity(n);
    for _ in 0..n {
        let name = r.str()?;
        let type_params = decode_strs(r)?;
        let associated_types = decode_strs(r)?;
        let mn = r.u32()? as usize;
        let mut methods = Vec::with_capacity(mn);
        for _ in 0..mn {
            methods.push(BytecodeInterfaceMethod {
                name: r.str()?,
                params: decode_strs(r)?,
                default_fn: if r.u8()? == 1 {
                    Some(decode_fn(r)?)
                } else {
                    None
                },
            });
        }
        out.push(BytecodeInterfaceDef {
            name,
            type_params,
            associated_types,
            methods,
        });
    }
    Ok(out)
}

fn encode_enums(w: &mut W, xs: &[BytecodeEnumDef]) {
    w.u32(xs.len() as u32);
    for e in xs {
        w.str(&e.name);
        w.u32(e.variants.len() as u32);
        for v in &e.variants {
            w.str(&v.name);
            encode_strs(w, &v.fields);
        }
    }
}

fn decode_enums(r: &mut R) -> Result<Vec<BytecodeEnumDef>, String> {
    let n = r.u32()? as usize;
    let mut out = Vec::with_capacity(n);
    for _ in 0..n {
        let name = r.str()?;
        let vn = r.u32()? as usize;
        let mut variants = Vec::with_capacity(vn);
        for _ in 0..vn {
            variants.push(BytecodeEnumVariantDef {
                name: r.str()?,
                fields: decode_strs(r)?,
            });
        }
        out.push(BytecodeEnumDef { name, variants });
    }
    Ok(out)
}

fn encode_ops(w: &mut W, ops: &[Opcode]) {
    w.u32(ops.len() as u32);
    for op in ops {
        encode_op(w, op);
    }
}

fn decode_ops(r: &mut R) -> Result<Vec<Opcode>, String> {
    let n = r.u32()? as usize;
    let mut out = Vec::with_capacity(n);
    for _ in 0..n {
        out.push(decode_op(r)?);
    }
    Ok(out)
}

fn encode_op(w: &mut W, op: &Opcode) {
    match *op {
        Opcode::Const(i) => {
            w.u8(0);
            w.u16(i);
        }
        Opcode::LoadLocal(i) => {
            w.u8(1);
            w.u16(i);
        }
        Opcode::StoreLocal(i) => {
            w.u8(2);
            w.u16(i);
        }
        Opcode::LoadGlobal(i) => {
            w.u8(3);
            w.u16(i);
        }
        Opcode::StoreGlobal(i) => {
            w.u8(4);
            w.u16(i);
        }
        Opcode::Pop => w.u8(5),
        Opcode::Add => w.u8(6),
        Opcode::Sub => w.u8(7),
        Opcode::Mul => w.u8(8),
        Opcode::Div => w.u8(9),
        Opcode::Mod => w.u8(10),
        Opcode::Pow => w.u8(11),
        Opcode::Eq => w.u8(12),
        Opcode::Ne => w.u8(13),
        Opcode::Lt => w.u8(14),
        Opcode::Le => w.u8(15),
        Opcode::Gt => w.u8(16),
        Opcode::Ge => w.u8(17),
        Opcode::And => w.u8(18),
        Opcode::Or => w.u8(19),
        Opcode::In => w.u8(20),
        Opcode::BitAnd => w.u8(21),
        Opcode::BitOr => w.u8(22),
        Opcode::BitXor => w.u8(23),
        Opcode::Shl => w.u8(24),
        Opcode::Shr => w.u8(25),
        Opcode::Ushr => w.u8(26),
        Opcode::JumpIfNotNullish(o) => {
            w.u8(27);
            w.i32(o);
        }
        Opcode::Not => w.u8(28),
        Opcode::Neg => w.u8(29),
        Opcode::BitNot => w.u8(30),
        Opcode::Jump(o) => {
            w.u8(31);
            w.i32(o);
        }
        Opcode::JumpIfFalse(o) => {
            w.u8(32);
            w.i32(o);
        }
        Opcode::Call(n) => {
            w.u8(33);
            w.u8(n);
        }
        Opcode::Dup => w.u8(34),
        Opcode::MakeArray(n) => {
            w.u8(35);
            w.u8(n);
        }
        Opcode::MakeObject(n) => {
            w.u8(36);
            w.u8(n);
        }
        Opcode::IndexGet => w.u8(37),
        Opcode::IndexSet => w.u8(38),
        Opcode::GetLength => w.u8(39),
        Opcode::ArrayPush => w.u8(40),
        Opcode::TakeLocal(i) => {
            w.u8(41);
            w.u16(i);
        }
        Opcode::TakeGlobal(i) => {
            w.u8(42);
            w.u16(i);
        }
        Opcode::ArrayPushLocal(i) => {
            w.u8(43);
            w.u16(i);
        }
        Opcode::ArrayPushGlobal(i) => {
            w.u8(44);
            w.u16(i);
        }
        Opcode::ArrayPopLocal(i) => {
            w.u8(45);
            w.u16(i);
        }
        Opcode::ArrayPopGlobal(i) => {
            w.u8(46);
            w.u16(i);
        }
        Opcode::AccAddLocal(i) => {
            w.u8(47);
            w.u16(i);
        }
        Opcode::AccAddGlobal(i) => {
            w.u8(48);
            w.u16(i);
        }
        Opcode::LenLocal(i) => {
            w.u8(49);
            w.u16(i);
        }
        Opcode::LenGlobal(i) => {
            w.u8(50);
            w.u16(i);
        }
        Opcode::IndexGetLocal(i) => {
            w.u8(51);
            w.u16(i);
        }
        Opcode::IndexGetGlobal(i) => {
            w.u8(52);
            w.u16(i);
        }
        Opcode::GetMember(i) => {
            w.u8(53);
            w.u16(i);
        }
        Opcode::MemberSet(i) => {
            w.u8(54);
            w.u16(i);
        }
        Opcode::Swap => w.u8(55),
        Opcode::ConcatArray => w.u8(56),
        Opcode::MergeObject => w.u8(57),
        Opcode::CallFromArray => w.u8(58),
        Opcode::MakeOk => w.u8(59),
        Opcode::MakeErr => w.u8(60),
        Opcode::MakeSome => w.u8(61),
        Opcode::MakeNone => w.u8(62),
        Opcode::JumpIfResultErr(o) => {
            w.u8(63);
            w.i32(o);
        }
        Opcode::ArraySliceFrom(n) => {
            w.u8(64);
            w.u8(n);
        }
        Opcode::MakeArrowFn(i) => {
            w.u8(65);
            w.u16(i);
        }
        Opcode::JumpUnlessResultOk(o) => {
            w.u8(66);
            w.i32(o);
        }
        Opcode::UnwrapResultOk => w.u8(67),
        Opcode::JumpUnlessResultErr(o) => {
            w.u8(68);
            w.i32(o);
        }
        Opcode::UnwrapResultErr => w.u8(69),
        Opcode::JumpUnlessOptionSome(o) => {
            w.u8(70);
            w.i32(o);
        }
        Opcode::UnwrapOptionSome => w.u8(71),
        Opcode::JumpUnlessOptionNone(o) => {
            w.u8(72);
            w.i32(o);
        }
        Opcode::JumpUnlessEnumVariant(a, b, o) => {
            w.u8(73);
            w.u16(a);
            w.u16(b);
            w.i32(o);
        }
        Opcode::UnpackEnumFields(n) => {
            w.u8(74);
            w.u8(n);
        }
        Opcode::JumpUnlessConstEq(i, o) => {
            w.u8(75);
            w.u16(i);
            w.i32(o);
        }
        Opcode::JumpUnlessArray(o) => {
            w.u8(76);
            w.i32(o);
        }
        Opcode::JumpUnlessObject(o) => {
            w.u8(77);
            w.i32(o);
        }
        Opcode::JumpUnlessObjectEmpty(o) => {
            w.u8(78);
            w.i32(o);
        }
        Opcode::JumpUnlessHasMember(i, o) => {
            w.u8(79);
            w.u16(i);
            w.i32(o);
        }
        Opcode::IndexPeekFromEnd(n) => {
            w.u8(80);
            w.u8(n);
        }
        Opcode::ArraySliceRest(a, b) => {
            w.u8(81);
            w.u8(a);
            w.u8(b);
        }
        Opcode::ObjectRest(n) => {
            w.u8(82);
            w.u8(n);
        }
        Opcode::Await => w.u8(83),
        Opcode::Yield => w.u8(84),
        Opcode::YieldStar => w.u8(85),
        Opcode::IteratorStepInPlace => w.u8(86),
        Opcode::AsyncIteratorStepInPlace => w.u8(87),
        Opcode::NewInstance(c, a) => {
            w.u8(88);
            w.u16(c);
            w.u8(a);
        }
        Opcode::NewInstanceFromArray(c) => {
            w.u8(89);
            w.u16(c);
        }
        Opcode::GetSuperMethod(i) => {
            w.u8(90);
            w.u16(i);
        }
        Opcode::ResultQuestion => w.u8(91),
        Opcode::MatchFail => w.u8(92),
        Opcode::Throw => w.u8(93),
        Opcode::Return => w.u8(94),
        Opcode::Halt => w.u8(95),
    }
}

fn decode_op(r: &mut R) -> Result<Opcode, String> {
    Ok(match r.u8()? {
        0 => Opcode::Const(r.u16()?),
        1 => Opcode::LoadLocal(r.u16()?),
        2 => Opcode::StoreLocal(r.u16()?),
        3 => Opcode::LoadGlobal(r.u16()?),
        4 => Opcode::StoreGlobal(r.u16()?),
        5 => Opcode::Pop,
        6 => Opcode::Add,
        7 => Opcode::Sub,
        8 => Opcode::Mul,
        9 => Opcode::Div,
        10 => Opcode::Mod,
        11 => Opcode::Pow,
        12 => Opcode::Eq,
        13 => Opcode::Ne,
        14 => Opcode::Lt,
        15 => Opcode::Le,
        16 => Opcode::Gt,
        17 => Opcode::Ge,
        18 => Opcode::And,
        19 => Opcode::Or,
        20 => Opcode::In,
        21 => Opcode::BitAnd,
        22 => Opcode::BitOr,
        23 => Opcode::BitXor,
        24 => Opcode::Shl,
        25 => Opcode::Shr,
        26 => Opcode::Ushr,
        27 => Opcode::JumpIfNotNullish(r.i32()?),
        28 => Opcode::Not,
        29 => Opcode::Neg,
        30 => Opcode::BitNot,
        31 => Opcode::Jump(r.i32()?),
        32 => Opcode::JumpIfFalse(r.i32()?),
        33 => Opcode::Call(r.u8()?),
        34 => Opcode::Dup,
        35 => Opcode::MakeArray(r.u8()?),
        36 => Opcode::MakeObject(r.u8()?),
        37 => Opcode::IndexGet,
        38 => Opcode::IndexSet,
        39 => Opcode::GetLength,
        40 => Opcode::ArrayPush,
        41 => Opcode::TakeLocal(r.u16()?),
        42 => Opcode::TakeGlobal(r.u16()?),
        43 => Opcode::ArrayPushLocal(r.u16()?),
        44 => Opcode::ArrayPushGlobal(r.u16()?),
        45 => Opcode::ArrayPopLocal(r.u16()?),
        46 => Opcode::ArrayPopGlobal(r.u16()?),
        47 => Opcode::AccAddLocal(r.u16()?),
        48 => Opcode::AccAddGlobal(r.u16()?),
        49 => Opcode::LenLocal(r.u16()?),
        50 => Opcode::LenGlobal(r.u16()?),
        51 => Opcode::IndexGetLocal(r.u16()?),
        52 => Opcode::IndexGetGlobal(r.u16()?),
        53 => Opcode::GetMember(r.u16()?),
        54 => Opcode::MemberSet(r.u16()?),
        55 => Opcode::Swap,
        56 => Opcode::ConcatArray,
        57 => Opcode::MergeObject,
        58 => Opcode::CallFromArray,
        59 => Opcode::MakeOk,
        60 => Opcode::MakeErr,
        61 => Opcode::MakeSome,
        62 => Opcode::MakeNone,
        63 => Opcode::JumpIfResultErr(r.i32()?),
        64 => Opcode::ArraySliceFrom(r.u8()?),
        65 => Opcode::MakeArrowFn(r.u16()?),
        66 => Opcode::JumpUnlessResultOk(r.i32()?),
        67 => Opcode::UnwrapResultOk,
        68 => Opcode::JumpUnlessResultErr(r.i32()?),
        69 => Opcode::UnwrapResultErr,
        70 => Opcode::JumpUnlessOptionSome(r.i32()?),
        71 => Opcode::UnwrapOptionSome,
        72 => Opcode::JumpUnlessOptionNone(r.i32()?),
        73 => Opcode::JumpUnlessEnumVariant(r.u16()?, r.u16()?, r.i32()?),
        74 => Opcode::UnpackEnumFields(r.u8()?),
        75 => Opcode::JumpUnlessConstEq(r.u16()?, r.i32()?),
        76 => Opcode::JumpUnlessArray(r.i32()?),
        77 => Opcode::JumpUnlessObject(r.i32()?),
        78 => Opcode::JumpUnlessObjectEmpty(r.i32()?),
        79 => Opcode::JumpUnlessHasMember(r.u16()?, r.i32()?),
        80 => Opcode::IndexPeekFromEnd(r.u8()?),
        81 => Opcode::ArraySliceRest(r.u8()?, r.u8()?),
        82 => Opcode::ObjectRest(r.u8()?),
        83 => Opcode::Await,
        84 => Opcode::Yield,
        85 => Opcode::YieldStar,
        86 => Opcode::IteratorStepInPlace,
        87 => Opcode::AsyncIteratorStepInPlace,
        88 => Opcode::NewInstance(r.u16()?, r.u8()?),
        89 => Opcode::NewInstanceFromArray(r.u16()?),
        90 => Opcode::GetSuperMethod(r.u16()?),
        91 => Opcode::ResultQuestion,
        92 => Opcode::MatchFail,
        93 => Opcode::Throw,
        94 => Opcode::Return,
        95 => Opcode::Halt,
        t => return Err(format!("kbcb v2 bad op tag {t}")),
    })
}
