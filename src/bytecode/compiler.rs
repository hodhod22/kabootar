//! AST → bytecode compiler (subset) with graceful fallback.

use crate::ast::{
    exported_binding_names, ArrayPiece, AssignTarget, BinaryOp, BindingPattern, CallArg,
    ClassField, ClassMethod, Expr, Literal, MatchArm, ObjectBind, ObjectPiece, Pattern,
    PatternField, PatternPiece, Stmt, UnaryOp,
};
use super::types::{
    BytecodeClassDef, BytecodeClassField, BytecodeEnumDef, BytecodeEnumVariantDef, BytecodeFnDef,
    BytecodeInterfaceDef, BytecodeInterfaceMethod, BytecodeModule, Constant, GeneratorTryRegion,
    Opcode,
};
use std::collections::HashMap;

pub struct CompileError;

struct IteratorCloseCtx {
    src_local: String,
    async_for: bool,
}

impl Clone for IteratorCloseCtx {
    fn clone(&self) -> Self {
        Self {
            src_local: self.src_local.clone(),
            async_for: self.async_for,
        }
    }
}

struct LoopFrame {
    break_patches: Vec<usize>,
    continue_target: usize,
    iterator_close: Option<IteratorCloseCtx>,
}

struct Compiler {
    constants: Vec<Constant>,
    globals: Vec<String>,
    locals: Vec<String>,
    immutable_locals: Vec<bool>,
    /// Locals from the enclosing function/block (for closure capture).
    enclosing_locals: Vec<String>,
    enclosing_immutable: Vec<bool>,
    code: Vec<Opcode>,
    arrow_functions: Vec<BytecodeFnDef>,
    class_names: HashMap<String, u16>,
    for_loop_counter: usize,
    assign_tmp_counter: usize,
    loop_stack: Vec<LoopFrame>,
    exports: Vec<String>,
    in_generator: bool,
    in_async_fn: bool,
    try_regions: Vec<GeneratorTryRegion>,
}

impl Compiler {
    fn new() -> Self {
        Self {
            constants: Vec::new(),
            locals: Vec::new(),
            globals: Vec::new(),
            immutable_locals: Vec::new(),
            enclosing_locals: Vec::new(),
            enclosing_immutable: Vec::new(),
            code: Vec::new(),
            arrow_functions: Vec::new(),
            class_names: HashMap::new(),
            for_loop_counter: 0,
            assign_tmp_counter: 0,
            loop_stack: Vec::new(),
            exports: Vec::new(),
            in_generator: false,
            in_async_fn: false,
            try_regions: Vec::new(),
        }
    }

    fn const_index(&mut self, c: Constant) -> u16 {
        if let Some(i) = self.constants.iter().position(|x| x == &c) {
            return i as u16;
        }
        let i = self.constants.len();
        self.constants.push(c);
        i as u16
    }

    fn global_index(&mut self, name: &str) -> u16 {
        if let Some(i) = self.globals.iter().position(|g| g == name) {
            return i as u16;
        }
        let i = self.globals.len();
        self.globals.push(name.to_string());
        i as u16
    }

    fn local_index(&mut self, name: &str) -> u16 {
        if let Some(i) = self.locals.iter().position(|l| l == name) {
            return i as u16;
        }
        let i = self.locals.len();
        self.locals.push(name.to_string());
        self.immutable_locals.push(false);
        i as u16
    }

    fn mark_local_immutable(&mut self, name: &str) {
        if let Some(i) = self.locals.iter().position(|l| l == name) {
            if i >= self.immutable_locals.len() {
                self.immutable_locals.resize(i + 1, false);
            }
            self.immutable_locals[i] = true;
        }
    }

    fn is_enclosed_local(&self, name: &str) -> bool {
        self.enclosing_locals.iter().any(|l| l == name)
    }

    fn capture_local_index(&mut self, name: &str) -> u16 {
        if let Some(i) = self.locals.iter().position(|l| l == name) {
            return i as u16;
        }
        let i = self.locals.len();
        self.locals.push(name.to_string());
        let immutable = self
            .enclosing_locals
            .iter()
            .position(|l| l == name)
            .and_then(|ei| self.enclosing_immutable.get(ei).copied())
            .unwrap_or(false);
        self.immutable_locals.push(immutable);
        i as u16
    }

    fn emit(&mut self, op: Opcode) {
        self.code.push(op);
    }

    fn emit_store_name(&mut self, name: &str) {
        if self.locals.iter().any(|l| l == name) {
            let idx = self.local_index(name);
            self.emit(Opcode::StoreLocal(idx));
        } else if self.is_enclosed_local(name) {
            let idx = self.capture_local_index(name);
            self.emit(Opcode::StoreLocal(idx));
        } else {
            let idx = self.global_index(name);
            self.emit(Opcode::StoreGlobal(idx));
        }
    }

    fn emit_load_name(&mut self, name: &str) {
        if self.locals.iter().any(|l| l == name) {
            let idx = self.local_index(name);
            self.emit(Opcode::LoadLocal(idx));
        } else if self.is_enclosed_local(name) {
            let idx = self.capture_local_index(name);
            self.emit(Opcode::LoadLocal(idx));
        } else {
            let idx = self.global_index(name);
            self.emit(Opcode::LoadGlobal(idx));
        }
    }

    fn flush_array_items(
        &mut self,
        item_exprs: &mut Vec<&Expr>,
        have_array: &mut bool,
    ) -> Result<(), CompileError> {
        if item_exprs.is_empty() {
            return Ok(());
        }
        for e in item_exprs.iter() {
            self.compile_expr(e)?;
        }
        if item_exprs.len() > u8::MAX as usize {
            return Err(CompileError);
        }
        self.emit(Opcode::MakeArray(item_exprs.len() as u8));
        item_exprs.clear();
        if *have_array {
            self.emit(Opcode::ConcatArray);
        } else {
            *have_array = true;
        }
        Ok(())
    }

    fn compile_array_pieces(&mut self, pieces: &[ArrayPiece]) -> Result<(), CompileError> {
        let mut have_array = false;
        let mut item_exprs: Vec<&Expr> = Vec::new();

        for piece in pieces {
            match piece {
                ArrayPiece::Item(e) => item_exprs.push(e),
                ArrayPiece::Spread(e) => {
                    self.flush_array_items(&mut item_exprs, &mut have_array)?;
                    self.compile_expr(e)?;
                    if have_array {
                        self.emit(Opcode::ConcatArray);
                    } else {
                        have_array = true;
                    }
                }
            }
        }
        self.flush_array_items(&mut item_exprs, &mut have_array)?;
        if !have_array {
            self.emit(Opcode::MakeArray(0));
        }
        Ok(())
    }

    fn flush_object_fields(
        &mut self,
        field_exprs: &mut Vec<(&String, &Expr)>,
        have_object: &mut bool,
    ) -> Result<(), CompileError> {
        if field_exprs.is_empty() {
            return Ok(());
        }
        for (key, value) in field_exprs.iter() {
            self.compile_expr(value)?;
            let idx = self.const_index(Constant::String((*key).clone()));
            self.emit(Opcode::Const(idx));
        }
        if field_exprs.len() > u8::MAX as usize {
            return Err(CompileError);
        }
        self.emit(Opcode::MakeObject(field_exprs.len() as u8));
        field_exprs.clear();
        if *have_object {
            self.emit(Opcode::MergeObject);
        } else {
            *have_object = true;
        }
        Ok(())
    }

    fn compile_object_pieces(&mut self, pieces: &[ObjectPiece]) -> Result<(), CompileError> {
        let mut have_object = false;
        let mut field_exprs: Vec<(&String, &Expr)> = Vec::new();

        for piece in pieces {
            match piece {
                ObjectPiece::Field { key, value } => field_exprs.push((key, value)),
                ObjectPiece::Method {
                    key,
                    params,
                    rest,
                    body,
                    async_fn,
                } => {
                    self.flush_object_fields(&mut field_exprs, &mut have_object)?;
                    if *async_fn || crate::ast::fn_has_defaults_or_rest(params, rest) {
                        return Err(CompileError);
                    }
                    let names = crate::ast::fn_param_names(params);
                    let idx = self.compile_arrow(&names, body, *async_fn, false)?;
                    self.emit(Opcode::MakeArrowFn(idx));
                    let key_idx = self.const_index(Constant::String(key.clone()));
                    self.emit(Opcode::Const(key_idx));
                    self.emit(Opcode::MakeObject(1));
                    if have_object {
                        self.emit(Opcode::MergeObject);
                    } else {
                        have_object = true;
                    }
                }
                ObjectPiece::Spread(e) => {
                    self.flush_object_fields(&mut field_exprs, &mut have_object)?;
                    self.compile_expr(e)?;
                    if have_object {
                        self.emit(Opcode::MergeObject);
                    } else {
                        have_object = true;
                    }
                }
            }
        }
        self.flush_object_fields(&mut field_exprs, &mut have_object)?;
        if !have_object {
            self.emit(Opcode::MakeObject(0));
        }
        Ok(())
    }

    fn compile_call_args_array(&mut self, args: &[CallArg]) -> Result<(), CompileError> {
        let mut pieces: Vec<ArrayPiece> = Vec::new();
        for arg in args {
            match arg {
                CallArg::Expr(e) => pieces.push(ArrayPiece::Item(e.clone())),
                CallArg::Spread(e) => pieces.push(ArrayPiece::Spread(e.clone())),
            }
        }
        self.compile_array_pieces(&pieces)
    }

    fn store_binding_name(&mut self, name: &str, immutable: bool) {
        if immutable {
            self.mark_local_immutable(name);
        }
        self.emit_store_name(name);
    }

