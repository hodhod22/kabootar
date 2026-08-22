//! Bytecode optimizer — constant folding, peephole, dead-code trimming.
//! Try-region absolute IPs are remapped when ops are removed (not skipped).

use super::types::{BytecodeFnDef, BytecodeModule, Constant, GeneratorTryRegion, Opcode};

#[derive(Debug, Default, Clone, Copy)]
pub struct OptStats {
    pub folds: usize,
    pub peepholes: usize,
    pub dead_removed: usize,
    pub inlines: usize,
}

pub fn optimize_module(module: &mut BytecodeModule) -> OptStats {
    let mut stats = OptStats::default();
    let s = optimize_chunk(
        &mut module.main_code,
        &mut module.constants,
        &mut module.main_try_regions,
    );
    stats.folds += s.folds;
    stats.peepholes += s.peepholes;
    stats.dead_removed += s.dead_removed;

    for f in &mut module.functions {
        let s = optimize_chunk(&mut f.code, &mut f.constants, &mut f.try_regions);
        stats.folds += s.folds;
        stats.peepholes += s.peepholes;
        stats.dead_removed += s.dead_removed;
    }
    for f in &mut module.arrow_functions {
        let s = optimize_chunk(&mut f.code, &mut f.constants, &mut f.try_regions);
        stats.folds += s.folds;
        stats.peepholes += s.peepholes;
        stats.dead_removed += s.dead_removed;
    }
    for c in &mut module.classes {
        for m in &mut c.methods {
            let s = optimize_chunk(&mut m.code, &mut m.constants, &mut m.try_regions);
            stats.folds += s.folds;
            stats.peepholes += s.peepholes;
            stats.dead_removed += s.dead_removed;
        }
    }
    let n = inline_small_accessors(module);
    stats.inlines += n;
    stats
}

fn optimize_chunk(
    code: &mut Vec<Opcode>,
    constants: &mut Vec<Constant>,
    regions: &mut [GeneratorTryRegion],
) -> OptStats {
    let mut stats = OptStats::default();
    for _ in 0..8 {
        let before = stats.folds + stats.peepholes + stats.dead_removed;
        fold_constants(code, constants, regions, &mut stats);
        fold_unaries(code, constants, regions, &mut stats);
        peephole(code, constants, regions, &mut stats);
        trim_dead_after_halt(code, regions, &mut stats);
        if stats.folds + stats.peepholes + stats.dead_removed == before {
            break;
        }
    }
    stats
}

fn map_ip_after_remove(ip: usize, removed_at: usize, count: usize) -> usize {
    if ip <= removed_at {
        ip
    } else if ip < removed_at + count {
        removed_at
    } else {
        ip - count
    }
}

fn remap_try_regions(regions: &mut [GeneratorTryRegion], removed_at: usize, count: usize) {
    for r in regions.iter_mut() {
        r.body_start = map_ip_after_remove(r.body_start, removed_at, count);
        r.body_end = map_ip_after_remove(r.body_end, removed_at, count);
        r.catch_start = map_ip_after_remove(r.catch_start, removed_at, count);
        if r.body_end < r.body_start {
            r.body_end = r.body_start;
        }
    }
}

fn fold_constants(
    code: &mut Vec<Opcode>,
    constants: &mut Vec<Constant>,
    regions: &mut [GeneratorTryRegion],
    stats: &mut OptStats,
) {
    let mut i = 0;
    while i + 2 < code.len() {
        let (ai, bi) = match (&code[i], &code[i + 1]) {
            (Opcode::Const(a), Opcode::Const(b)) => (*a, *b),
            _ => {
                i += 1;
                continue;
            }
        };
        let binop = code[i + 2].clone();
        if let (Some(ca), Some(cb)) = (
            constants.get(ai as usize),
            constants.get(bi as usize),
        ) {
            if let Some(folded) = fold_binop(ca, cb, &binop) {
                let idx = find_or_push_const(constants, folded);
                code[i] = Opcode::Const(idx);
                code.remove(i + 1);
                code.remove(i + 1);
                remap_try_regions(regions, i + 1, 2);
                adjust_jump_targets(code, i + 1, 2);
                stats.folds += 1;
                continue;
            }
        }
        i += 1;
    }
}

