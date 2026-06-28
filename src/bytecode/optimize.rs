//! Bytecode optimizer — constant folding, peephole, dead-code trimming.

use super::types::{BytecodeModule, Constant, Opcode};

#[derive(Debug, Default, Clone, Copy)]
pub struct OptStats {
    pub folds: usize,
    pub peepholes: usize,
    pub dead_removed: usize,
}

pub fn optimize_module(module: &mut BytecodeModule) -> OptStats {
    let mut stats = OptStats::default();
    let s = optimize_chunk(&mut module.main_code, &mut module.constants);
    stats.folds += s.folds;
    stats.peepholes += s.peepholes;
    stats.dead_removed += s.dead_removed;

    for f in &mut module.functions {
        let s = optimize_chunk(&mut f.code, &mut f.constants);
        stats.folds += s.folds;
        stats.peepholes += s.peepholes;
        stats.dead_removed += s.dead_removed;
    }
    for f in &mut module.arrow_functions {
        let s = optimize_chunk(&mut f.code, &mut f.constants);
        stats.folds += s.folds;
        stats.peepholes += s.peepholes;
        stats.dead_removed += s.dead_removed;
    }
    for c in &mut module.classes {
        for m in &mut c.methods {
            let s = optimize_chunk(&mut m.code, &mut m.constants);
            stats.folds += s.folds;
            stats.peepholes += s.peepholes;
            stats.dead_removed += s.dead_removed;
        }
    }
    stats
}

fn optimize_chunk(code: &mut Vec<Opcode>, constants: &mut Vec<Constant>) -> OptStats {
    let mut stats = OptStats::default();
    fold_constants(code, constants, &mut stats);
    peephole(code, &mut stats);
    trim_dead_after_halt(code, &mut stats);
    stats
}

fn fold_constants(code: &mut Vec<Opcode>, constants: &mut Vec<Constant>, stats: &mut OptStats) {
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

fn peephole(code: &mut Vec<Opcode>, stats: &mut OptStats) {
    let mut i = 0;
    while i < code.len() {
        let removed = match peephole_step(code, i) {
            Some(n) => n,
            None => {
                i += 1;
                continue;
            }
        };
        adjust_jump_targets(code, i, removed);
        stats.peepholes += removed;
    }
}

fn peephole_step(code: &mut Vec<Opcode>, i: usize) -> Option<usize> {
    match &code[i] {
        Opcode::Jump(0) | Opcode::JumpIfFalse(0) | Opcode::JumpIfResultErr(0) => {
            code.remove(i);
            Some(1)
        }
        Opcode::Const(_) if i + 1 < code.len() && matches!(code[i + 1], Opcode::Pop) => {
            code.remove(i);
            code.remove(i);
            Some(2)
        }
        Opcode::Not if i > 0 && matches!(code[i - 1], Opcode::Not) => {
            code.remove(i);
            Some(1)
        }
        _ => None,
    }
}

fn jump_target(ip: usize, op: &Opcode) -> Option<usize> {
    let off = match op {
        Opcode::Jump(off)
        | Opcode::JumpIfFalse(off)
        | Opcode::JumpIfResultErr(off)
        | Opcode::JumpUnlessResultOk(off)
        | Opcode::JumpUnlessResultErr(off)
        | Opcode::JumpUnlessOptionSome(off)
        | Opcode::JumpUnlessOptionNone(off)
        | Opcode::JumpUnlessConstEq(_, off)
        | Opcode::JumpUnlessArray(off)
        | Opcode::JumpUnlessObject(off)
        | Opcode::JumpUnlessObjectEmpty(off)
        | Opcode::JumpUnlessHasMember(_, off)
        | Opcode::JumpIfNotNullish(off) => *off,
        _ => return None,
    };
    Some(((ip as i32 + 1) + off) as usize)
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

fn trim_dead_after_halt(code: &mut Vec<Opcode>, stats: &mut OptStats) {
    // Only trim after `Halt`. `Return` may appear mid-function (e.g. if-then) with
    // live fallthrough code still to emit (else / following statements).
    if let Some(pos) = code.iter().position(|op| matches!(op, Opcode::Halt)) {
        let end = pos + 1;
        if end < code.len() {
            let removed = code.len() - end;
            code.truncate(end);
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
        optimize_chunk(&mut code, &mut constants);
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
        let stats = optimize_chunk(&mut code, &mut constants);
        assert!(stats.folds >= 1);
        assert!(matches!(code.first(), Some(Opcode::Const(2))));
    }
}