    fn compile_bind_pattern(&mut self, pattern: &BindingPattern, immutable: bool) -> Result<(), CompileError> {
        match pattern {
            BindingPattern::Name(name) => {
                self.store_binding_name(name, immutable);
                Ok(())
            }
            BindingPattern::Wildcard => {
                self.emit(Opcode::Pop);
                Ok(())
            }
            BindingPattern::Rest(_) => Err(CompileError),
            BindingPattern::Array(items) => {
                let rest_at = items.iter().position(|i| matches!(i, BindingPattern::Rest(_)));
                match rest_at {
                    None => {
                        let mut idx = 0u8;
                        for item in items {
                            match item {
                                BindingPattern::Rest(name) => {
                                    self.emit(Opcode::Dup);
                                    self.emit(Opcode::ArraySliceFrom(idx));
                                    if name.is_empty() {
                                        self.emit(Opcode::Pop);
                                    } else {
                                        self.store_binding_name(name, immutable);
                                    }
                                    break;
                                }
                                other => {
                                    self.emit(Opcode::Dup);
                                    let c = self.const_index(Constant::Number(idx as i64));
                                    self.emit(Opcode::Const(c));
                                    self.emit(Opcode::IndexGet);
                                    self.compile_bind_pattern(other, immutable)?;
                                    idx = idx.saturating_add(1);
                                }
                            }
                        }
                    }
                    Some(rest_idx) => {
                        let fixed_before = &items[..rest_idx];
                        let fixed_after = &items[rest_idx + 1..];
                        if fixed_before.len() + fixed_after.len() > u8::MAX as usize {
                            return Err(CompileError);
                        }
                        for (i, item) in fixed_before.iter().enumerate() {
                            self.emit(Opcode::Dup);
                            let c = self.const_index(Constant::Number(i as i64));
                            self.emit(Opcode::Const(c));
                            self.emit(Opcode::IndexGet);
                            self.compile_bind_pattern(item, immutable)?;
                        }
                        if let BindingPattern::Rest(name) = &items[rest_idx] {
                            self.emit(Opcode::Dup);
                            self.emit(Opcode::ArraySliceRest(
                                rest_idx as u8,
                                fixed_after.len() as u8,
                            ));
                            if name.is_empty() {
                                self.emit(Opcode::Pop);
                            } else {
                                self.store_binding_name(name, immutable);
                            }
                        }
                        for (j, item) in fixed_after.iter().enumerate() {
                            let from_end = (fixed_after.len() - j) as u8;
                            if from_end == 0 {
                                return Err(CompileError);
                            }
                            self.emit(Opcode::Dup);
                            self.emit(Opcode::IndexPeekFromEnd(from_end));
                            self.compile_bind_pattern(item, immutable)?;
                        }
                    }
                }
                self.emit(Opcode::Pop);
                Ok(())
            }
            BindingPattern::Object(fields) => {
                let mut bound_keys: Vec<String> = Vec::new();
                for field in fields {
                    match field {
                        ObjectBind::Shorthand(key) => {
                            self.emit(Opcode::Dup);
                            let idx = self.const_index(Constant::String(key.clone()));
                            self.emit(Opcode::GetMember(idx));
                            self.store_binding_name(key, immutable);
                            bound_keys.push(key.clone());
                        }
                        ObjectBind::Field { key, pattern } => {
                            self.emit(Opcode::Dup);
                            let idx = self.const_index(Constant::String(key.clone()));
                            self.emit(Opcode::GetMember(idx));
                            self.compile_bind_pattern(pattern, immutable)?;
                            bound_keys.push(key.clone());
                        }
                        ObjectBind::Rest(name) => {
                            if name.is_empty() {
                                continue;
                            }
                            self.emit(Opcode::Dup);
                            if bound_keys.is_empty() {
                                self.store_binding_name(name, immutable);
                            } else {
                                if bound_keys.len() > u8::MAX as usize {
                                    return Err(CompileError);
                                }
                                for key in &bound_keys {
                                    let ki = self.const_index(Constant::String(key.clone()));
                                    self.emit(Opcode::Const(ki));
                                }
                                self.emit(Opcode::ObjectRest(bound_keys.len() as u8));
                                self.store_binding_name(name, immutable);
                            }
                        }
                    }
                }
                self.emit(Opcode::Pop);
                Ok(())
            }
        }
    }

    fn compile_callable(&mut self, func: &Expr) -> Result<(), CompileError> {
        match func {
            Expr::Variable(name) => {
                self.emit_load_name(name);
                Ok(())
            }
            Expr::Member(obj, method) => {
                if matches!(obj.as_ref(), Expr::Super) {
                    let idx = self.const_index(Constant::String(method.clone()));
                    self.emit(Opcode::GetSuperMethod(idx));
                } else {
                    self.compile_expr(obj)?;
                    let idx = self.const_index(Constant::String(method.clone()));
                    self.emit(Opcode::GetMember(idx));
                }
                Ok(())
            }
            other => self.compile_expr(other),
        }
    }

    fn compile_literal(&mut self, lit: &Literal) -> Result<(), CompileError> {
        match lit {
            Literal::Number(n) => {
                let idx = self.const_index(Constant::Number(*n));
                self.emit(Opcode::Const(idx));
                Ok(())
            }
            Literal::BigInt(digits) => {
                let idx = self.const_index(Constant::BigInt(digits.clone()));
                self.emit(Opcode::Const(idx));
                Ok(())
            }
            Literal::Float(f) => {
                let idx = self.const_index(Constant::Float(*f));
                self.emit(Opcode::Const(idx));
                Ok(())
            }
            Literal::String(s) => {
                let idx = self.const_index(Constant::String(s.clone()));
                self.emit(Opcode::Const(idx));
                Ok(())
            }
            Literal::Bool(b) => {
                let idx = self.const_index(Constant::Bool(*b));
                self.emit(Opcode::Const(idx));
                Ok(())
            }
            Literal::Null => {
                let idx = self.const_index(Constant::Null);
                self.emit(Opcode::Const(idx));
                Ok(())
            }
            Literal::Undefined => {
                let idx = self.const_index(Constant::Undefined);
                self.emit(Opcode::Const(idx));
                Ok(())
            }
            Literal::Nan => {
                let idx = self.const_index(Constant::Nan);
                self.emit(Opcode::Const(idx));
                Ok(())
            }
            Literal::Array(items) => self.compile_array_pieces(items),
            Literal::Object(pieces) => self.compile_object_pieces(pieces),
            Literal::Ok(inner) => {
                self.compile_expr(inner)?;
                self.emit(Opcode::MakeOk);
                Ok(())
            }
            Literal::Err(inner) => {
                self.compile_expr(inner)?;
                self.emit(Opcode::MakeErr);
                Ok(())
            }
            Literal::Some(inner) => {
                self.compile_expr(inner)?;
                self.emit(Opcode::MakeSome);
                Ok(())
            }
            Literal::None => {
                self.emit(Opcode::MakeNone);
                Ok(())
            }
        }
    }