fn find_or_push_const(constants: &mut Vec<Constant>, c: Constant) -> u16 {
    if let Some(i) = constants.iter().position(|x| x == &c) {
        i as u16
    } else {
        let i = constants.len();
        constants.push(c);
        i as u16
    }
}

fn fold_binop(a: &Constant, b: &Constant, op: &Opcode) -> Option<Constant> {
    match (a, b, op) {
        (Constant::Number(x), Constant::Number(y), Opcode::Add) => Some(Constant::Number(x + y)),
        (Constant::Number(x), Constant::Number(y), Opcode::Sub) => Some(Constant::Number(x - y)),
        (Constant::Number(x), Constant::Number(y), Opcode::Mul) => Some(Constant::Number(x * y)),
        (Constant::Number(x), Constant::Number(y), Opcode::Div) if *y != 0 => {
            Some(Constant::Number(x / y))
        }
        (Constant::Number(x), Constant::Number(y), Opcode::Mod) if *y != 0 => {
            Some(Constant::Number(x % y))
        }
        (Constant::Number(x), Constant::Number(y), Opcode::Pow) if *y >= 0 && *y <= 32 => {
            Some(Constant::Number(x.pow(*y as u32)))
        }
        (Constant::Number(x), Constant::Number(y), Opcode::BitAnd) => Some(Constant::Number(x & y)),
        (Constant::Number(x), Constant::Number(y), Opcode::BitOr) => Some(Constant::Number(x | y)),
        (Constant::Number(x), Constant::Number(y), Opcode::BitXor) => Some(Constant::Number(x ^ y)),
        (Constant::Number(x), Constant::Number(y), Opcode::Shl) if *y >= 0 && *y < 64 => {
            Some(Constant::Number(x.wrapping_shl(*y as u32)))
        }
        (Constant::Number(x), Constant::Number(y), Opcode::Shr) if *y >= 0 && *y < 64 => {
            Some(Constant::Number(x >> y))
        }
        (Constant::String(x), Constant::String(y), Opcode::Add) => {
            Some(Constant::String(format!("{x}{y}")))
        }
        (Constant::Number(x), Constant::Number(y), Opcode::Eq) => {
            Some(Constant::Bool(x == y))
        }
        (Constant::Number(x), Constant::Number(y), Opcode::Ne) => {
            Some(Constant::Bool(x != y))
        }
        (Constant::Number(x), Constant::Number(y), Opcode::Lt) => {
            Some(Constant::Bool(x < y))
        }
        (Constant::Number(x), Constant::Number(y), Opcode::Le) => {
            Some(Constant::Bool(x <= y))
        }
        (Constant::Number(x), Constant::Number(y), Opcode::Gt) => {
            Some(Constant::Bool(x > y))
        }
        (Constant::Number(x), Constant::Number(y), Opcode::Ge) => {
            Some(Constant::Bool(x >= y))
        }
        (Constant::Bool(x), Constant::Bool(y), Opcode::And) => Some(Constant::Bool(*x && *y)),
        (Constant::Bool(x), Constant::Bool(y), Opcode::Or) => Some(Constant::Bool(*x || *y)),
        _ => None,
    }
}

fn fold_unary(c: &Constant, op: &Opcode) -> Option<Constant> {
    match (c, op) {
        (Constant::Bool(b), Opcode::Not) => Some(Constant::Bool(!*b)),
        (Constant::Number(0), Opcode::Not) => Some(Constant::Bool(true)),
        (Constant::Number(_), Opcode::Not) => Some(Constant::Bool(false)),
        (Constant::Null | Constant::Undefined, Opcode::Not) => Some(Constant::Bool(true)),
        (Constant::Number(n), Opcode::Neg) => Some(Constant::Number(-n)),
        (Constant::Float(n), Opcode::Neg) => Some(Constant::Float(-n)),
        (Constant::Number(n), Opcode::BitNot) => Some(Constant::Number(!n)),
        _ => None,
    }
}

fn fold_unaries(
    code: &mut Vec<Opcode>,
    constants: &mut Vec<Constant>,
    regions: &mut [GeneratorTryRegion],
    stats: &mut OptStats,
) {
    let mut i = 0;
    while i + 1 < code.len() {
        let ci = match &code[i] {
            Opcode::Const(c) => *c,
            _ => {
                i += 1;
                continue;
            }
        };
        let uop = code[i + 1].clone();
        if matches!(uop, Opcode::Not | Opcode::Neg | Opcode::BitNot) {
            if let Some(c) = constants.get(ci as usize) {
                if let Some(folded) = fold_unary(c, &uop) {
                    let idx = find_or_push_const(constants, folded);
                    code[i] = Opcode::Const(idx);
                    code.remove(i + 1);
                    remap_try_regions(regions, i + 1, 1);
                    adjust_jump_targets(code, i + 1, 1);
                    stats.folds += 1;
                    continue;
                }
            }
        }
        i += 1;
    }
}

