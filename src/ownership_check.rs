//! Compile-time ownership check for `@manual` modules (Våg O1–O3).
//!
//! GC modules are skipped. Not Rust lifetimes — affine Owned + borrows.

use crate::ast::*;
use crate::lang_preprocess::MemoryMode;
use std::collections::HashMap;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Place {
    Owned,
    Moved,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Consume {
    Peek,
    Move,
}

#[derive(Default)]
struct Checker {
    places: HashMap<String, Place>,
    mut_borrows: HashMap<String, usize>,
    shared_borrows: HashMap<String, usize>,
    errors: Vec<String>,
}

impl Checker {
    fn err(&mut self, msg: impl Into<String>) {
        self.errors.push(msg.into());
    }

    fn check_program(&mut self, stmts: &[Stmt]) {
        for stmt in stmts {
            self.check_stmt(stmt);
        }
        self.leak_lint_owned(None);
    }

    fn check_stmt(&mut self, stmt: &Stmt) {
        match stmt {
            Stmt::Let {
                pattern,
                init,
                ..
            } => {
                let produced = if let Some(init) = init {
                    self.check_expr(init, Consume::Move)
                } else {
                    false
                };
                if let BindingPattern::Name(name) = pattern {
                    if produced {
                        self.places.insert(name.clone(), Place::Owned);
                    } else {
                        self.places.remove(name);
                    }
                }
            }
            Stmt::Expr(e) => {
                self.check_expr(e, Consume::Peek);
            }
            Stmt::Return(Some(e)) => {
                self.check_expr(e, Consume::Move);
            }
            Stmt::Return(None) | Stmt::Interface { .. } | Stmt::Enum { .. } | Stmt::Import { .. } => {}
            Stmt::Class { methods, .. } => {
                for m in methods {
                    // ClassMethod params are untyped strings today — still walk body.
                    let params: Vec<FnParam> = m
                        .params
                        .iter()
                        .map(|n| FnParam {
                            name: n.clone(),
                            type_ann: None,
                            default: None,
                        })
                        .collect();
                    self.check_fn(&params, &m.body);
                }
            }
            Stmt::Using { init, .. } => {
                self.check_expr(init, Consume::Peek);
            }
        }
    }

    fn check_fn(&mut self, params: &[FnParam], body: &Expr) {
        let snap = self.snapshot();
        for p in params {
            if is_owned_type(p.type_ann.as_ref()) {
                self.places.insert(p.name.clone(), Place::Owned);
            } else if is_ref_owned_type(p.type_ann.as_ref()) {
                self.places.remove(&p.name);
            }
        }
        self.check_expr(body, Consume::Peek);
        self.leak_lint_owned(Some(&snap.0));
        self.restore_full(&snap);
    }

    fn leak_lint_owned(&mut self, outer: Option<&HashMap<String, Place>>) {
        let names: Vec<String> = self
            .places
            .iter()
            .filter(|(name, place)| {
                **place == Place::Owned
                    && outer.map(|o| !o.contains_key(*name)).unwrap_or(true)
            })
            .map(|(name, _)| name.clone())
            .collect();
        for name in names {
            self.err(format!(
                "ownership: Owned '{name}' dropped out of scope without move/drop (leak-lint)"
            ));
        }
    }

    /// Returns true if `expr` produces an Owned value under `outer` consume mode.
    fn check_expr(&mut self, expr: &Expr, outer: Consume) -> bool {
        match expr {
            Expr::Variable(name) => {
                let was_owned = matches!(self.places.get(name), Some(Place::Owned));
                self.use_place(name, outer);
                was_owned && outer == Consume::Move
            }
            Expr::Unary(UnaryOp::Ref, inner) => {
                if let Expr::Variable(name) = inner.as_ref() {
                    self.borrow_shared(name);
                } else {
                    self.check_expr(inner, Consume::Peek);
                }
                false
            }
            Expr::Unary(UnaryOp::RefMut, inner) => {
                if let Expr::Variable(name) = inner.as_ref() {
                    self.borrow_mut(name);
                } else {
                    self.check_expr(inner, Consume::Peek);
                }
                false
            }
            Expr::Assign(target, value) => {
                let produced = self.check_expr(value, Consume::Move);
                if let AssignTarget::Name(name) = target {
                    if produced {
                        self.places.insert(name.clone(), Place::Owned);
                    } else {
                        self.places.remove(name);
                    }
                } else {
                    self.walk_assign_target(target);
                }
                false
            }
            Expr::Call { func, args, .. } => {
                let fname = call_name(func);
                let peek_api = is_peek_api(fname.as_deref());
                let move_api = is_move_api(fname.as_deref());
                let produces = is_alloc_api(fname.as_deref()) || move_api;

                if !matches!(func.as_ref(), Expr::Variable(_)) {
                    self.check_expr(func, Consume::Peek);
                }

                // Borrow lifetimes end when the call expression finishes (O3).
                let borrow_snap = (self.mut_borrows.clone(), self.shared_borrows.clone());
                for (i, arg) in args.iter().enumerate() {
                    let mode = if peek_api {
                        Consume::Peek
                    } else if move_api && i == 0 {
                        Consume::Move
                    } else if arg_is_borrow(arg) {
                        Consume::Peek
                    } else if arg_is_owned_var(arg, &self.places) {
                        Consume::Move
                    } else {
                        Consume::Peek
                    };
                    match arg {
                        CallArg::Expr(e) | CallArg::Spread(e) => {
                            self.check_expr(e, mode);
                        }
                    }
                }
                self.mut_borrows = borrow_snap.0;
                self.shared_borrows = borrow_snap.1;
                produces
            }
            Expr::Function {
                params, body, ..
            }
            | Expr::Arrow { params, body, .. } => {
                self.check_fn(params, body);
                false
            }
            Expr::Block(stmts) => {
                let snap = self.snapshot();
                let mut last = false;
                for s in stmts {
                    match s {
                        Stmt::Expr(e) => last = self.check_expr(e, Consume::Peek),
                        Stmt::Return(Some(e)) => {
                            last = self.check_expr(e, Consume::Move);
                        }
                        other => {
                            self.check_stmt(other);
                            last = false;
                        }
                    }
                }
                self.leak_lint_owned(Some(&snap.0));
                self.restore_moved_only(&snap);
                last
            }
            Expr::If(cond, then_b, else_b) => {
                self.check_expr(cond, Consume::Peek);
                let snap = self.snapshot();
                self.check_expr(then_b, Consume::Peek);
                let after_then = self.snapshot();
                self.restore_full(&snap);
                if let Some(e) = else_b {
                    self.check_expr(e, Consume::Peek);
                }
                self.merge_places(&after_then);
                false
            }
            Expr::While(cond, body) | Expr::DoWhile(body, cond) => {
                self.check_expr(cond, Consume::Peek);
                let snap = self.snapshot();
                self.check_expr(body, Consume::Peek);
                self.restore_moved_only(&snap);
                false
            }
            Expr::Binary(l, _, r) => {
                self.check_expr(l, Consume::Peek);
                self.check_expr(r, Consume::Peek);
                false
            }
            Expr::Unary(_, inner) => {
                self.check_expr(inner, Consume::Peek);
                false
            }
            Expr::Ternary(c, t, e) => {
                self.check_expr(c, Consume::Peek);
                self.check_expr(t, Consume::Peek);
                self.check_expr(e, Consume::Peek);
                false
            }
            Expr::Await(e)
            | Expr::Yield(e)
            | Expr::YieldStar(e)
            | Expr::DynamicImport(e)
            | Expr::ResultQuestion(e) => {
                self.check_expr(e, Consume::Peek);
                false
            }
            Expr::Member(obj, _, _) | Expr::OptionalMember(obj, _) => {
                self.check_expr(obj, Consume::Peek);
                false
            }
            Expr::Index(obj, idx) | Expr::OptionalIndex(obj, idx) => {
                self.check_expr(obj, Consume::Peek);
                self.check_expr(idx, Consume::Peek);
                false
            }
            Expr::OptionalCall(func, args) => {
                self.check_expr(func, Consume::Peek);
                for a in args {
                    match a {
                        CallArg::Expr(e) | CallArg::Spread(e) => {
                            self.check_expr(e, Consume::Peek);
                        }
                    }
                }
                false
            }
            Expr::Literal(lit) => self.check_literal(lit),
            Expr::Match(scrut, arms) => {
                self.check_expr(scrut, Consume::Peek);
                for arm in arms {
                    if let Some(g) = &arm.guard {
                        self.check_expr(g, Consume::Peek);
                    }
                    self.check_expr(&arm.body, Consume::Peek);
                }
                false
            }
            Expr::ForEach(f) => {
                self.check_expr(&f.iterable, Consume::Peek);
                self.check_expr(&f.body, Consume::Peek);
                false
            }
            Expr::ForClassic {
                init,
                cond,
                step,
                body,
            } => {
                if let Some(i) = init {
                    self.check_stmt(i);
                }
                if let Some(c) = cond {
                    self.check_expr(c, Consume::Peek);
                }
                if let Some(s) = step {
                    self.check_expr(s, Consume::Peek);
                }
                self.check_expr(body, Consume::Peek);
                false
            }
            Expr::TryCatch {
                body,
                handler,
                finally,
                ..
            } => {
                self.check_expr(body, Consume::Peek);
                self.check_expr(handler, Consume::Peek);
                if let Some(f) = finally {
                    self.check_expr(f, Consume::Peek);
                }
                false
            }
            Expr::With { value, body, .. } => {
                self.check_expr(value, Consume::Peek);
                self.check_expr(body, Consume::Peek);
                false
            }
            Expr::Assert { condition, message } => {
                self.check_expr(condition, Consume::Peek);
                if let Some(m) = message {
                    self.check_expr(m, Consume::Peek);
                }
                false
            }
            Expr::WhileLet {
                scrutinee, body, ..
            } => {
                self.check_expr(scrutinee, Consume::Peek);
                self.check_expr(body, Consume::Peek);
                false
            }
            Expr::IfLet {
                scrutinee,
                body,
                else_branch,
                ..
            } => {
                self.check_expr(scrutinee, Consume::Peek);
                self.check_expr(body, Consume::Peek);
                if let Some(e) = else_branch {
                    self.check_expr(e, Consume::Peek);
                }
                false
            }
            Expr::Switch {
                scrutinee,
                cases,
                default_body,
            } => {
                self.check_expr(scrutinee, Consume::Peek);
                for c in cases {
                    self.check_expr(&c.value, Consume::Peek);
                    self.check_expr(&c.body, Consume::Peek);
                }
                if let Some(d) = default_body {
                    self.check_expr(d, Consume::Peek);
                }
                false
            }
            Expr::This
            | Expr::Self_
            | Expr::Super
            | Expr::Break
            | Expr::Continue
            | Expr::Pass
            | Expr::Fallthrough
            | Expr::ImportMeta => false,
        }
    }

    fn check_literal(&mut self, lit: &Literal) -> bool {
        match lit {
            Literal::Array(items) => {
                for it in items {
                    match it {
                        ArrayPiece::Item(e) | ArrayPiece::Spread(e) => {
                            self.check_expr(e, Consume::Peek);
                        }
                    }
                }
            }
            Literal::Object(fields) => {
                for f in fields {
                    match f {
                        ObjectPiece::Field { value, .. } => {
                            self.check_expr(value, Consume::Peek);
                        }
                        ObjectPiece::Method { body, params, .. } => {
                            self.check_fn(params, body);
                        }
                        ObjectPiece::Spread(e) => {
                            self.check_expr(e, Consume::Peek);
                        }
                    }
                }
            }
            Literal::Some(e) | Literal::Ok(e) | Literal::Err(e) => {
                self.check_expr(e, Consume::Peek);
            }
            _ => {}
        }
        false
    }

    fn walk_assign_target(&mut self, target: &AssignTarget) {
        match target {
            AssignTarget::Member(obj, _) => {
                self.check_expr(obj, Consume::Peek);
            }
            AssignTarget::Index(obj, idx) => {
                self.check_expr(obj, Consume::Peek);
                self.check_expr(idx, Consume::Peek);
            }
            AssignTarget::Name(_) | AssignTarget::Pattern(_) => {}
        }
    }

    fn use_place(&mut self, name: &str, mode: Consume) {
        match self.places.get(name).copied() {
            Some(Place::Moved) => {
                self.err(format!(
                    "ownership: use after move of `{name}` (compile-time, @manual)"
                ));
            }
            Some(Place::Owned) => {
                if mode == Consume::Move {
                    if self.shared_borrows.get(name).copied().unwrap_or(0) > 0
                        || self.mut_borrows.get(name).copied().unwrap_or(0) > 0
                    {
                        self.err(format!(
                            "ownership: cannot move `{name}` while borrowed"
                        ));
                    }
                    self.places.insert(name.to_string(), Place::Moved);
                }
            }
            None => {}
        }
    }

    fn borrow_shared(&mut self, name: &str) {
        match self.places.get(name).copied() {
            Some(Place::Moved) => {
                self.err(format!("ownership: cannot borrow `{name}` after move"));
            }
            Some(Place::Owned) => {
                if self.mut_borrows.get(name).copied().unwrap_or(0) > 0 {
                    self.err(format!(
                        "ownership: cannot shared-borrow `{name}` while `&mut` is active"
                    ));
                }
                *self.shared_borrows.entry(name.to_string()).or_insert(0) += 1;
            }
            None => {}
        }
    }

    fn borrow_mut(&mut self, name: &str) {
        match self.places.get(name).copied() {
            Some(Place::Moved) => {
                self.err(format!(
                    "ownership: cannot `&mut` borrow `{name}` after move"
                ));
            }
            Some(Place::Owned) => {
                if self.shared_borrows.get(name).copied().unwrap_or(0) > 0 {
                    self.err(format!(
                        "ownership: cannot `&mut` borrow `{name}` while shared borrows exist"
                    ));
                }
                if self.mut_borrows.get(name).copied().unwrap_or(0) > 0 {
                    self.err(format!(
                        "ownership: cannot `&mut` borrow `{name}` twice"
                    ));
                }
                *self.mut_borrows.entry(name.to_string()).or_insert(0) += 1;
            }
            None => {}
        }
    }

    fn snapshot(&self) -> (HashMap<String, Place>, HashMap<String, usize>, HashMap<String, usize>) {
        (
            self.places.clone(),
            self.mut_borrows.clone(),
            self.shared_borrows.clone(),
        )
    }

    fn restore_full(
        &mut self,
        snap: &(HashMap<String, Place>, HashMap<String, usize>, HashMap<String, usize>),
    ) {
        self.places = snap.0.clone();
        self.mut_borrows = snap.1.clone();
        self.shared_borrows = snap.2.clone();
    }

    fn restore_moved_only(
        &mut self,
        snap: &(HashMap<String, Place>, HashMap<String, usize>, HashMap<String, usize>),
    ) {
        for (k, v) in &snap.0 {
            if !matches!(self.places.get(k), Some(Place::Moved)) {
                self.places.insert(k.clone(), *v);
            }
        }
        self.mut_borrows = snap.1.clone();
        self.shared_borrows = snap.2.clone();
    }

    fn merge_places(
        &mut self,
        other: &(HashMap<String, Place>, HashMap<String, usize>, HashMap<String, usize>),
    ) {
        for (k, v) in &other.0 {
            match (self.places.get(k).copied(), *v) {
                (Some(Place::Moved), _) | (_, Place::Moved) => {
                    self.places.insert(k.clone(), Place::Moved);
                }
                (None, Place::Owned) => {
                    self.places.insert(k.clone(), Place::Owned);
                }
                _ => {}
            }
        }
        self.mut_borrows.clear();
        self.shared_borrows.clear();
    }
}

fn is_owned_type(t: Option<&KabType>) -> bool {
    matches!(t, Some(KabType::Named(n)) if n == "Owned")
}

fn is_ref_owned_type(t: Option<&KabType>) -> bool {
    match t {
        Some(KabType::Ref(inner) | KabType::RefMut(inner)) => {
            matches!(inner.as_ref(), KabType::Named(n) if n == "Owned")
        }
        _ => false,
    }
}

fn call_name(func: &Expr) -> Option<String> {
    match func {
        Expr::Variable(n) => Some(n.clone()),
        Expr::Member(_, field, _) => Some(field.clone()),
        _ => None,
    }
}

fn is_peek_api(name: Option<&str>) -> bool {
    matches!(
        name,
        Some("owned_read" | "owned_write" | "read" | "write")
    )
}

fn is_move_api(name: Option<&str>) -> bool {
    matches!(name, Some("owned_move" | "move" | "drop" | "free"))
}

fn is_alloc_api(name: Option<&str>) -> bool {
    matches!(name, Some("owned_alloc" | "alloc"))
}

fn arg_is_borrow(arg: &CallArg) -> bool {
    let e = match arg {
        CallArg::Expr(e) | CallArg::Spread(e) => e,
    };
    matches!(e, Expr::Unary(UnaryOp::Ref | UnaryOp::RefMut, _))
}

fn arg_is_owned_var(arg: &CallArg, places: &HashMap<String, Place>) -> bool {
    let e = match arg {
        CallArg::Expr(e) | CallArg::Spread(e) => e,
    };
    match e {
        Expr::Variable(n) => matches!(places.get(n), Some(Place::Owned)),
        _ => false,
    }
}

/// Run ownership check when `mode` is Manual. No-op for GC.
pub fn check_ownership(stmts: &[Stmt], mode: MemoryMode) -> Result<(), String> {
    if mode != MemoryMode::Manual {
        return Ok(());
    }
    let mut c = Checker::default();
    c.check_program(stmts);
    if c.errors.is_empty() {
        Ok(())
    } else {
        Err(c.errors.join("\n"))
    }
}