    fn compile_expr(&mut self, expr: &Expr) -> Result<(), CompileError> {
        match expr {
            Expr::Literal(lit) => self.compile_literal(lit),
            Expr::Variable(name) => {
                self.emit_load_name(name);
                Ok(())
            }
            Expr::Unary(op, inner) => {
                if matches!(op, UnaryOp::Delete) {
                    return Err(CompileError);
                }
                if matches!(op, UnaryOp::Throw | UnaryOp::Raise) {
                    self.emit_active_iterator_closes();
                    self.compile_expr(inner)?;
                    self.emit(Opcode::Throw);
                    return Ok(());
                }
                self.compile_expr(inner)?;
                match op {
                    UnaryOp::Not => self.emit(Opcode::Not),
                    UnaryOp::Neg => self.emit(Opcode::Neg),
                    UnaryOp::BitNot => self.emit(Opcode::BitNot),
                    UnaryOp::Delete | UnaryOp::Throw | UnaryOp::Raise => unreachable!(),
                }
                Ok(())
            }
            Expr::Binary(left, op, right) => {
                if matches!(op, BinaryOp::NullishCoalesce) {
                    self.compile_expr(left)?;
                    self.emit(Opcode::Dup);
                    let jump_keep = self.code.len();
                    self.emit(Opcode::JumpIfNotNullish(0));
                    self.emit(Opcode::Pop);
                    self.emit(Opcode::Pop);
                    self.compile_expr(right)?;
                    let jump_end = self.code.len();
                    self.emit(Opcode::Jump(0));
                    let keep_start = self.code.len();
                    patch_jump(&mut self.code, jump_keep, keep_start);
                    self.emit(Opcode::Pop);
                    let end = self.code.len();
                    patch_jump(&mut self.code, jump_end, end);
                    return Ok(());
                }
                if matches!(op, BinaryOp::And) {
                    self.compile_expr(left)?;
                    let jump_end = self.code.len();
                    self.emit(Opcode::JumpIfFalse(0));
                    self.compile_expr(right)?;
                    let end = self.code.len();
                    patch_jump(&mut self.code, jump_end, end);
                    return Ok(());
                }
                if matches!(op, BinaryOp::Or) {
                    self.compile_expr(left)?;
                    self.emit(Opcode::Dup);
                    let jump_end = self.code.len();
                    self.emit(Opcode::JumpIfFalse(0));
                    self.emit(Opcode::Pop);
                    self.compile_expr(right)?;
                    let end = self.code.len();
                    patch_jump(&mut self.code, jump_end, end);
                    return Ok(());
                }
                if matches!(op, BinaryOp::Is | BinaryOp::IsNot) {
                    return Err(CompileError);
                }
                self.compile_expr(left)?;
                self.compile_expr(right)?;
                self.emit(match op {
                    BinaryOp::Add => Opcode::Add,
                    BinaryOp::Sub => Opcode::Sub,
                    BinaryOp::Mul => Opcode::Mul,
                    BinaryOp::Div => Opcode::Div,
                    BinaryOp::Mod => Opcode::Mod,
                    BinaryOp::Pow => Opcode::Pow,
                    BinaryOp::Eq => Opcode::Eq,
                    BinaryOp::Ne => Opcode::Ne,
                    BinaryOp::Lt => Opcode::Lt,
                    BinaryOp::Le => Opcode::Le,
                    BinaryOp::Gt => Opcode::Gt,
                    BinaryOp::Ge => Opcode::Ge,
                    BinaryOp::And => Opcode::And,
                    BinaryOp::Or => Opcode::Or,
                    BinaryOp::In => Opcode::In,
                    BinaryOp::Is | BinaryOp::IsNot => Opcode::In,
                    BinaryOp::BitAnd => Opcode::BitAnd,
                    BinaryOp::BitOr => Opcode::BitOr,
                    BinaryOp::BitXor => Opcode::BitXor,
                    BinaryOp::Shl => Opcode::Shl,
                    BinaryOp::Shr => Opcode::Shr,
                    BinaryOp::Ushr => Opcode::Ushr,
                    BinaryOp::NullishCoalesce => Opcode::Add,
                });
                Ok(())
            }
            Expr::Ternary(cond, then_b, else_b) => {
                self.compile_expr(cond)?;
                let jump_false = self.code.len();
                self.emit(Opcode::JumpIfFalse(0));
                self.compile_expr(then_b)?;
                let jump_end = self.code.len();
                self.emit(Opcode::Jump(0));
                let else_start = self.code.len();
                patch_jump(&mut self.code, jump_false, else_start);
                self.compile_expr(else_b)?;
                let end = self.code.len();
                patch_jump(&mut self.code, jump_end, end);
                Ok(())
            }
            Expr::If(cond, then_b, else_b) => {
                self.compile_expr(cond)?;
                let jump_false = self.code.len();
                self.emit(Opcode::JumpIfFalse(0));
                self.compile_block_or_expr(then_b, false)?;
                let jump_end = self.code.len();
                self.emit(Opcode::Jump(0));
                let else_start = self.code.len();
                patch_jump(&mut self.code, jump_false, else_start);
                if let Some(e) = else_b {
                    self.compile_block_or_expr(e, false)?;
                }
                let end = self.code.len();
                patch_jump(&mut self.code, jump_end, end);
                let null_idx = self.const_index(Constant::Null);
                self.emit(Opcode::Const(null_idx));
                Ok(())
            }
            Expr::IfLet {
                pattern,
                scrutinee,
                body,
                else_branch,
            } => {
                let mut arms = vec![MatchArm {
                    pattern: pattern.clone(),
                    guard: None,
                    body: body.as_ref().clone(),
                }];
                if let Some(else_b) = else_branch {
                    arms.push(MatchArm {
                        pattern: Pattern::Wildcard,
                        guard: None,
                        body: else_b.as_ref().clone(),
                    });
                } else {
                    arms.push(MatchArm {
                        pattern: Pattern::Wildcard,
                        guard: None,
                        body: Expr::Literal(Literal::Null),
                    });
                }
                self.compile_expr(&Expr::Match(scrutinee.clone(), arms))
            }
            Expr::WhileLet { .. } => Err(CompileError),
            Expr::Call(func, args) => {
                let has_spread = args.iter().any(|a| matches!(a, CallArg::Spread(_)));
                if has_spread {
                    self.compile_call_args_array(args)?;
                    if let Expr::Member(obj, method) = func.as_ref() {
                        if matches!(obj.as_ref(), Expr::Super) {
                            let idx = self.const_index(Constant::String(method.clone()));
                            self.emit(Opcode::GetSuperMethod(idx));
                            self.emit(Opcode::CallFromArray);
                            return Ok(());
                        }
                    }
                    if let Expr::Variable(name) = func.as_ref() {
                        if let Some(&class_idx) = self.class_names.get(name) {
                            self.emit(Opcode::NewInstanceFromArray(class_idx));
                            return Ok(());
                        }
                    }
                    self.compile_callable(func)?;
                    self.emit(Opcode::Swap);
                    self.emit(Opcode::CallFromArray);
                    return Ok(());
                }
                for arg in args {
                    match arg {
                        CallArg::Expr(e) => self.compile_expr(e)?,
                        CallArg::Spread(_) => return Err(CompileError),
                    }
                }
                if let Expr::Member(obj, method) = func.as_ref() {
                    if matches!(obj.as_ref(), Expr::Super) {
                        let idx = self.const_index(Constant::String(method.clone()));
                        self.emit(Opcode::GetSuperMethod(idx));
                        self.emit(Opcode::Call(args.len() as u8));
                        return Ok(());
                    }
                }
                if let Expr::Variable(name) = func.as_ref() {
                    if let Some(&class_idx) = self.class_names.get(name) {
                        self.emit(Opcode::NewInstance(class_idx, args.len() as u8));
                        return Ok(());
                    }
                }
                self.compile_callable(func)?;
                self.emit(Opcode::Call(args.len() as u8));
                if let Some(var) = crate::runtime::stdlib::object::mutator_writeback_var(func, args) {
                    let idx = self.local_index(&var);
                    self.emit(Opcode::Dup);
                    self.emit(Opcode::StoreLocal(idx));
                }
                Ok(())
            }
            Expr::Index(container, idx) => {
                self.compile_expr(container)?;
                self.compile_expr(idx)?;
                self.emit(Opcode::IndexGet);
                Ok(())
            }
            Expr::Member(obj, field) => {
                if matches!(obj.as_ref(), Expr::Super) {
                    let idx = self.const_index(Constant::String(field.clone()));
                    self.emit(Opcode::GetSuperMethod(idx));
                    Ok(())
                } else {
                    self.compile_expr(obj)?;
                    if field == "length" {
                        self.emit(Opcode::GetLength);
                    } else {
                        let idx = self.const_index(Constant::String(field.clone()));
                        self.emit(Opcode::GetMember(idx));
                    }
                    Ok(())
                }
            }
            Expr::OptionalMember(obj, field) => {
                self.compile_expr(obj)?;
                let idx = self.const_index(Constant::String(field.clone()));
                self.emit(Opcode::Const(idx));
                self.emit_load_name("__opt_member");
                self.emit(Opcode::Call(2));
                Ok(())
            }
            Expr::OptionalIndex(obj, idx) => {
                self.compile_expr(obj)?;
                self.compile_expr(idx)?;
                self.emit_load_name("__opt_index");
                self.emit(Opcode::Call(2));
                Ok(())
            }
            Expr::OptionalCall(func, args) => {
                self.compile_call_args_array(args)?;
                self.compile_expr(func)?;
                self.emit_load_name("__opt_call");
                self.emit(Opcode::Call((args.len() + 1) as u8));
                Ok(())
            }
            Expr::Assign(target, value) => self.compile_assign(target, value),
            Expr::While(cond, body) => self.compile_while(cond, body),
            Expr::DoWhile(body, cond) => self.compile_do_while(body, cond),
            Expr::ForEach(loop_) => self.compile_foreach(loop_),
            Expr::Switch {
                scrutinee,
                cases,
                default_body,
            } => self.compile_switch(scrutinee, cases, default_body.as_deref()),
            Expr::ForClassic {
                init,
                cond,
                step,
                body,
            } => self.compile_for_classic(init, cond, step, body),
            Expr::TryCatch {
                body,
                err_name,
                handler,
                finally,
            } => {
                if finally.is_some() {
                    return Err(CompileError);
                }
                let body_start = self.code.len();
                self.compile_block_or_expr(body, true)?;
                let body_end = self.code.len();
                let jump_catch = self.code.len();
                self.emit(Opcode::JumpIfResultErr(0));
                let jump_end = self.code.len();
                self.emit(Opcode::Jump(0));
                let catch_start = self.code.len();
                patch_jump(&mut self.code, jump_catch, catch_start);
                let err_local = self.local_index(err_name);
                self.try_regions.push(GeneratorTryRegion {
                    body_start,
                    body_end,
                    catch_start,
                    err_local,
                });
                self.store_binding_name(err_name, false);
                self.compile_block_or_expr(handler, true)?;
                let end = self.code.len();
                patch_jump(&mut self.code, jump_end, end);
                Ok(())
            }
            Expr::Break => {
                let patch = self.code.len();
                self.emit(Opcode::Jump(0));
                self.loop_stack
                    .last_mut()
                    .ok_or(CompileError)?
                    .break_patches
                    .push(patch);
                Ok(())
            }
            Expr::Continue => {
                let target = self.loop_stack.last().ok_or(CompileError)?.continue_target;
                let offset = (target as i32) - (self.code.len() as i32) - 1;
                self.emit(Opcode::Jump(offset));
                Ok(())
            }
            Expr::Fallthrough => Ok(()),
            Expr::Pass => {
                let idx = self.const_index(Constant::Null);
                self.emit(Opcode::Const(idx));
                Ok(())
            }
            Expr::ImportMeta => {
                self.emit_load_name("import_meta");
                self.emit(Opcode::Call(0));
                Ok(())
            }
            Expr::DynamicImport(spec) => {
                self.compile_expr(spec)?;
                self.emit_load_name("dynamic_import");
                self.emit(Opcode::Call(1));
                Ok(())
            }
            Expr::Assert { .. } | Expr::With { .. } => {
                Err(CompileError)
            }
            Expr::Arrow {
                params,
                rest,
                body,
                async_fn,
                generator_fn,
            } => {
                if crate::ast::fn_has_defaults_or_rest(params, rest) {
                    return Err(CompileError);
                }
                let names = crate::ast::fn_param_names(params);
                let idx = self.compile_arrow(&names, body, *async_fn, *generator_fn)?;
                self.emit(Opcode::MakeArrowFn(idx));
                Ok(())
            }
            Expr::Function {
                name,
                params,
                rest,
                body,
                public,
                async_fn,
                generator_fn,
            } => {
                if *public || crate::ast::fn_has_defaults_or_rest(params, rest) {
                    return Err(CompileError);
                }
                let names = crate::ast::fn_param_names(params);
                let idx = self.compile_arrow(&names, body, *async_fn, *generator_fn)?;
                self.emit(Opcode::MakeArrowFn(idx));
                self.emit(Opcode::Dup);
                self.emit_store_name(name);
                Ok(())
            }
            Expr::Await(inner) => {
                self.compile_expr(inner)?;
                self.emit(Opcode::Await);
                Ok(())
            }
            Expr::Yield(inner) => {
                if !self.in_generator {
                    return Err(CompileError);
                }
                self.compile_expr(inner)?;
                self.emit(Opcode::Yield);
                Ok(())
            }
            Expr::YieldStar(inner) => {
                if !self.in_generator {
                    return Err(CompileError);
                }
                self.compile_expr(inner)?;
                self.emit(Opcode::YieldStar);
                Ok(())
            }
            Expr::This => {
                self.emit_load_name("self");
                Ok(())
            }
            Expr::Match(value, arms) => self.compile_match(value, arms),
            Expr::ResultQuestion(inner) => {
                self.compile_expr(inner)?;
                self.emit(Opcode::ResultQuestion);
                Ok(())
            }
            Expr::Block(stmts) => {
                if stmts.is_empty() {
                    let idx = self.const_index(Constant::Null);
                    self.emit(Opcode::Const(idx));
                    return Ok(());
                }
                for (i, stmt) in stmts.iter().enumerate() {
                    self.compile_stmt(stmt, i + 1 == stmts.len())?;
                }
                Ok(())
            }
            _ => Err(CompileError),
        }
    }