fn const_is_truthy(c: &Constant) -> Option<bool> {
    match c {
        Constant::Bool(b) => Some(*b),
        Constant::Number(0) | Constant::Null | Constant::Undefined => Some(false),
        Constant::Number(_) => Some(true),
        Constant::Float(n) => Some(*n != 0.0 && !n.is_nan()),
        Constant::String(s) => Some(!s.is_empty()),
        Constant::Nan => Some(false),
        _ => None,
    }
}

fn next_live(live: &[Option<Opcode>], mut i: usize) -> Option<usize> {
    while i < live.len() {
        if live[i].is_some() {
            return Some(i);
        }
        i += 1;
    }
    None
}

fn jump_offset(op: &Opcode) -> Option<i32> {
    match op {
        Opcode::Jump(o)
        | Opcode::JumpIfFalse(o)
        | Opcode::JumpIfResultErr(o)
        | Opcode::JumpUnlessResultOk(o)
        | Opcode::JumpUnlessResultErr(o)
        | Opcode::JumpUnlessOptionSome(o)
        | Opcode::JumpUnlessOptionNone(o)
        | Opcode::JumpUnlessEnumVariant(_, _, o)
        | Opcode::JumpUnlessConstEq(_, o)
        | Opcode::JumpUnlessArray(o)
        | Opcode::JumpUnlessObject(o)
        | Opcode::JumpUnlessObjectEmpty(o)
        | Opcode::JumpUnlessHasMember(_, o)
        | Opcode::JumpIfNotNullish(o) => Some(*o),
        _ => None,
    }
}

/// O(n) peephole: mark deletions, then compact. `Vec::remove` per hit is O(n²) on
/// large main chunks (SH14 100k `s = s + 1` → Const+Pop after every AccAdd).
fn peephole(code: &mut Vec<Opcode>, constants: &[Constant], regions: &mut [GeneratorTryRegion], stats: &mut OptStats) {
    if code.is_empty() {
        return;
    }
    let mut live: Vec<Option<Opcode>> = code.iter().cloned().map(Some).collect();
    let mut i = 0;
    while i < live.len() {
        let Some(op) = live[i].clone() else {
            i += 1;
            continue;
        };
        match &op {
            Opcode::Jump(0) | Opcode::JumpIfFalse(0) | Opcode::JumpIfResultErr(0) => {
                live[i] = None;
                stats.peepholes += 1;
            }
            Opcode::Const(ci) => {
                if let Some(j) = next_live(&live, i + 1) {
                    match live[j].clone() {
                        Some(Opcode::IndexGet) => match constants.get(*ci as usize) {
                            Some(Constant::String(s)) if ident_member_key(s) => {
                                live[i] = Some(Opcode::GetMember(*ci));
                                live[j] = None;
                                stats.peepholes += 1;
                            }
                            _ => {}
                        },
                        Some(Opcode::IndexGetLocal(li)) => match constants.get(*ci as usize) {
                            Some(Constant::String(s)) if ident_member_key(s) => {
                                live[i] = Some(Opcode::LoadLocal(li));
                                live[j] = Some(Opcode::GetMember(*ci));
                                stats.peepholes += 1;
                            }
                            _ => {}
                        },
                        Some(Opcode::IndexGetGlobal(gi)) => match constants.get(*ci as usize) {
                            Some(Constant::String(s)) if ident_member_key(s) => {
                                live[i] = Some(Opcode::LoadGlobal(gi));
                                live[j] = Some(Opcode::GetMember(*ci));
                                stats.peepholes += 1;
                            }
                            _ => {}
                        },
                        Some(Opcode::Pop) => {
                            live[i] = None;
                            live[j] = None;
                            stats.peepholes += 2;
                        }
                        Some(Opcode::JumpIfFalse(off)) => {
                            match constants.get(*ci as usize).and_then(const_is_truthy) {
                                Some(false) => {
                                    live[i] = None;
                                    live[j] = Some(Opcode::Jump(off));
                                    stats.peepholes += 1;
                                }
                                Some(true) => {
                                    live[i] = None;
                                    live[j] = None;
                                    stats.peepholes += 2;
                                }
                                None => {}
                            }
                        }
                        _ => {}
                    }
                }
            }
            Opcode::LoadLocal(a) => {
                if let Some(j) = next_live(&live, i + 1) {
                    if matches!(live[j], Some(Opcode::StoreLocal(b)) if b == *a) {
                        live[i] = None;
                        live[j] = None;
                        stats.peepholes += 2;
                    }
                }
            }
            Opcode::Not if i > 0 => {
                if let Some(p) = (0..i).rev().find(|&k| live[k].is_some()) {
                    if matches!(live[p], Some(Opcode::Not)) {
                        live[i] = None;
                        live[p] = None;
                        stats.peepholes += 2;
                    }
                }
            }
            _ => {}
        }
        i += 1;
    }
    compact_live_ops(code, &live, regions);
}

