//! Bytecode optimizer — constant folding, peephole, dead-code trimming.
//! Try-region absolute IPs are remapped when ops are removed (not skipped).

use super::types::{BytecodeModule, Constant, GeneratorTryRegion, Opcode};

#[derive(Debug, Default, Clone, Copy)]
pub struct OptStats {
    pub folds: usize,
    pub peepholes: usize,
    pub dead_removed: usize,
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
    stats
}

fn optimize_chunk(
    code: &mut Vec<Opcode>,
    constants: &mut Vec<Constant>,
    regions: &mut [GeneratorTryRegion],
) -> OptStats {
    let mut stats = OptStats::default();
    fold_constants(code, constants, regions, &mut stats);
    peephole(code, constants, regions, &mut stats);
    trim_dead_after_halt(code, regions, &mut stats);
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

fn peephole(code: &mut Vec<Opcode>, constants: &[Constant], regions: &mut [GeneratorTryRegion], stats: &mut OptStats) {
    let mut i = 0;
    while i < code.len() {
        let removed = match peephole_step(code, constants, i) {
            Some(n) => n,
            None => {
                i += 1;
                continue;
            }
        };
        remap_try_regions(regions, i, removed);
        adjust_jump_targets(code, i, removed);
        stats.peepholes += removed;
    }
}

fn peephole_step(code: &mut Vec<Opcode>, constants: &[Constant], i: usize) -> Option<usize> {
    match &code[i] {
        Opcode::Jump(0) | Opcode::JumpIfFalse(0) | Opcode::JumpIfResultErr(0) => {
            code.remove(i);
            Some(1)
        }
        Opcode::Const(ci) if i + 1 < code.len() => match code[i + 1].clone() {
            Opcode::Pop => {
                code.remove(i);
                code.remove(i);
                Some(2)
            }
            Opcode::JumpIfFalse(off) => match constants.get(*ci as usize) {
                Some(Constant::Bool(false)) => {
                    code.remove(i);
                    code[i] = Opcode::Jump(off);
                    Some(1)
                }
                Some(Constant::Bool(true)) => {
                    code.remove(i + 1);
                    code.remove(i);
                    Some(2)
                }
                _ => None,
            },
            _ => None,
        },
        Opcode::LoadLocal(a) if i + 1 < code.len() && matches!(code[i + 1], Opcode::StoreLocal(b) if *a == b) => {
            code.remove(i + 1);
            code.remove(i);
            Some(2)
        }
        Opcode::Not if i > 0 && matches!(code[i - 1], Opcode::Not) => {
            code.remove(i);
            code.remove(i - 1);
            Some(2)
        }
        _ => None,
    }
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
}