    fn compile_class(
        &mut self,
        name: &str,
        extends: &Option<String>,
        implements: &[String],
        fields: &[ClassField],
        methods: &[ClassMethod],
    ) -> Result<BytecodeClassDef, CompileError> {
        let mut class_constants = Vec::new();
        let mut bc_fields = Vec::new();
        for field in fields {
            let mut default_const = None;
            let mut default_globals = Vec::new();
            let mut default_code = Vec::new();
            if let Some(default) = &field.default {
                if let Ok(idx) = self.compile_class_default_const(default, &mut class_constants) {
                    default_const = Some(idx);
                } else {
                    let mut fc = Compiler::new();
                    fc.globals = self.globals.clone();
                    fc.constants = class_constants.clone();
                    fc.compile_expr(default)?;
                    class_constants = fc.constants;
                    default_globals = fc.globals;
                    default_code = fc.code;
                    if !matches!(default_code.last(), Some(Opcode::Return)) {
                        default_code.push(Opcode::Return);
                    }
                }
            }
            bc_fields.push(BytecodeClassField {
                name: field.name.clone(),
                type_name: if field.type_name.is_empty() {
                    None
                } else {
                    Some(field.type_name.clone())
                },
                default_const,
                default_globals,
                default_code,
            });
        }

        let mut bc_methods = Vec::new();
        for method in methods {
            let mut method_compiler = Compiler::new();
            method_compiler.constants = self.constants.clone();
            method_compiler.globals = self.globals.clone();
            method_compiler.class_names = self.class_names.clone();
            method_compiler.compile_function_body(&method.params, &method.body, false)?;
            self.constants = method_compiler.constants.clone();
            self.globals = method_compiler.globals.clone();
            bc_methods.push(BytecodeFnDef {
                name: method.name.clone(),
                params: method.params.clone(),
                locals: method_compiler.locals,
                globals: method_compiler.globals,
                constants: method_compiler.constants,
                code: method_compiler.code,
                immutable_locals: method_compiler.immutable_locals,
                arrow_functions: method_compiler.arrow_functions,
                async_fn: false,
                generator_fn: false,
                try_regions: Vec::new(),
            });
        }

        Ok(BytecodeClassDef {
            name: name.to_string(),
            extends: extends.clone(),
            implements: implements.to_vec(),
            fields: bc_fields,
            constants: class_constants,
            methods: bc_methods,
        })
    }

    fn compile_class_default_const(
        &self,
        expr: &Expr,
        constants: &mut Vec<Constant>,
    ) -> Result<u16, CompileError> {
        let c = match expr {
            Expr::Literal(lit) => match lit {
                Literal::Number(n) => Constant::Number(*n),
                Literal::BigInt(digits) => Constant::BigInt(digits.clone()),
                Literal::Float(f) => Constant::Float(*f),
                Literal::String(s) => Constant::String(s.clone()),
                Literal::Bool(b) => Constant::Bool(*b),
                Literal::Null => Constant::Null,
                Literal::Undefined => Constant::Undefined,
                Literal::Nan => Constant::Nan,
                _ => return Err(CompileError),
            },
            _ => return Err(CompileError),
        };
        if let Some(i) = constants.iter().position(|x| x == &c) {
            return Ok(i as u16);
        }
        let i = constants.len();
        constants.push(c);
        Ok(i as u16)
    }

    fn compile_arrow(
        &mut self,
        params: &[String],
        body: &Expr,
        async_fn: bool,
        generator_fn: bool,
    ) -> Result<u16, CompileError> {
        let mut fn_compiler = Compiler::new();
        fn_compiler.constants = self.constants.clone();
        fn_compiler.globals = self.globals.clone();
        fn_compiler.enclosing_locals = self.locals.clone();
        fn_compiler.enclosing_immutable = self.immutable_locals.clone();
        fn_compiler.in_generator = generator_fn;
        fn_compiler.in_async_fn = async_fn;
        fn_compiler.compile_function_body(params, body, async_fn)?;
        self.constants = fn_compiler.constants.clone();
        self.globals = fn_compiler.globals.clone();
        let idx = self.arrow_functions.len();
        self.arrow_functions.push(BytecodeFnDef {
            name: format!("__arrow_{idx}"),
            params: params.to_vec(),
            locals: fn_compiler.locals,
            globals: fn_compiler.globals,
            constants: fn_compiler.constants,
            code: fn_compiler.code,
            immutable_locals: fn_compiler.immutable_locals,
            arrow_functions: fn_compiler.arrow_functions,
            async_fn,
            generator_fn,
            try_regions: fn_compiler.try_regions,
        });
        Ok(idx as u16)
    }

    fn compile_match(&mut self, value: &Expr, arms: &[MatchArm]) -> Result<(), CompileError> {
        if arms.is_empty() {
            return Err(CompileError);
        }
        self.compile_expr(value)?;
        let mut end_jumps: Vec<usize> = Vec::new();
        let mut arm_heads: Vec<usize> = Vec::new();
        let mut guard_fail_patches: Vec<(usize, usize)> = Vec::new();

        for (arm_idx, arm) in arms.iter().enumerate() {
            let arm_head = self.code.len();
            arm_heads.push(arm_head);
            self.emit(Opcode::Dup);
            let mut pattern_fails: Vec<usize> = Vec::new();
            self.compile_match_pattern(&arm.pattern, &mut pattern_fails)?;

            if let Some(guard) = &arm.guard {
                self.compile_expr(guard)?;
                let guard_fail = self.code.len();
                self.emit(Opcode::JumpIfFalse(0));
                guard_fail_patches.push((guard_fail, arm_idx + 1));
            }

            self.compile_block_or_expr(&arm.body, true)?;
            self.emit(Opcode::Swap);
            self.emit(Opcode::Pop);
            let end_jump = self.code.len();
            self.emit(Opcode::Jump(0));
            end_jumps.push(end_jump);

            let pattern_fail = self.code.len();
            for fail in pattern_fails {
                patch_jump(&mut self.code, fail, pattern_fail);
            }
            self.emit(Opcode::Pop);
        }

        for (patch_at, next_arm) in guard_fail_patches {
            if next_arm < arm_heads.len() {
                patch_jump(&mut self.code, patch_at, arm_heads[next_arm]);
            }
        }

        self.emit(Opcode::MatchFail);
        let end = self.code.len();
        for jump in end_jumps {
            patch_jump(&mut self.code, jump, end);
        }
        Ok(())
    }

    fn compile_match_const_pattern(
        &mut self,
        constant: Constant,
        pattern_fails: &mut Vec<usize>,
    ) -> Result<(), CompileError> {
        let idx = self.const_index(constant);
        let jump = self.code.len();
        self.emit(Opcode::JumpUnlessConstEq(idx, 0));
        pattern_fails.push(jump);
        self.emit(Opcode::Pop);
        Ok(())
    }