fn compact_live_ops(
    code: &mut Vec<Opcode>,
    live: &[Option<Opcode>],
    regions: &mut [GeneratorTryRegion],
) {
    let n = live.len();
    let mut map = vec![0usize; n + 1];
    let mut kept: Vec<(usize, Opcode)> = Vec::with_capacity(n);
    for (old, slot) in live.iter().enumerate() {
        map[old] = kept.len();
        if let Some(op) = slot {
            kept.push((old, op.clone()));
        }
    }
    map[n] = kept.len();
    let mut out = Vec::with_capacity(kept.len());
    for (new_ip, (old_ip, mut op)) in kept.into_iter().enumerate() {
        if let Some(off) = jump_offset(&op) {
            let target = ((old_ip as i32) + 1 + off).clamp(0, n as i32) as usize;
            let nt = map[target];
            set_jump_target(new_ip, &mut op, nt);
        }
        out.push(op);
    }
    for r in regions.iter_mut() {
        r.body_start = map[r.body_start.min(n)];
        r.body_end = map[r.body_end.min(n)];
        r.catch_start = map[r.catch_start.min(n)];
        if r.body_end < r.body_start {
            r.body_end = r.body_start;
        }
    }
    *code = out;
}

fn set_jump_target(ip: usize, op: &mut Opcode, target: usize) {
    let off = target as i32 - ip as i32 - 1;
    match op {
        Opcode::Jump(ref mut o)
        | Opcode::JumpIfFalse(ref mut o)
        | Opcode::JumpIfResultErr(ref mut o)
        | Opcode::JumpUnlessResultOk(ref mut o)
        | Opcode::JumpUnlessResultErr(ref mut o)
        | Opcode::JumpUnlessOptionSome(ref mut o)
        | Opcode::JumpUnlessOptionNone(ref mut o)
        | Opcode::JumpUnlessEnumVariant(_, _, ref mut o)
        | Opcode::JumpUnlessConstEq(_, ref mut o)
        | Opcode::JumpUnlessArray(ref mut o)
        | Opcode::JumpUnlessObject(ref mut o)
        | Opcode::JumpUnlessObjectEmpty(ref mut o)
        | Opcode::JumpUnlessHasMember(_, ref mut o)
        | Opcode::JumpIfNotNullish(ref mut o) => *o = off,
        _ => {}
    }
}

fn adjust_jump_targets(code: &mut [Opcode], removed_at: usize, count: usize) {
    for ip in 0..code.len() {
        let off = match &code[ip] {
            Opcode::Jump(o)
            | Opcode::JumpIfFalse(o)
            | Opcode::JumpIfResultErr(o)
            | Opcode::JumpUnlessResultOk(o)
            | Opcode::JumpUnlessResultErr(o)
            | Opcode::JumpUnlessOptionSome(o)
            | Opcode::JumpUnlessOptionNone(o)
            | Opcode::JumpUnlessEnumVariant(_, _, o)
            | Opcode::JumpUnlessConstEq(_, o)
            | Opcode::JumpUnlessArray(o)
            | Opcode::JumpUnlessObject(o)
            | Opcode::JumpUnlessObjectEmpty(o)
            | Opcode::JumpUnlessHasMember(_, o)
            | Opcode::JumpIfNotNullish(o) => *o,
            _ => continue,
        };
        let old_ip = if ip >= removed_at { ip + count } else { ip };
        let target = ((old_ip as i32 + 1) + off) as usize;
        let new_target = if target > removed_at {
            target - count
        } else {
            target
        };
        set_jump_target(ip, &mut code[ip], new_target);
    }
}