    fn compile_match_pattern(
        &mut self,
        pattern: &Pattern,
        pattern_fails: &mut Vec<usize>,
    ) -> Result<(), CompileError> {
        match pattern {
            Pattern::Wildcard => {
                self.emit(Opcode::Pop);
                Ok(())
            }
            Pattern::Number(n) => {
                self.compile_match_const_pattern(Constant::Number(*n), pattern_fails)
            }
            Pattern::Float(f) => {
                self.compile_match_const_pattern(Constant::Float(*f), pattern_fails)
            }
            Pattern::String(s) => {
                self.compile_match_const_pattern(Constant::String(s.clone()), pattern_fails)
            }
            Pattern::Bool(b) => {
                self.compile_match_const_pattern(Constant::Bool(*b), pattern_fails)
            }
            Pattern::Null => {
                self.compile_match_const_pattern(Constant::Null, pattern_fails)
            }
            Pattern::Undefined => {
                self.compile_match_const_pattern(Constant::Undefined, pattern_fails)
            }
            Pattern::Nan => {
                self.compile_match_const_pattern(Constant::Nan, pattern_fails)
            }
            Pattern::Variable(name) => {
                let idx = self.local_index(name);
                self.emit(Opcode::StoreLocal(idx));
                Ok(())
            }
            Pattern::Some(inner) => {
                let jump = self.code.len();
                self.emit(Opcode::JumpUnlessOptionSome(0));
                pattern_fails.push(jump);
                self.emit(Opcode::UnwrapOptionSome);
                self.compile_match_pattern(inner, pattern_fails)
            }
            Pattern::None => {
                let jump = self.code.len();
                self.emit(Opcode::JumpUnlessOptionNone(0));
                pattern_fails.push(jump);
                self.emit(Opcode::Pop);
                Ok(())
            }
            Pattern::Ok(inner) => {
                let jump = self.code.len();
                self.emit(Opcode::JumpUnlessResultOk(0));
                pattern_fails.push(jump);
                self.emit(Opcode::UnwrapResultOk);
                self.compile_match_pattern(inner, pattern_fails)
            }
            Pattern::Err(inner) => {
                let jump = self.code.len();
                self.emit(Opcode::JumpUnlessResultErr(0));
                pattern_fails.push(jump);
                self.emit(Opcode::UnwrapResultErr);
                self.compile_match_pattern(inner, pattern_fails)
            }
            Pattern::Array(pieces) => self.compile_match_array_pattern(pieces, pattern_fails),
            Pattern::Object(fields) => self.compile_match_object_pattern(fields, pattern_fails),
            Pattern::EnumVariant { .. } => Err(CompileError),
        }
    }

    fn emit_array_len_eq(
        &mut self,
        len: u8,
        pattern_fails: &mut Vec<usize>,
    ) -> Result<(), CompileError> {
        self.emit(Opcode::Dup);
        self.emit(Opcode::GetLength);
        let idx = self.const_index(Constant::Number(len as i64));
        self.emit(Opcode::Const(idx));
        self.emit(Opcode::Eq);
        let jump = self.code.len();
        self.emit(Opcode::JumpIfFalse(0));
        pattern_fails.push(jump);
        Ok(())
    }

    fn emit_array_len_min(
        &mut self,
        min: u8,
        pattern_fails: &mut Vec<usize>,
    ) -> Result<(), CompileError> {
        self.emit(Opcode::Dup);
        self.emit(Opcode::GetLength);
        let idx = self.const_index(Constant::Number(min as i64));
        self.emit(Opcode::Const(idx));
        self.emit(Opcode::Ge);
        let jump = self.code.len();
        self.emit(Opcode::JumpIfFalse(0));
        pattern_fails.push(jump);
        Ok(())
    }

    fn compile_match_array_piece(
        &mut self,
        piece: &PatternPiece,
        idx: u8,
        pattern_fails: &mut Vec<usize>,
    ) -> Result<(), CompileError> {
        match piece {
            PatternPiece::Item(pat) => {
                self.emit(Opcode::Dup);
                let c = self.const_index(Constant::Number(idx as i64));
                self.emit(Opcode::Const(c));
                self.emit(Opcode::IndexGet);
                self.compile_match_pattern(pat, pattern_fails)
            }
            PatternPiece::Wildcard => {
                self.emit(Opcode::Dup);
                let c = self.const_index(Constant::Number(idx as i64));
                self.emit(Opcode::Const(c));
                self.emit(Opcode::IndexGet);
                self.emit(Opcode::Pop);
                Ok(())
            }
            PatternPiece::Rest(_) => Err(CompileError),
        }
    }

    fn compile_match_array_piece_from_end(
        &mut self,
        piece: &PatternPiece,
        from_end: u8,
        pattern_fails: &mut Vec<usize>,
    ) -> Result<(), CompileError> {
        match piece {
            PatternPiece::Item(pat) => {
                self.emit(Opcode::Dup);
                self.emit(Opcode::IndexPeekFromEnd(from_end));
                self.compile_match_pattern(pat, pattern_fails)
            }
            PatternPiece::Wildcard => {
                self.emit(Opcode::Dup);
                self.emit(Opcode::IndexPeekFromEnd(from_end));
                self.emit(Opcode::Pop);
                Ok(())
            }
            PatternPiece::Rest(_) => Err(CompileError),
        }
    }

    fn compile_match_array_pattern(
        &mut self,
        pieces: &[PatternPiece],
        pattern_fails: &mut Vec<usize>,
    ) -> Result<(), CompileError> {
        let jump = self.code.len();
        self.emit(Opcode::JumpUnlessArray(0));
        pattern_fails.push(jump);

        let rest_at = pieces.iter().position(|p| matches!(p, PatternPiece::Rest(_)));
        match rest_at {
            None => {
                if pieces.len() > u8::MAX as usize {
                    return Err(CompileError);
                }
                self.emit_array_len_eq(pieces.len() as u8, pattern_fails)?;
                for (i, piece) in pieces.iter().enumerate() {
                    self.compile_match_array_piece(piece, i as u8, pattern_fails)?;
                }
            }
            Some(idx) => {
                let fixed_before = &pieces[..idx];
                let fixed_after = &pieces[idx + 1..];
                if fixed_before.len() + fixed_after.len() > u8::MAX as usize {
                    return Err(CompileError);
                }
                self.emit_array_len_min(
                    (fixed_before.len() + fixed_after.len()) as u8,
                    pattern_fails,
                )?;
                for (i, piece) in fixed_before.iter().enumerate() {
                    self.compile_match_array_piece(piece, i as u8, pattern_fails)?;
                }
                if let PatternPiece::Rest(name) = &pieces[idx] {
                    if !name.is_empty() {
                        self.emit(Opcode::Dup);
                        self.emit(Opcode::ArraySliceRest(
                            idx as u8,
                            fixed_after.len() as u8,
                        ));
                        let li = self.local_index(name);
                        self.emit(Opcode::StoreLocal(li));
                    }
                }
                for (j, piece) in fixed_after.iter().enumerate() {
                    let from_end = (fixed_after.len() - j) as u8;
                    if from_end == 0 {
                        return Err(CompileError);
                    }
                    self.compile_match_array_piece_from_end(piece, from_end, pattern_fails)?;
                }
            }
        }
        self.emit(Opcode::Pop);
        Ok(())
    }

    fn compile_match_object_pattern(
        &mut self,
        fields: &[PatternField],
        pattern_fails: &mut Vec<usize>,
    ) -> Result<(), CompileError> {
        let jump = self.code.len();
        self.emit(Opcode::JumpUnlessObject(0));
        pattern_fails.push(jump);

        if fields.is_empty() {
            let empty_jump = self.code.len();
            self.emit(Opcode::JumpUnlessObjectEmpty(0));
            pattern_fails.push(empty_jump);
            self.emit(Opcode::Pop);
            return Ok(());
        }

        let mut bound_keys: Vec<String> = Vec::new();
        for field in fields {
            match field {
                PatternField::Shorthand(key) => {
                    let ki = self.const_index(Constant::String(key.clone()));
                    let member_jump = self.code.len();
                    self.emit(Opcode::JumpUnlessHasMember(ki, 0));
                    pattern_fails.push(member_jump);
                    self.emit(Opcode::Dup);
                    self.emit(Opcode::GetMember(ki));
                    let li = self.local_index(key);
                    self.emit(Opcode::StoreLocal(li));
                    bound_keys.push(key.clone());
                }
                PatternField::Field { key, pattern } => {
                    let ki = self.const_index(Constant::String(key.clone()));
                    let member_jump = self.code.len();
                    self.emit(Opcode::JumpUnlessHasMember(ki, 0));
                    pattern_fails.push(member_jump);
                    self.emit(Opcode::Dup);
                    self.emit(Opcode::GetMember(ki));
                    self.compile_match_pattern(pattern, pattern_fails)?;
                    bound_keys.push(key.clone());
                }
                PatternField::Rest(name) => {
                    if name.is_empty() {
                        continue;
                    }
                    self.emit(Opcode::Dup);
                    if bound_keys.is_empty() {
                        let li = self.local_index(name);
                        self.emit(Opcode::StoreLocal(li));
                    } else {
                        if bound_keys.len() > u8::MAX as usize {
                            return Err(CompileError);
                        }
                        for key in &bound_keys {
                            let ki = self.const_index(Constant::String(key.clone()));
                            self.emit(Opcode::Const(ki));
                        }
                        self.emit(Opcode::ObjectRest(bound_keys.len() as u8));
                        let li = self.local_index(name);
                        self.emit(Opcode::StoreLocal(li));
                    }
                }
            }
        }
        self.emit(Opcode::Pop);
        Ok(())
    }

    fn fresh_assign_tmp(&mut self) -> String {
        let n = self.assign_tmp_counter;
        self.assign_tmp_counter += 1;
        format!("__kab_a{n}")
    }

    fn emit_load_lvalue_container(&mut self, obj: &Expr) -> Result<(), CompileError> {
        match obj {
            Expr::Variable(name) => {
                self.emit_load_name(name);
                Ok(())
            }
            Expr::This => {
                self.emit_load_name("self");
                Ok(())
            }
            Expr::Super => {
                self.emit_load_name("self");
                Ok(())
            }
            Expr::Member(inner, field) => {
                self.compile_expr(inner)?;
                let idx = self.const_index(Constant::String(field.clone()));
                self.emit(Opcode::GetMember(idx));
                Ok(())
            }
            Expr::Index(container, idx) => {
                self.compile_expr(container)?;
                self.compile_expr(idx)?;
                self.emit(Opcode::IndexGet);
                Ok(())
            }
            _ => Err(CompileError),
        }
    }