fn trim_dead_after_halt(
    code: &mut Vec<Opcode>,
    regions: &mut [GeneratorTryRegion],
    stats: &mut OptStats,
) {
    // Only trim after `Halt`. `Return` may appear mid-function (e.g. if-then) with
    // live fallthrough code still to emit (else / following statements).
    if let Some(pos) = code.iter().position(|op| matches!(op, Opcode::Halt)) {
        let end = pos + 1;
        if end < code.len() {
            let removed = code.len() - end;
            code.truncate(end);
            remap_try_regions(regions, end, removed);
            stats.dead_removed += removed;
        }
    }
}

fn ident_member_key(s: &str) -> bool {
    let mut chars = s.chars();
    match chars.next() {
        Some(c) if c == '_' || c.is_ascii_alphabetic() => {
            chars.all(|c| c == '_' || c.is_ascii_alphanumeric())
        }
        _ => false,
    }
}

fn accessor_member_key(f: &BytecodeFnDef) -> Option<String> {
    if f.params.len() != 1 || f.async_fn || f.generator_fn {
        return None;
    }
    let ops: Vec<&Opcode> = f
        .code
        .iter()
        .filter(|o| !matches!(o, Opcode::Halt | Opcode::Return))
        .collect();
    match ops.as_slice() {
        [Opcode::LoadLocal(0), Opcode::GetMember(k)] => match f.constants.get(*k as usize) {
            Some(Constant::String(s)) => Some(s.clone()),
            _ => None,
        },
        [Opcode::LoadLocal(0), Opcode::Const(k), Opcode::IndexGetLocal(0)] => {
            match f.constants.get(*k as usize) {
                Some(Constant::String(s)) if ident_member_key(s) => Some(s.clone()),
                _ => None,
            }
        }
        _ => None,
    }
}

fn intern_const_string(constants: &mut Vec<Constant>, s: &str) -> u16 {
    if let Some(i) = constants
        .iter()
        .position(|c| matches!(c, Constant::String(t) if t == s))
    {
        return i as u16;
    }
    let i = constants.len();
    constants.push(Constant::String(s.to_string()));
    i as u16
}

fn inline_in_chunk(
    code: &mut Vec<Opcode>,
    constants: &mut Vec<Constant>,
    regions: &mut [GeneratorTryRegion],
    globals: &[String],
    accessors: &[(String, String)],
) -> usize {
    let mut i = 0;
    let mut n = 0;
    while i + 1 < code.len() {
        if let (Opcode::LoadGlobal(g), Opcode::Call(1)) = (&code[i], &code[i + 1]) {
            if let Some(gname) = globals.get(*g as usize) {
                if let Some((_, key)) = accessors.iter().find(|(name, _)| name == gname) {
                    let ki = intern_const_string(constants, key);
                    code[i] = Opcode::GetMember(ki);
                    code.remove(i + 1);
                    remap_try_regions(regions, i + 1, 1);
                    adjust_jump_targets(code, i + 1, 1);
                    n += 1;
                    continue;
                }
            }
        }
        i += 1;
    }
    n
}