    fn emit_store_lvalue(&mut self, obj: &Expr) -> Result<(), CompileError> {
        match obj {
            Expr::Variable(name) => {
                self.emit_store_name(name);
                Ok(())
            }
            Expr::This => {
                self.emit_store_name("self");
                Ok(())
            }
            Expr::Super => {
                self.emit_store_name("self");
                Ok(())
            }
            Expr::Member(inner, field) => {
                let tmp = self.fresh_assign_tmp();
                let tmp_idx = self.local_index(&tmp);
                self.emit(Opcode::Swap);
                self.emit(Opcode::StoreLocal(tmp_idx));
                self.emit_load_lvalue_container(inner)?;
                self.emit(Opcode::Swap);
                let field_idx = self.const_index(Constant::String(field.clone()));
                self.emit(Opcode::MemberSet(field_idx));
                self.emit(Opcode::Pop);
                self.emit(Opcode::LoadLocal(tmp_idx));
                self.emit(Opcode::Swap);
                self.emit_store_lvalue(inner)
            }
            Expr::Index(container, idx) => {
                let result_tmp = self.fresh_assign_tmp();
                let result_idx = self.local_index(&result_tmp);
                self.emit(Opcode::Swap);
                self.emit(Opcode::StoreLocal(result_idx));
                let val_tmp = self.fresh_assign_tmp();
                let val_idx = self.local_index(&val_tmp);
                self.emit(Opcode::StoreLocal(val_idx));
                self.emit_load_lvalue_container(container)?;
                self.compile_expr(idx)?;
                self.emit(Opcode::LoadLocal(val_idx));
                self.emit(Opcode::IndexSet);
                self.emit(Opcode::Pop);
                self.emit(Opcode::LoadLocal(result_idx));
                self.emit(Opcode::Swap);
                self.emit_store_lvalue(container)
            }
            _ => Err(CompileError),
        }
    }

    fn emit_load_object(&mut self, obj: &Expr) -> Result<(), CompileError> {
        self.emit_load_lvalue_container(obj)
    }

    fn emit_store_object(&mut self, obj: &Expr) -> Result<(), CompileError> {
        self.emit_store_lvalue(obj)
    }

    fn compile_assign(&mut self, target: &AssignTarget, value: &Expr) -> Result<(), CompileError> {
        match target {
            AssignTarget::Name(name) => {
                self.compile_expr(value)?;
                self.emit(Opcode::Dup);
                self.emit_store_name(name);
                Ok(())
            }
            AssignTarget::Index(obj, idx) => {
                self.emit_load_object(obj)?;
                self.compile_expr(idx)?;
                self.compile_expr(value)?;
                self.emit(Opcode::IndexSet);
                self.emit(Opcode::Swap);
                self.emit_store_object(obj)?;
                Ok(())
            }
            AssignTarget::Member(obj, field) => {
                self.emit_load_object(obj)?;
                self.compile_expr(value)?;
                let field_idx = self.const_index(Constant::String(field.clone()));
                self.emit(Opcode::MemberSet(field_idx));
                self.emit(Opcode::Swap);
                self.emit_store_object(obj)?;
                Ok(())
            }
            AssignTarget::Pattern(pat) => {
                self.compile_expr(value)?;
                self.emit(Opcode::Dup);
                self.compile_bind_pattern(pat, false)?;
                Ok(())
            }
        }
    }

    fn finish_loop(&mut self, frame: LoopFrame, end: usize) {
        for patch in frame.break_patches {
            patch_jump(&mut self.code, patch, end);
        }
    }

    fn emit_iterator_close(&mut self, close: &IteratorCloseCtx) {
        self.emit_load_name(&close.src_local);
        let null_idx = self.const_index(Constant::Null);
        self.emit(Opcode::Const(null_idx));
        let helper = if close.async_for {
            "async_iterator_close"
        } else {
            "iterator_close"
        };
        let idx = self.global_index(helper);
        self.emit(Opcode::LoadGlobal(idx));
        self.emit(Opcode::Call(2));
        if close.async_for {
            self.emit(Opcode::Await);
        }
        self.emit(Opcode::Pop);
    }

    fn emit_active_iterator_closes(&mut self) {
        let closes: Vec<IteratorCloseCtx> = self
            .loop_stack
            .iter()
            .rev()
            .filter_map(|frame| frame.iterator_close.clone())
            .collect();
        for close in closes {
            self.emit_iterator_close(&close);
        }
    }

    fn finish_foreach_loop(&mut self, mut frame: LoopFrame, normal_end: usize) {
        if let Some(close) = frame.iterator_close.take() {
            let skip_close = self.code.len();
            self.emit(Opcode::Jump(0));
            let close_start = self.code.len();
            for patch in frame.break_patches.drain(..) {
                patch_jump(&mut self.code, patch, close_start);
            }
            self.emit_iterator_close(&close);
            let final_end = self.code.len();
            patch_jump(&mut self.code, skip_close, final_end);
        } else {
            self.finish_loop(frame, normal_end);
        }
    }

    fn compile_while(&mut self, cond: &Expr, body: &Expr) -> Result<(), CompileError> {
        let loop_start = self.code.len();
        self.loop_stack.push(LoopFrame {
            break_patches: Vec::new(),
            continue_target: loop_start,
            iterator_close: None,
        });
        self.compile_expr(cond)?;
        let exit_jump = self.code.len();
        self.emit(Opcode::JumpIfFalse(0));
        self.compile_block_body(body)?;
        let back_jump = self.code.len();
        self.emit(Opcode::Jump(0));
        let end = self.code.len();
        patch_jump(&mut self.code, exit_jump, end);
        patch_jump(&mut self.code, back_jump, loop_start);
        let frame = self.loop_stack.pop().expect("loop stack");
        self.finish_loop(frame, end);
        let null_idx = self.const_index(Constant::Null);
        self.emit(Opcode::Const(null_idx));
        Ok(())
    }

    fn compile_do_while(&mut self, body: &Expr, cond: &Expr) -> Result<(), CompileError> {
        let loop_start = self.code.len();
        self.loop_stack.push(LoopFrame {
            break_patches: Vec::new(),
            continue_target: loop_start,
            iterator_close: None,
        });
        self.compile_block_body(body)?;
        self.compile_expr(cond)?;
        let exit_jump = self.code.len();
        self.emit(Opcode::JumpIfFalse(0));
        let back_jump = self.code.len();
        self.emit(Opcode::Jump(0));
        let end = self.code.len();
        patch_jump(&mut self.code, exit_jump, end);
        patch_jump(&mut self.code, back_jump, loop_start);
        let frame = self.loop_stack.pop().expect("loop stack");
        self.finish_loop(frame, end);
        let null_idx = self.const_index(Constant::Null);
        self.emit(Opcode::Const(null_idx));
        Ok(())
    }

    fn compile_foreach(&mut self, loop_: &crate::ast::ForeachLoop) -> Result<(), CompileError> {
        if loop_.async_for {
            if !self.in_async_fn {
                return Err(CompileError);
            }
            if !loop_.by_value {
                return Err(CompileError);
            }
        }
        let n = self.for_loop_counter;
        self.for_loop_counter += 1;
        let src = format!("__kab_f{n}_src");
        let result_name = format!("__kab_f{n}_r");
        self.local_index(&src);
        self.local_index(&result_name);

        self.compile_expr(&loop_.iterable)?;
        self.emit_store_name(&src);

        if loop_.by_value {
            self.emit_load_name(&src);
            let begin = if loop_.async_for {
                "async_iterator_begin"
            } else {
                "iterator_begin"
            };
            let begin_idx = self.global_index(begin);
            self.emit(Opcode::LoadGlobal(begin_idx));
            self.emit(Opcode::Call(1));
            self.emit_store_name(&src);

            let loop_start = self.code.len();
            self.loop_stack.push(LoopFrame {
                break_patches: Vec::new(),
                continue_target: loop_start,
                iterator_close: Some(IteratorCloseCtx {
                    src_local: src.clone(),
                    async_for: loop_.async_for,
                }),
            });

            self.emit_load_name(&src);
            if loop_.async_for {
                self.emit(Opcode::AsyncIteratorStepInPlace);
                self.emit(Opcode::Await);
            } else {
                self.emit(Opcode::IteratorStepInPlace);
            }
            self.emit_store_name(&result_name);
            self.emit_store_name(&src);

            self.emit_load_name(&result_name);
            let done_key = self.const_index(Constant::String("done".into()));
            self.emit(Opcode::Const(done_key));
            self.emit(Opcode::IndexGet);
            let body_jump = self.code.len();
            self.emit(Opcode::JumpIfFalse(0));
            let exit_jump = self.code.len();
            self.emit(Opcode::Jump(0));

            let body_start = self.code.len();
            patch_jump(&mut self.code, body_jump, body_start);

            self.emit_load_name(&result_name);
            let value_key = self.const_index(Constant::String("value".into()));
            self.emit(Opcode::Const(value_key));
            self.emit(Opcode::IndexGet);
            if loop_.immutable {
                self.mark_local_immutable(&loop_.var);
            }
            self.emit_store_name(&loop_.var);

            self.compile_block_body(&loop_.body)?;

            let continue_target = self.code.len();
            if let Some(frame) = self.loop_stack.last_mut() {
                frame.continue_target = continue_target;
            }
            let back_jump = self.code.len();
            self.emit(Opcode::Jump(0));
            let normal_end = self.code.len();
            patch_jump(&mut self.code, exit_jump, normal_end);
            patch_jump(&mut self.code, back_jump, loop_start);

            let frame = self.loop_stack.pop().expect("loop stack");
            self.finish_foreach_loop(frame, normal_end);

            let null_idx = self.const_index(Constant::Null);
            self.emit(Opcode::Const(null_idx));
            return Ok(());
        }

        let idx_name = format!("__kab_f{n}_i");
        {
            // Objects: materialize keys into an array before indexed iteration.
            self.emit_load_name(&src);
            let jump_not_object = self.code.len();
            self.emit(Opcode::JumpUnlessObject(0));
            self.emit_load_name(&src);
            let helper_idx = self.global_index("keys");
            self.emit(Opcode::LoadGlobal(helper_idx));
            self.emit(Opcode::Call(1));
            self.emit_store_name(&src);
            let after_object = self.code.len();
            patch_jump(&mut self.code, jump_not_object, after_object);
        }

        let zero = self.const_index(Constant::Number(0));
        self.emit(Opcode::Const(zero));
        self.emit_store_name(&idx_name);

        let loop_start = self.code.len();
        self.loop_stack.push(LoopFrame {
            break_patches: Vec::new(),
            continue_target: loop_start,
            iterator_close: None,
        });
        self.emit_load_name(&idx_name);
        self.emit_load_name(&src);
        self.emit(Opcode::GetLength);
        self.emit(Opcode::Lt);
        let exit_jump = self.code.len();
        self.emit(Opcode::JumpIfFalse(0));

        if loop_.by_value {
            self.emit_load_name(&src);
            self.emit_load_name(&idx_name);
            self.emit(Opcode::IndexGet);
        } else {
            self.emit_load_name(&idx_name);
        }
        self.emit_store_name(&loop_.var);
        if loop_.immutable {
            self.mark_local_immutable(&loop_.var);
        }

        self.compile_block_body(&loop_.body)?;

        let continue_target = self.code.len();
        if let Some(frame) = self.loop_stack.last_mut() {
            frame.continue_target = continue_target;
        }

        self.emit_load_name(&idx_name);
        let one = self.const_index(Constant::Number(1));
        self.emit(Opcode::Const(one));
        self.emit(Opcode::Add);
        self.emit_store_name(&idx_name);

        let back_jump = self.code.len();
        self.emit(Opcode::Jump(0));
        let end = self.code.len();
        patch_jump(&mut self.code, exit_jump, end);
        patch_jump(&mut self.code, back_jump, loop_start);

        let frame = self.loop_stack.pop().expect("loop stack");
        self.finish_loop(frame, end);

        let null_idx = self.const_index(Constant::Null);
        self.emit(Opcode::Const(null_idx));
        Ok(())
    }

    fn compile_switch(
        &mut self,
        scrutinee: &Expr,
        cases: &[crate::ast::SwitchCase],
        default_body: Option<&Expr>,
    ) -> Result<(), CompileError> {
        let tmp = format!("__kab_sw_{}", self.for_loop_counter);
        self.for_loop_counter += 1;
        self.compile_expr(scrutinee)?;
        self.emit_store_name(&tmp);

        let mut test_starts = Vec::new();
        let mut fail_patches = Vec::new();
        let mut enter_body_jumps = Vec::new();
        for case in cases {
            test_starts.push(self.code.len());
            self.emit_load_name(&tmp);
            self.compile_expr(&case.value)?;
            self.emit(Opcode::Eq);
            let fail_jump = self.code.len();
            self.emit(Opcode::JumpIfFalse(0));
            let enter_jump = self.code.len();
            self.emit(Opcode::Jump(0));
            fail_patches.push(fail_jump);
            enter_body_jumps.push(enter_jump);
            let _ = case;
        }

        let default_start = self.code.len();
        for (i, fail) in fail_patches.iter().enumerate() {
            let next_test = if i + 1 < cases.len() {
                test_starts[i + 1]
            } else {
                default_start
            };
            patch_jump(&mut self.code, *fail, next_test);
        }

        let mut end_jumps: Vec<usize> = Vec::new();
        for (i, case) in cases.iter().enumerate() {
            let body_start = self.code.len();
            patch_jump(&mut self.code, enter_body_jumps[i], body_start);
            self.compile_block_or_expr(&case.body, true)?;
            if !switch_body_fallthroughs(&case.body) {
                let end_jump = self.code.len();
                self.emit(Opcode::Jump(0));
                end_jumps.push(end_jump);
            }
        }
        if let Some(def) = default_body {
            self.compile_block_or_expr(def, true)?;
        } else {
            let null_idx = self.const_index(Constant::Null);
            self.emit(Opcode::Const(null_idx));
        }
        let end = self.code.len();
        for jump in end_jumps {
            patch_jump(&mut self.code, jump, end);
        }
        Ok(())
    }

    fn compile_for_classic(
        &mut self,
        init: &Option<Box<Stmt>>,
        cond: &Option<Box<Expr>>,
        step: &Option<Box<Expr>>,
        body: &Expr,
    ) -> Result<(), CompileError> {
        if let Some(init_stmt) = init {
            self.compile_stmt(init_stmt.as_ref(), false)?;
        }
        let loop_start = self.code.len();
        let exit_jump = if let Some(c) = cond {
            self.loop_stack.push(LoopFrame {
                break_patches: Vec::new(),
                continue_target: loop_start,
                iterator_close: None,
            });
            self.compile_expr(c)?;
            let j = self.code.len();
            self.emit(Opcode::JumpIfFalse(0));
            Some(j)
        } else {
            None
        };

        self.compile_block_body(body)?;

        let step_target = self.code.len();
        if let Some(frame) = self.loop_stack.last_mut() {
            frame.continue_target = step_target;
        }

        if let Some(s) = step {
            self.compile_expr(s)?;
            self.emit(Opcode::Pop);
        }

        let back_jump = self.code.len();
        self.emit(Opcode::Jump(0));
        let end = self.code.len();
        if let Some(j) = exit_jump {
            patch_jump(&mut self.code, j, end);
        }
        patch_jump(&mut self.code, back_jump, loop_start);
        if exit_jump.is_some() {
            let frame = self.loop_stack.pop().expect("loop stack");
            self.finish_loop(frame, end);
        }

        let null_idx = self.const_index(Constant::Null);
        self.emit(Opcode::Const(null_idx));
        Ok(())
    }

    fn compile_block_or_expr(&mut self, expr: &Expr, want_value: bool) -> Result<(), CompileError> {
        match expr {
            Expr::Block(stmts) => {
                for (i, stmt) in stmts.iter().enumerate() {
                    self.compile_stmt(stmt, want_value && i + 1 == stmts.len())?;
                }
                Ok(())
            }
            other => self.compile_expr(other),
        }
    }

    fn compile_block_body(&mut self, body: &Expr) -> Result<(), CompileError> {
        match body {
            Expr::Block(stmts) => {
                for stmt in stmts {
                    self.compile_stmt(stmt, false)?;
                }
                Ok(())
            }
            other => {
                self.compile_expr(other)?;
                self.emit(Opcode::Pop);
                Ok(())
            }
        }
    }

    fn compile_stmt(&mut self, stmt: &Stmt, is_last: bool) -> Result<(), CompileError> {
        match stmt {
            Stmt::Let {
                pattern,
                init,
                public,
                immutable,
            } => {
                if *public && init.is_none() {
                    return Err(CompileError);
                }
                if let Some(expr) = init {
                    self.compile_expr(expr)?;
                } else {
                    let idx = self.const_index(Constant::Undefined);
                    self.emit(Opcode::Const(idx));
                }
                match pattern {
                    BindingPattern::Name(name) => {
                        let idx = self.local_index(name);
                        if *immutable {
                            self.mark_local_immutable(name);
                        }
                        self.emit(Opcode::StoreLocal(idx));
                        if is_last {
                            self.emit(Opcode::LoadLocal(idx));
                        }
                    }
                    _ => {
                        self.compile_bind_pattern(pattern, *immutable)?;
                        if is_last {
                            let null_idx = self.const_index(Constant::Null);
                            self.emit(Opcode::Const(null_idx));
                        }
                    }
                }
                if *public {
                    for name in exported_binding_names(pattern) {
                        self.exports.push(name);
                    }
                }
                Ok(())
            }
            Stmt::Expr(expr) => {
                self.compile_expr(expr)?;
                if !is_last
                    && !matches!(
                        expr,
                        Expr::Yield(_) | Expr::YieldStar(_) | Expr::Break | Expr::Continue
                    )
                {
                    self.emit(Opcode::Pop);
                }
                Ok(())
            }
            Stmt::Return(Some(expr)) => {
                self.emit_active_iterator_closes();
                self.compile_expr(expr)?;
                self.emit(Opcode::Return);
                Ok(())
            }
            Stmt::Return(None) => {
                self.emit_active_iterator_closes();
                let idx = self.const_index(Constant::Null);
                self.emit(Opcode::Const(idx));
                self.emit(Opcode::Return);
                Ok(())
            }
            _ => Err(CompileError),
        }
    }

    fn compile_function_body(
        &mut self,
        params: &[String],
        body: &Expr,
        async_fn: bool,
    ) -> Result<(), CompileError> {
        let saved_async = self.in_async_fn;
        self.in_async_fn = async_fn;
        for p in params {
            self.local_index(p);
        }
        match body {
            Expr::Block(stmts) => {
                for (i, stmt) in stmts.iter().enumerate() {
                    self.compile_stmt(stmt, i + 1 == stmts.len())?;
                }
            }
            other => {
                self.compile_expr(other)?;
            }
        }
        if !matches!(self.code.last(), Some(Opcode::Return)) {
            if self.in_generator {
                let idx = self.const_index(Constant::Undefined);
                self.emit(Opcode::Const(idx));
            }
            self.emit(Opcode::Return);
        }
        self.in_async_fn = saved_async;
        Ok(())
    }
}