fn inline_small_accessors(module: &mut BytecodeModule) -> usize {
    let accessors: Vec<(String, String)> = module
        .functions
        .iter()
        .filter_map(|f| Some((f.name.clone(), accessor_member_key(f)?)))
        .collect();
    if accessors.is_empty() {
        return 0;
    }
    let mut n = 0;
    n += inline_in_chunk(
        &mut module.main_code,
        &mut module.constants,
        &mut module.main_try_regions,
        &module.globals,
        &accessors,
    );
    let globals = module.globals.clone();
    for f in &mut module.functions {
        n += inline_in_chunk(
            &mut f.code,
            &mut f.constants,
            &mut f.try_regions,
            &globals,
            &accessors,
        );
    }
    for f in &mut module.arrow_functions {
        n += inline_in_chunk(
            &mut f.code,
            &mut f.constants,
            &mut f.try_regions,
            &globals,
            &accessors,
        );
    }
    n
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn preserves_code_after_if_then_return() {
        let mut code = vec![
            Opcode::LoadLocal(0),
            Opcode::Const(0),
            Opcode::Lt,
            Opcode::JumpIfFalse(2),
            Opcode::Const(1),
            Opcode::Return,
            Opcode::Const(2),
            Opcode::Return,
        ];
        let mut constants = vec![
            Constant::Number(3),
            Constant::Number(1),
            Constant::Number(2),
        ];
        let mut regions = vec![];
        optimize_chunk(&mut code, &mut constants, &mut regions);
        assert_eq!(code.len(), 8, "must not truncate fallthrough return after if-then return");
    }

    #[test]
    fn folds_add() {
        let constants = vec![
            Constant::Number(1),
            Constant::Number(2),
            Constant::Number(3),
        ];
        let mut code = vec![
            Opcode::Const(0),
            Opcode::Const(1),
            Opcode::Add,
            Opcode::Halt,
        ];
        let mut constants = constants;
        let mut regions = vec![];
        let stats = optimize_chunk(&mut code, &mut constants, &mut regions);
        assert!(stats.folds >= 1);
        assert!(matches!(code.first(), Some(Opcode::Const(2))));
    }

    #[test]
    fn drops_const_pop_without_n_squared_remove() {
        let mut constants = vec![Constant::Null];
        let mut code = Vec::new();
        for _ in 0..8 {
            code.push(Opcode::AccAddLocal(0));
            code.push(Opcode::Const(0));
            code.push(Opcode::Pop);
        }
        code.push(Opcode::Halt);
        let mut regions = vec![];
        let stats = optimize_chunk(&mut code, &mut constants, &mut regions);
        assert!(stats.peepholes >= 16);
        assert_eq!(
            code.iter().filter(|o| matches!(o, Opcode::AccAddLocal(0))).count(),
            8
        );
        assert!(!code.iter().any(|o| matches!(o, Opcode::Pop)));
        assert!(matches!(code.last(), Some(Opcode::Halt)));
    }

    #[test]
    fn folds_const_false_jump_if_false_to_jump() {
        let mut constants = vec![Constant::Bool(false), Constant::Bool(true)];
        let mut code = vec![
            Opcode::Const(0),
            Opcode::JumpIfFalse(1),
            Opcode::Const(1),
            Opcode::Halt,
        ];
        let mut regions = vec![];
        let stats = optimize_chunk(&mut code, &mut constants, &mut regions);
        assert!(stats.peepholes >= 1);
        assert!(matches!(code.first(), Some(Opcode::Jump(1))));
    }

    #[test]
    fn removes_double_not() {
        let mut code = vec![Opcode::Not, Opcode::Not, Opcode::Halt];
        let mut constants = vec![];
        let mut regions = vec![];
        optimize_chunk(&mut code, &mut constants, &mut regions);
        assert_eq!(code, vec![Opcode::Halt]);
    }

    #[test]
    fn removes_load_store_same_local() {
        let mut code = vec![
            Opcode::LoadLocal(2),
            Opcode::StoreLocal(2),
            Opcode::Halt,
        ];
        let mut constants = vec![];
        let mut regions = vec![];
        optimize_chunk(&mut code, &mut constants, &mut regions);
        assert_eq!(code, vec![Opcode::Halt]);
    }

    #[test]
    fn remaps_try_regions_on_fold() {
        let mut constants = vec![
            Constant::Number(1),
            Constant::Number(2),
            Constant::Number(3),
        ];
        // body: const 0, const 1, add  | catch at index 4 after fold becomes 2
        let mut code = vec![
            Opcode::Const(0),
            Opcode::Const(1),
            Opcode::Add,
            Opcode::JumpIfResultErr(1),
            Opcode::Jump(1),
            Opcode::LoadLocal(0),
            Opcode::Halt,
        ];
        let mut regions = vec![GeneratorTryRegion {
            body_start: 0,
            body_end: 2,
            catch_start: 5,
            err_local: 0,
        }];
        optimize_chunk(&mut code, &mut constants, &mut regions);
        assert_eq!(regions[0].body_start, 0);
        assert!(regions[0].body_end <= regions[0].catch_start);
        assert!(regions[0].catch_start < code.len());
    }

    #[test]
    fn inlines_small_member_accessor() {
        let src = r#"
fn peek(n) { return n.kind }
let x = { "kind": 7 }
peek(x)
"#;
        let prog = crate::bytecode::compile_source(src).expect("compile");
        let m = prog.bytecode.expect("bc");
        let has_get = m
            .main_code
            .iter()
            .any(|op| matches!(op, Opcode::GetMember(_)));
        let call1 = m
            .main_code
            .iter()
            .filter(|op| matches!(op, Opcode::Call(1)))
            .count();
        assert!(
            has_get,
            "expected GetMember in main after inline: {:?}",
            m.main_code
        );
        assert_eq!(
            call1, 0,
            "LoadGlobal+Call(1) peek should be inlined, code={:?}",
            m.main_code
        );
    }

    #[test]
    fn const_ident_indexget_becomes_getmember() {
        let mut constants = vec![Constant::String("pCur".into())];
        let mut code = vec![
            Opcode::LoadLocal(0),
            Opcode::Const(0),
            Opcode::IndexGet,
            Opcode::Halt,
        ];
        let mut regions = vec![];
        let stats = optimize_chunk(&mut code, &mut constants, &mut regions);
        assert!(stats.peepholes >= 1);
        assert!(
            matches!(code.as_slice(), [Opcode::LoadLocal(0), Opcode::GetMember(0), Opcode::Halt]),
            "got {code:?}"
        );
    }

    #[test]
    fn const_ident_indexget_local_becomes_getmember() {
        let mut constants = vec![Constant::String("pCur".into())];
        let mut code = vec![
            Opcode::Const(0),
            Opcode::IndexGetLocal(0),
            Opcode::Halt,
        ];
        let mut regions = vec![];
        let stats = optimize_chunk(&mut code, &mut constants, &mut regions);
        assert!(stats.peepholes >= 1);
        assert!(
            matches!(code.as_slice(), [Opcode::LoadLocal(0), Opcode::GetMember(0), Opcode::Halt]),
            "got {code:?}"
        );
    }

    #[test]
    fn inlines_index_peek_accessor() {
        let src = r#"
fn peek(n) { return n["pCur"] }
let x = { "pCur": 9 }
peek(x)
"#;
        let prog = crate::bytecode::compile_source(src).expect("compile");
        let m = prog.bytecode.expect("bc");
        let has_get = m
            .main_code
            .iter()
            .any(|op| matches!(op, Opcode::GetMember(_)));
        let call1 = m
            .main_code
            .iter()
            .filter(|op| matches!(op, Opcode::Call(1)))
            .count();
        assert!(has_get, "expected GetMember after sess[\"pCur\"] inline: {:?}", m.main_code);
        assert_eq!(call1, 0, "peek(n[\"pCur\"]) should inline, code={:?}", m.main_code);
    }

    #[test]
    fn folds_unary_not() {
        let mut constants = vec![Constant::Bool(true)];
        let mut code = vec![Opcode::Const(0), Opcode::Not, Opcode::Halt];
        let mut regions = vec![];
        let stats = optimize_chunk(&mut code, &mut constants, &mut regions);
        assert!(stats.folds >= 1);
        assert!(matches!(code.first(), Some(Opcode::Const(_))));
        assert!(!code.iter().any(|o| matches!(o, Opcode::Not)));
    }

    #[test]
    fn folds_string_concat() {
        let mut constants = vec![
            Constant::String("ka".into()),
            Constant::String("b".into()),
        ];
        let mut code = vec![
            Opcode::Const(0),
            Opcode::Const(1),
            Opcode::Add,
            Opcode::Halt,
        ];
        let mut regions = vec![];
        let stats = optimize_chunk(&mut code, &mut constants, &mut regions);
        assert!(stats.folds >= 1);
        match code.first() {
            Some(Opcode::Const(i)) => {
                assert_eq!(constants[*i as usize], Constant::String("kab".into()));
            }
            other => panic!("expected folded const, got {other:?}"),
        }
    }

    #[test]
    fn folds_bitand() {
        let mut constants = vec![Constant::Number(6), Constant::Number(3)];
        let mut code = vec![
            Opcode::Const(0),
            Opcode::Const(1),
            Opcode::BitAnd,
            Opcode::Halt,
        ];
        let mut regions = vec![];
        let stats = optimize_chunk(&mut code, &mut constants, &mut regions);
        assert!(stats.folds >= 1);
    }
}