fn switch_body_fallthroughs(body: &crate::ast::Expr) -> bool {
    match body {
        crate::ast::Expr::Block(stmts) => stmts
            .last()
            .is_some_and(|stmt| matches!(stmt, crate::ast::Stmt::Expr(crate::ast::Expr::Fallthrough))),
        crate::ast::Expr::Fallthrough => true,
        _ => false,
    }
}

fn patch_jump(code: &mut [Opcode], at: usize, target: usize) {
    let offset = (target as i32) - (at as i32) - 1;
    match &mut code[at] {
        Opcode::Jump(ref mut off)
        | Opcode::JumpIfFalse(ref mut off)
        | Opcode::JumpIfResultErr(ref mut off)
        | Opcode::JumpUnlessResultOk(ref mut off)
        | Opcode::JumpUnlessResultErr(ref mut off)
        | Opcode::JumpUnlessOptionSome(ref mut off)
        | Opcode::JumpUnlessOptionNone(ref mut off)
        | Opcode::JumpUnlessConstEq(_, ref mut off)
        | Opcode::JumpUnlessArray(ref mut off)
        | Opcode::JumpUnlessObject(ref mut off)
        | Opcode::JumpUnlessObjectEmpty(ref mut off)
        | Opcode::JumpUnlessHasMember(_, ref mut off)
        | Opcode::JumpIfNotNullish(ref mut off) => *off = offset,
        _ => {}
    }
}

pub fn try_compile(stmts: &[Stmt]) -> Option<BytecodeModule> {
    let mut main = Compiler::new();
    let mut functions = Vec::new();
    let mut classes = Vec::new();
    let mut interfaces = Vec::new();
    let mut enums = Vec::new();
    let mut imports = Vec::new();
    let mut pub_imports = Vec::new();
    let mut exports = Vec::new();

    for (i, stmt) in stmts.iter().enumerate() {
        let is_last = i + 1 == stmts.len();
        if let Stmt::Import { module, public } = stmt {
            imports.push(module.clone());
            if *public {
                pub_imports.push(module.clone());
            }
            continue;
        }
        if let Stmt::Interface { name, methods } = stmt {
            interfaces.push(BytecodeInterfaceDef {
                name: name.clone(),
                methods: methods
                    .iter()
                    .map(|m| BytecodeInterfaceMethod {
                        name: m.name.clone(),
                        params: m.params.clone(),
                    })
                    .collect(),
            });
            continue;
        }
        if let Stmt::Enum { name, variants } = stmt {
            enums.push(BytecodeEnumDef {
                name: name.clone(),
                variants: variants
                    .iter()
                    .map(|v| BytecodeEnumVariantDef {
                        name: v.name.clone(),
                        fields: v.fields.clone(),
                    })
                    .collect(),
            });
            continue;
        }
        if let Stmt::Class {
            name,
            extends,
            implements,
            fields,
            methods,
        } = stmt
        {
            let class_idx = classes.len() as u16;
            main.class_names.insert(name.clone(), class_idx);
            let class_def = match main.compile_class(name, extends, implements, fields, methods) {
                Ok(def) => def,
                Err(_) => return None,
            };
            classes.push(class_def);
            continue;
        }
        if let Stmt::Expr(Expr::Function {
            name,
            params,
            rest,
            body,
            public,
            async_fn,
            generator_fn,
        }) = stmt
        {
            if crate::ast::fn_has_defaults_or_rest(params, rest) {
                return None;
            }
            let param_names = crate::ast::fn_param_names(params);
            let mut fn_compiler = Compiler::new();
            fn_compiler.constants = main.constants.clone();
            fn_compiler.globals = main.globals.clone();
            fn_compiler.in_generator = *generator_fn;
            fn_compiler.in_async_fn = *async_fn;
            if fn_compiler
                .compile_function_body(&param_names, body, *async_fn)
                .is_err()
            {
                return None;
            }
            main.constants = fn_compiler.constants.clone();
            main.globals = fn_compiler.globals.clone();
            main.global_index(name);
            functions.push(BytecodeFnDef {
                name: name.clone(),
                params: param_names,
                locals: fn_compiler.locals,
                globals: fn_compiler.globals,
                constants: fn_compiler.constants,
                code: fn_compiler.code,
                immutable_locals: fn_compiler.immutable_locals,
                arrow_functions: fn_compiler.arrow_functions,
                async_fn: *async_fn,
                generator_fn: *generator_fn,
                try_regions: fn_compiler.try_regions,
            });
            if *public {
                exports.push(name.clone());
            }
            continue;
        }

        if main.compile_stmt(stmt, is_last).is_err() {
            return None;
        }
    }

    if !matches!(
        main.code.last(),
        Some(Opcode::Return) | Some(Opcode::Halt)
    ) {
        main.emit(Opcode::Halt);
    }

    Some({
        let mut module = BytecodeModule {
        constants: main.constants,
        globals: main.globals,
        main_locals: main.locals,
        main_immutable_locals: main.immutable_locals,
        main_try_regions: main.try_regions,
        main_code: main.code,
        functions,
        arrow_functions: main.arrow_functions,
        classes,
        interfaces,
        enums,
        imports,
        pub_imports,
        exports: {
            exports.append(&mut main.exports);
            exports
        },
    };
        super::optimize::optimize_module(&mut module);
        module
    })
}

pub fn can_compile(source: &str) -> bool {
    crate::bytecode::compile_source(source)
        .ok()
        .and_then(|p| try_compile(&p.stmts))
        .is_some()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::bytecode::compile_source;

    #[test]
    fn compiles_arithmetic_program() {
        let p = compile_source(
            r#"
            let x = 10
            let y = 20
            x + y
        "#,
        )
        .unwrap();
        let bc = try_compile(&p.stmts).expect("bytecode");
        assert!(!bc.main_code.is_empty());
    }

    #[test]
    fn compiles_user_function() {
        let p = compile_source(
            r#"
            fn dbl(x) { return x * 2 }
            dbl(5)
        "#,
        )
        .unwrap();
        let bc = try_compile(&p.stmts).expect("bytecode");
        assert_eq!(bc.functions.len(), 1);
    }

    #[test]
    fn compiles_iife_arrow() {
        let p = compile_source("((n) => n + 1)(4)").unwrap();
        assert!(try_compile(&p.stmts).is_some());
    }

    #[test]
    fn compiles_pub_fn() {
        let p = compile_source("pub fn f() { return 1 }").unwrap();
        let bc = try_compile(&p.stmts).expect("bytecode");
        assert_eq!(bc.exports, vec!["f".to_string()]);
    }

    #[test]
    fn accepts_import_stmt() {
        let p = compile_source(r#"import "math""#).unwrap();
        let bc = try_compile(&p.stmts).expect("bytecode");
        assert_eq!(bc.imports, vec!["math".to_string()]);
    }

    #[test]
    fn compiles_const_binding() {
        let p = compile_source(
            r#"
            const PI = 3
            PI
        "#,
        )
        .unwrap();
        let bc = try_compile(&p.stmts).expect("bytecode");
        assert_eq!(bc.main_immutable_locals.get(0), Some(&true));
    }

    #[test]
    fn compiles_spread_destructure_and_try() {
        let p = compile_source(
            r#"
            let [a, ...rest] = [1, 2, 3]
            let o = { ...{ x: 1 }, y: 2 }
            try { Err("x") } catch (e) { e }
        "#,
        )
        .unwrap();
        let bc = try_compile(&p.stmts).expect("bytecode");
        assert!(bc.main_code.iter().any(|op| matches!(op, Opcode::ConcatArray | Opcode::MergeObject)));
        assert!(bc.main_code.iter().any(|op| matches!(op, Opcode::JumpIfResultErr(_))));
    }

    #[test]
    fn compiles_arrow_and_match() {
        let p = compile_source(
            r#"
            let f = (x) => x + 1
            match 7 {
                7 => f(6),
                _ => 0
            }
        "#,
        )
        .unwrap();
        let bc = try_compile(&p.stmts).expect("bytecode");
        assert!(!bc.arrow_functions.is_empty());
        assert!(bc.main_code.iter().any(|op| matches!(op, Opcode::MakeArrowFn(_))));
    }

    #[test]
    fn compiles_point_class_program() {
        let p = compile_source(
            r#"
            class Point {
                x: number;
                y: number;
                fn init(a, b) {
                    self.x = a
                    self.y = b
                }
                fn sum() {
                    return self.x + self.y
                }
            }
            let p = Point(3, 4)
            p.sum()
        "#,
        )
        .unwrap();
        let bc = try_compile(&p.stmts).expect("bytecode");
        assert_eq!(bc.classes.len(), 1);
        assert!(bc.main_code.iter().any(|op| matches!(op, Opcode::NewInstance(_, _))));
    }

    #[test]
    fn compiles_class_shell_only() {
        let p = compile_source("class Empty {}").unwrap();
        let bc = try_compile(&p.stmts).expect("bytecode");
        assert_eq!(bc.classes.len(), 1);
    }

    #[test]
    fn compiles_class_with_init_only() {
        let p = compile_source(
            r#"
            class Point {
                x: number;
                fn init(a) { self.x = a }
            }
        "#,
        )
        .unwrap();
        let bc = try_compile(&p.stmts).expect("bytecode");
        assert_eq!(bc.classes.len(), 1);
    }
}
