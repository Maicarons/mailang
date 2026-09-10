use std::collections::HashMap;
use mailang_ast::*;
use mailang_bytecode::*;
use crate::error::CompilerError;

struct Local {
    name: String,
    depth: usize,
    captured: bool,
}

struct Upvalue {
    index: u8,
    is_local: bool,
}

impl Clone for Upvalue {
    fn clone(&self) -> Self {
        Self {
            index: self.index,
            is_local: self.is_local,
        }
    }
}

struct FunctionCompiler {
    chunk_index: usize,
    locals: Vec<Local>,
    upvalues: Vec<Upvalue>,
    scope_depth: usize,
}

pub struct Compiler {
    bytecode: Bytecode,
    current: FunctionCompiler,
    function_compilers: Vec<FunctionCompiler>,
    globals: HashMap<String, u32>,
    loop_breaks: Vec<Vec<usize>>,
    loop_continues: Vec<Vec<usize>>,
    loop_local_counts: Vec<usize>, // track locals count at loop start for break cleanup
}

impl Compiler {
    pub fn new() -> Self {
        let bytecode = Bytecode::new();
        let current = FunctionCompiler {
            chunk_index: 0,
            locals: Vec::new(),
            upvalues: Vec::new(),
            scope_depth: 0,
        };
        Self {
            bytecode,
            current,
            function_compilers: Vec::new(),
            globals: HashMap::new(),
            loop_breaks: Vec::new(),
            loop_continues: Vec::new(),
            loop_local_counts: Vec::new(),
        }
    }

    pub fn compile(mut self, program: &Program) -> Result<Bytecode, CompilerError> {
        let len = program.statements.len();
        for (i, stmt) in program.statements.iter().enumerate() {
            if i == len - 1 {
                if let Stmt::Expression(expr) = stmt {
                    self.compile_expression(expr)?;
                } else {
                    self.compile_statement(stmt)?;
                    self.emit_push_constant(Value::Null, 0)?;
                }
            } else {
                self.compile_statement(stmt)?;
            }
        }
        self.emit(Opcode::Halt, None, 0);
        Ok(self.bytecode)
    }

    fn emit(&mut self, opcode: Opcode, operand: Option<u32>, line: usize) {
        self.bytecode.chunks[self.current.chunk_index].emit(opcode, operand, line);
    }

    fn emit_push_constant(&mut self, value: Value, line: usize) -> Result<(), CompilerError> {
        let idx = self.add_constant(value)?;
        self.emit(Opcode::Push, Some(idx), line);
        Ok(())
    }

    fn add_constant(&mut self, value: Value) -> Result<u32, CompilerError> {
        let index = self.bytecode.chunks[self.current.chunk_index].add_constant(value);
        if index > 65535 {
            return Err(CompilerError::TooManyConstants);
        }
        Ok(index)
    }

    fn current_chunk(&self) -> &Chunk {
        &self.bytecode.chunks[self.current.chunk_index]
    }

    fn current_chunk_mut(&mut self) -> &mut Chunk {
        &mut self.bytecode.chunks[self.current.chunk_index]
    }

    fn begin_scope(&mut self) {
        self.current.scope_depth += 1;
    }

    fn end_scope(&mut self) -> usize {
        self.current.scope_depth -= 1;
        let mut count = 0;
        while let Some(local) = self.current.locals.last() {
            if local.depth > self.current.scope_depth {
                if local.captured {
                    self.emit(Opcode::Pop, None, 0);
                } else {
                    self.emit(Opcode::Pop, None, 0);
                }
                self.current.locals.pop();
                count += 1;
            } else {
                break;
            }
        }
        count
    }

    fn resolve_local(&self, name: &str) -> Option<u32> {
        for (i, local) in self.current.locals.iter().enumerate().rev() {
            if local.name == name {
                return Some(i as u32);
            }
        }
        None
    }

    fn resolve_upvalue(&mut self, name: &str) -> Option<u32> {
        if self.function_compilers.is_empty() {
            return None;
        }

        let compiler_index = self.function_compilers.len() - 1;
        if let Some(local) = self.resolve_local_in(compiler_index, name) {
            self.function_compilers[compiler_index].locals[local as usize].captured = true;
            return Some(self.add_upvalue(local, true)?);
        }

        None
    }

    fn resolve_local_in(&self, compiler_index: usize, name: &str) -> Option<u32> {
        let compiler = &self.function_compilers[compiler_index];
        for (i, local) in compiler.locals.iter().enumerate().rev() {
            if local.name == name {
                return Some(i as u32);
            }
        }
        None
    }

    fn add_upvalue(&mut self, index: u32, is_local: bool) -> Option<u32> {
        for (i, upvalue) in self.current.upvalues.iter().enumerate() {
            if upvalue.index == index as u8 && upvalue.is_local == is_local {
                return Some(i as u32);
            }
        }

        if self.current.upvalues.len() >= 256 {
            return None;
        }

        self.current.upvalues.push(Upvalue {
            index: index as u8,
            is_local,
        });
        Some((self.current.upvalues.len() - 1) as u32)
    }

    fn compile_statement(&mut self, stmt: &Stmt) -> Result<(), CompilerError> {
        match stmt {
            Stmt::Let {
                name,
                mutable,
                value,
                ..
            } => {
                if let Some(val) = value {
                    self.compile_expression(val)?;
                } else {
                    self.emit_push_constant(Value::Null, 0)?;
                }
                self.define_variable(name)?;
            }
            Stmt::Const {
                name, value, ..
            } => {
                self.compile_expression(value)?;
                self.define_variable(name)?;
            }
            Stmt::FunctionDef {
                name,
                params,
                return_type,
                body,
            } => {
                self.compile_function(name, params, body)?;
            }
            Stmt::ClassDef {
                name,
                superclass,
                traits,
                members,
            } => {
                self.compile_class(name, superclass, traits, members)?;
            }
            Stmt::TraitDef { name, methods } => {
                self.compile_trait(name, methods)?;
            }
            Stmt::ModuleDef { name, body } => {
                self.begin_scope();
                for stmt in body {
                    self.compile_statement(stmt)?;
                }
                self.end_scope();
            }
            Stmt::Import { .. } => {}
            Stmt::Expression(expr) => {
                self.compile_expression(expr)?;
                // Assignments don't leave a value on the stack, so don't Pop
                match expr {
                    Expr::Assign { .. } | Expr::CompoundAssign { .. } => {}
                    _ => { self.emit(Opcode::Pop, None, 0); }
                }
            }
            Stmt::Return(value) => {
                if let Some(val) = value {
                    self.compile_expression(val)?;
                } else {
                    self.emit_push_constant(Value::Null, 0)?;
                }
                self.emit(Opcode::Return, None, 0);
            }
            Stmt::If {
                condition,
                then_branch,
                elif_branches,
                else_branch,
            } => {
                self.compile_expression(condition)?;
                let jump_else = self.emit_jump(Opcode::JumpIfFalse, 0);
                self.emit(Opcode::Pop, None, 0);

                self.begin_scope();
                for stmt in then_branch {
                    self.compile_statement(stmt)?;
                }
                self.end_scope();

                let jump_end = self.emit_jump(Opcode::Jump, 0);
                self.patch_jump(jump_else)?;
                self.emit(Opcode::Pop, None, 0);

                for (cond, body) in elif_branches {
                    self.compile_expression(cond)?;
                    let jump = self.emit_jump(Opcode::JumpIfFalse, 0);
                    self.emit(Opcode::Pop, None, 0);

                    self.begin_scope();
                    for stmt in body {
                        self.compile_statement(stmt)?;
                    }
                    self.end_scope();

                    let jump_end = self.emit_jump(Opcode::Jump, 0);
                    self.patch_jump(jump)?;
                    self.emit(Opcode::Pop, None, 0);
                }

                if let Some(body) = else_branch {
                    self.begin_scope();
                    for stmt in body {
                        self.compile_statement(stmt)?;
                    }
                    self.end_scope();
                }

                self.patch_jump(jump_end)?;
            }
            Stmt::For {
                variable,
                iterable,
                body,
            } => {
                // Check if iterable is a range expression
                match iterable {
                    Expr::Range { start, end } => {
                        // For range: iterate from start to end-1
                        // Compile start value and store in local
                        self.compile_expression(start)?;
                        let counter_local = self.current.locals.len() as u32;
                        self.emit(Opcode::StoreLocal, Some(counter_local), 0);
                        self.current.locals.push(Local {
                            name: format!("__counter_{}", counter_local),
                            depth: self.current.scope_depth,
                            captured: false,
                        });

                        // Compile end value and store in local
                        self.compile_expression(end)?;
                        let end_local = self.current.locals.len() as u32;
                        self.emit(Opcode::StoreLocal, Some(end_local), 0);
                        self.current.locals.push(Local {
                            name: format!("__end_{}", end_local),
                            depth: self.current.scope_depth,
                            captured: false,
                        });

                        let loop_start = self.current_chunk().instructions.len();
                        self.loop_breaks.push(Vec::new());
                        self.loop_continues.push(Vec::new());

                        // Check counter < end
                        self.emit(Opcode::LoadLocal, Some(counter_local), 0);
                        self.emit(Opcode::LoadLocal, Some(end_local), 0);
                        self.emit(Opcode::Lt, None, 0);
                        let jump_end = self.emit_jump(Opcode::JumpIfFalse, 0);
                        self.emit(Opcode::Pop, None, 0);

                        // Set loop variable to counter
                        self.begin_scope();
                        self.define_variable(variable)?;
                        self.emit(Opcode::LoadLocal, Some(counter_local), 0);
                        let var_idx = self.current.locals.len() as u32 - 1;
                        self.emit(Opcode::StoreLocal, Some(var_idx), 0);

                        // Execute body
                        for stmt in body {
                            self.compile_statement(stmt)?;
                        }
                        self.end_scope();

                        // Increment counter
                        self.emit(Opcode::LoadLocal, Some(counter_local), 0);
                        self.emit_push_constant(Value::Int(1), 0)?;
                        self.emit(Opcode::Add, None, 0);
                        self.emit(Opcode::StoreLocal, Some(counter_local), 0);

                        // Jump back to loop start
                        self.emit_jump(Opcode::Jump, loop_start);
                        self.patch_jump(jump_end)?;
                        self.emit(Opcode::Pop, None, 0);

                        let breaks = self.loop_breaks.pop().unwrap();
                        for break_pos in breaks {
                            self.patch_jump_at(break_pos, self.current_chunk().instructions.len())?;
                        }
                        let continues = self.loop_continues.pop().unwrap();
                        for continue_pos in continues {
                            self.patch_jump_at(continue_pos, loop_start)?;
                        }
                    }
                    _ => {
                        // For arrays: iterate through elements
                        // Compile iterable (should be an array) and store in local
                        self.compile_expression(iterable)?;
                        let array_local = self.current.locals.len() as u32;
                        self.emit(Opcode::StoreLocal, Some(array_local), 0);
                        self.current.locals.push(Local {
                            name: format!("__array_{}", array_local),
                            depth: self.current.scope_depth,
                            captured: false,
                        });

                        // Initialize index to 0 and store in local
                        self.emit_push_constant(Value::Int(0), 0)?;
                        let index_local = self.current.locals.len() as u32;
                        self.emit(Opcode::StoreLocal, Some(index_local), 0);
                        self.current.locals.push(Local {
                            name: format!("__index_{}", index_local),
                            depth: self.current.scope_depth,
                            captured: false,
                        });

                        let loop_start = self.current_chunk().instructions.len();
                        self.loop_breaks.push(Vec::new());
                        self.loop_continues.push(Vec::new());

                        // Check index < array.len()
                        self.emit(Opcode::LoadLocal, Some(index_local), 0);
                        self.emit(Opcode::LoadLocal, Some(array_local), 0);
                        // Get array length - use GetProperty with "len"
                        let len_const = self.add_constant(Value::Str("len".to_string()))?;
                        self.emit(Opcode::GetProperty, Some(len_const), 0);
                        self.emit(Opcode::Lt, None, 0);
                        let jump_end = self.emit_jump(Opcode::JumpIfFalse, 0);
                        self.emit(Opcode::Pop, None, 0);

                        // Set loop variable to array[index]
                        self.begin_scope();
                        self.define_variable(variable)?;
                        self.emit(Opcode::LoadLocal, Some(array_local), 0);
                        self.emit(Opcode::LoadLocal, Some(index_local), 0);
                        self.emit(Opcode::IndexGet, None, 0);
                        let var_idx = self.current.locals.len() as u32 - 1;
                        self.emit(Opcode::StoreLocal, Some(var_idx), 0);

                        // Execute body
                        for stmt in body {
                            self.compile_statement(stmt)?;
                        }
                        self.end_scope();

                        // Increment index
                        self.emit(Opcode::LoadLocal, Some(index_local), 0);
                        self.emit_push_constant(Value::Int(1), 0)?;
                        self.emit(Opcode::Add, None, 0);
                        self.emit(Opcode::StoreLocal, Some(index_local), 0);

                        // Jump back to loop start
                        self.emit_jump(Opcode::Jump, loop_start);
                        self.patch_jump(jump_end)?;
                        self.emit(Opcode::Pop, None, 0);

                        let breaks = self.loop_breaks.pop().unwrap();
                        for break_pos in breaks {
                            self.patch_jump_at(break_pos, self.current_chunk().instructions.len())?;
                        }
                        let continues = self.loop_continues.pop().unwrap();
                        for continue_pos in continues {
                            self.patch_jump_at(continue_pos, loop_start)?;
                        }
                    }
                }
            }
            Stmt::While { condition, body } => {
                let loop_start = self.current_chunk().instructions.len();
                self.loop_breaks.push(Vec::new());
                self.loop_continues.push(Vec::new());

                self.compile_expression(condition)?;
                let jump_end = self.emit_jump(Opcode::JumpIfFalse, 0);
                self.emit(Opcode::Pop, None, 0);

                let locals_before = self.current.locals.len();
                self.loop_local_counts.push(locals_before);
                self.begin_scope();
                for stmt in body {
                    self.compile_statement(stmt)?;
                }
                self.end_scope();
                self.loop_local_counts.pop();

                self.emit_jump(Opcode::Jump, loop_start);
                self.patch_jump(jump_end)?;
                self.emit(Opcode::Pop, None, 0);

                let breaks = self.loop_breaks.pop().unwrap();
                for break_pos in breaks {
                    self.patch_jump_at(break_pos, self.current_chunk().instructions.len())?;
                }
                let continues = self.loop_continues.pop().unwrap();
                for continue_pos in continues {
                    self.patch_jump_at(continue_pos, loop_start)?;
                }
            }
            Stmt::Break => {
                // Emit Pop instructions for locals defined in current loop body
                if let Some(&locals_before) = self.loop_local_counts.last() {
                    let locals_now = self.current.locals.len();
                    for _ in 0..(locals_now - locals_before) {
                        self.emit(Opcode::Pop, None, 0);
                    }
                }
                let jump = self.emit_jump(Opcode::Jump, 0);
                if let Some(breaks) = self.loop_breaks.last_mut() {
                    breaks.push(jump);
                }
            }
            Stmt::Continue => {
                let jump = self.emit_jump(Opcode::Jump, 0);
                if let Some(continues) = self.loop_continues.last_mut() {
                    continues.push(jump);
                }
            }
        }
        Ok(())
    }

    fn compile_expression(&mut self, expr: &Expr) -> Result<(), CompilerError> {
        match expr {
            Expr::Literal(lit) => {
                let value = match lit {
                    Literal::Int(n) => Value::Int(*n),
                    Literal::Float(n) => Value::Float(*n),
                    Literal::Bool(b) => Value::Bool(*b),
                    Literal::Str(s) => Value::Str(s.clone()),
                    Literal::Char(c) => Value::Char(*c),
                    Literal::Null => Value::Null,
                };
                let index = self.add_constant(value)?;
                self.emit(Opcode::Push, Some(index), 0);
            }
            Expr::Identifier(name) => {
                if let Some(local) = self.resolve_local(name) {
                    self.emit(Opcode::LoadLocal, Some(local), 0);
                } else if let Some(upvalue) = self.resolve_upvalue(name) {
                    self.emit(Opcode::LoadUpvalue, Some(upvalue), 0);
                } else {
                    let index = self.add_constant(Value::Str(name.clone()))?;
                    self.emit(Opcode::LoadGlobal, Some(index), 0);
                }
            }
            Expr::BinaryOp { op, left, right } => {
                self.compile_expression(left)?;
                self.compile_expression(right)?;
                let opcode = match op {
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
                    BinaryOp::BitAnd => Opcode::BitAnd,
                    BinaryOp::BitOr => Opcode::BitOr,
                    BinaryOp::BitXor => Opcode::BitXor,
                    BinaryOp::Shl => Opcode::Shl,
                    BinaryOp::Shr => Opcode::Shr,
                };
                self.emit(opcode, None, 0);
            }
            Expr::UnaryOp { op, operand } => {
                self.compile_expression(operand)?;
                let opcode = match op {
                    UnaryOp::Neg => Opcode::Neg,
                    UnaryOp::Not => Opcode::Not,
                    UnaryOp::BitNot => Opcode::BitNot,
                };
                self.emit(opcode, None, 0);
            }
            Expr::Call { callee, args } => {
                self.compile_expression(callee)?;
                for arg in args {
                    self.compile_expression(arg)?;
                }
                self.emit(Opcode::Call, Some(args.len() as u32), 0);
            }
            Expr::MethodCall {
                object,
                method,
                args,
            } => {
                self.compile_expression(object)?;
                for arg in args {
                    self.compile_expression(arg)?;
                }
                let method_index = self.add_constant(Value::Str(method.clone()))?;
                self.emit(Opcode::Invoke, Some(method_index), 0);
            }
            Expr::PropertyAccess { object, property } => {
                self.compile_expression(object)?;
                let prop_index = self.add_constant(Value::Str(property.clone()))?;
                self.emit(Opcode::GetProperty, Some(prop_index), 0);
            }
            Expr::Index { object, index } => {
                self.compile_expression(object)?;
                self.compile_expression(index)?;
                self.emit(Opcode::IndexGet, None, 0);
            }
            Expr::Array(elements) => {
                for elem in elements {
                    self.compile_expression(elem)?;
                }
                self.emit(Opcode::BuildArray, Some(elements.len() as u32), 0);
            }
            Expr::Range { start, end } => {
                // Compile range as a special marker: push start, then end, then a range marker
                self.compile_expression(start)?;
                self.compile_expression(end)?;
                // Use BuildArray with count 2 and mark it as range
                // Actually, we'll handle this in for-loop compilation
                // For now, just push start and end as an array with special handling
                self.emit(Opcode::BuildArray, Some(2), 0);
            }
            Expr::Map(entries) => {
                for (key, value) in entries {
                    self.compile_expression(key)?;
                    self.compile_expression(value)?;
                }
                self.emit(Opcode::BuildMap, Some(entries.len() as u32), 0);
            }
            Expr::Tuple(elements) => {
                for elem in elements {
                    self.compile_expression(elem)?;
                }
                self.emit(Opcode::BuildArray, Some(elements.len() as u32), 0);
            }
            Expr::Lambda { params, body } => {
                self.compile_lambda(params, body)?;
            }
            Expr::If {
                condition,
                then_branch,
                else_branch,
            } => {
                self.compile_expression(condition)?;
                let jump_else = self.emit_jump(Opcode::JumpIfFalse, 0);
                self.emit(Opcode::Pop, None, 0);
                self.compile_expression(then_branch)?;
                let jump_end = self.emit_jump(Opcode::Jump, 0);
                self.patch_jump(jump_else)?;
                self.emit(Opcode::Pop, None, 0);
                if let Some(else_expr) = else_branch {
                    self.compile_expression(else_expr)?;
                } else {
                    self.emit_push_constant(Value::Null, 0)?;
                }
                self.patch_jump(jump_end)?;
            }
            Expr::Match { scrutinee, arms } => {
                self.compile_expression(scrutinee)?;
                let mut end_jumps = Vec::new();
                for arm in arms {
                    self.emit(Opcode::Dup, None, 0);
                    self.compile_pattern(&arm.pattern)?;
                    self.emit(Opcode::Eq, None, 0);
                    let jump = self.emit_jump(Opcode::JumpIfFalse, 0);
                    self.emit(Opcode::Pop, None, 0);
                    self.emit(Opcode::Pop, None, 0);
                    self.compile_expression(&arm.body)?;
                    let end_jump = self.emit_jump(Opcode::Jump, 0);
                    end_jumps.push(end_jump);
                    self.patch_jump(jump)?;
                    self.emit(Opcode::Pop, None, 0);
                }
                self.emit_push_constant(Value::Null, 0)?;
                for jump in end_jumps {
                    self.patch_jump(jump)?;
                }
            }
            Expr::Block(stmts) => {
                self.begin_scope();
                for stmt in stmts {
                    self.compile_statement(stmt)?;
                }
                self.end_scope();
            }
            Expr::Assign { target, value } => {
                self.compile_expression(value)?;
                self.compile_assignment_target(target)?;
            }
            Expr::CompoundAssign {
                op,
                target,
                value,
            } => {
                self.compile_expression(target)?;
                self.compile_expression(value)?;
                let opcode = match op {
                    BinaryOp::Add => Opcode::Add,
                    BinaryOp::Sub => Opcode::Sub,
                    BinaryOp::Mul => Opcode::Mul,
                    BinaryOp::Div => Opcode::Div,
                    BinaryOp::Mod => Opcode::Mod,
                    _ => return Err(CompilerError::InvalidAssignmentTarget),
                };
                self.emit(opcode, None, 0);
                self.compile_assignment_target(target)?;
            }
            Expr::Ok(value) => {
                self.compile_expression(value)?;
                self.emit_push_constant(Value::Str("Ok".to_string()), 0)?;
                self.emit(Opcode::BuildArray, Some(2), 0);
            }
            Expr::Err(value) => {
                self.compile_expression(value)?;
                self.emit_push_constant(Value::Str("Err".to_string()), 0)?;
                self.emit(Opcode::BuildArray, Some(2), 0);
            }
            Expr::Some(value) => {
                self.compile_expression(value)?;
                self.emit_push_constant(Value::Str("Some".to_string()), 0)?;
                self.emit(Opcode::BuildArray, Some(2), 0);
            }
            Expr::None => {
                self.emit_push_constant(Value::Null, 0)?;
            }
            Expr::StringInterpolation(parts) => {
                // Compile first part
                if let Some(first) = parts.first() {
                    match first {
                        StringPart::Text(text) => {
                            let index = self.add_constant(Value::Str(text.clone()))?;
                            self.emit(Opcode::Push, Some(index), 0);
                        }
                        StringPart::Expr(expr) => {
                            self.compile_expression(expr)?;
                        }
                    }
                }
                // Compile remaining parts and add each one
                for part in parts.iter().skip(1) {
                    match part {
                        StringPart::Text(text) => {
                            let index = self.add_constant(Value::Str(text.clone()))?;
                            self.emit(Opcode::Push, Some(index), 0);
                        }
                        StringPart::Expr(expr) => {
                            self.compile_expression(expr)?;
                        }
                    }
                    self.emit(Opcode::Add, None, 0);
                }
                // If only one part, convert to string
                if parts.len() == 1 {
                    // Already a string, no need to do anything
                }
            }
        }
        Ok(())
    }

    fn compile_assignment_target(&mut self, target: &Expr) -> Result<(), CompilerError> {
        match target {
            Expr::Identifier(name) => {
                if let Some(local) = self.resolve_local(name) {
                    self.emit(Opcode::StoreLocal, Some(local), 0);
                } else {
                    let index = self.add_constant(Value::Str(name.clone()))?;
                    self.emit(Opcode::StoreGlobal, Some(index), 0);
                }
            }
            Expr::PropertyAccess { object, property } => {
                self.compile_expression(object)?;
                let prop_index = self.add_constant(Value::Str(property.clone()))?;
                self.emit(Opcode::SetProperty, Some(prop_index), 0);
            }
            Expr::Index { object, index } => {
                self.compile_expression(object)?;
                self.compile_expression(index)?;
                self.emit(Opcode::IndexSet, None, 0);
            }
            _ => return Err(CompilerError::InvalidAssignmentTarget),
        }
        Ok(())
    }

    fn compile_pattern(&mut self, pattern: &Pattern) -> Result<(), CompilerError> {
        match pattern {
            Pattern::Literal(lit) => {
                let value = match lit {
                    Literal::Int(n) => Value::Int(*n),
                    Literal::Float(n) => Value::Float(*n),
                    Literal::Bool(b) => Value::Bool(*b),
                    Literal::Str(s) => Value::Str(s.clone()),
                    Literal::Char(c) => Value::Char(*c),
                    Literal::Null => Value::Null,
                };
                let index = self.add_constant(value)?;
                self.emit(Opcode::Push, Some(index), 0);
            }
            Pattern::Identifier(name) => {
                self.define_variable(name)?;
                self.emit_push_constant(Value::Bool(true), 0)?;
            }
            Pattern::Wildcard => {
                self.emit_push_constant(Value::Bool(true), 0)?;
            }
            _ => {
                self.emit_push_constant(Value::Bool(true), 0)?;
            }
        }
        Ok(())
    }

    fn define_variable(&mut self, name: &str) -> Result<(), CompilerError> {
        if self.current.scope_depth > 0 {
            for local in self.current.locals.iter().rev() {
                if local.depth < self.current.scope_depth {
                    break;
                }
                if local.name == name {
                    return Err(CompilerError::DuplicateFunction(name.to_string()));
                }
            }
            if self.current.locals.len() >= 256 {
                return Err(CompilerError::TooManyLocals);
            }
            self.current.locals.push(Local {
                name: name.to_string(),
                depth: self.current.scope_depth,
                captured: false,
            });
        } else {
            let index = self.add_constant(Value::Str(name.to_string()))?;
            self.emit(Opcode::StoreGlobal, Some(index), 0);
        }
        Ok(())
    }

    fn emit_jump(&mut self, opcode: Opcode, target: usize) -> usize {
        let index = self.current_chunk().instructions.len();
        self.emit(opcode, Some(target as u32), 0);
        index
    }

    fn patch_jump(&mut self, index: usize) -> Result<(), CompilerError> {
        let target = self.current_chunk().instructions.len();
        self.patch_jump_at(index, target)
    }

    fn patch_jump_at(&mut self, index: usize, target: usize) -> Result<(), CompilerError> {
        let chunk = &mut self.bytecode.chunks[self.current.chunk_index];
        if index < chunk.instructions.len() {
            chunk.instructions[index].operand = Some(target as u32);
        }
        Ok(())
    }

    fn compile_function(
        &mut self,
        name: &str,
        params: &[Param],
        body: &[Stmt],
    ) -> Result<(), CompilerError> {
        let chunk_index = self.bytecode.chunks.len();
        self.bytecode.chunks.push(Chunk::new(name.to_string()));

        let compiler = FunctionCompiler {
            chunk_index,
            locals: Vec::new(),
            upvalues: Vec::new(),
            scope_depth: 0,
        };
        let old_compiler = std::mem::replace(&mut self.current, compiler);
        self.function_compilers.push(old_compiler);

        self.begin_scope();
        for param in params {
            self.define_variable(&param.name)?;
        }
        for stmt in body {
            self.compile_statement(stmt)?;
        }
        self.emit_push_constant(Value::Null, 0)?;
        self.emit(Opcode::Return, None, 0);

        let upvalues = self.current.upvalues.clone();
        let old_compiler = self.function_compilers.pop().unwrap();
        self.current = old_compiler;

        let func_index = self.add_constant(Value::Function {
            name: name.to_string(),
            arity: params.len(),
            chunk_index,
        })?;
        self.emit(Opcode::Push, Some(func_index), 0);
        self.define_variable(name)?;

        Ok(())
    }

    fn compile_lambda(
        &mut self,
        params: &[Param],
        body: &Expr,
    ) -> Result<(), CompilerError> {
        let chunk_index = self.bytecode.chunks.len();
        self.bytecode.chunks.push(Chunk::new("lambda".to_string()));

        let compiler = FunctionCompiler {
            chunk_index,
            locals: Vec::new(),
            upvalues: Vec::new(),
            scope_depth: 0,
        };
        let old_compiler = std::mem::replace(&mut self.current, compiler);
        self.function_compilers.push(old_compiler);

        self.begin_scope();
        for param in params {
            self.define_variable(&param.name)?;
        }
        self.compile_expression(body)?;
        self.emit(Opcode::Return, None, 0);

        let old_compiler = self.function_compilers.pop().unwrap();
        self.current = old_compiler;

        let func_index = self.add_constant(Value::Function {
            name: "lambda".to_string(),
            arity: params.len(),
            chunk_index,
        })?;
        self.emit(Opcode::Push, Some(func_index), 0);

        Ok(())
    }

    fn compile_class(
        &mut self,
        name: &str,
        superclass: &Option<String>,
        traits: &[String],
        members: &[ClassMember],
    ) -> Result<(), CompilerError> {
        let mut methods = Vec::new();
        for member in members {
            if let ClassMember::Method {
                name, params, body, ..
            } = member
            {
                let chunk_index = self.bytecode.chunks.len();
                self.bytecode.chunks.push(Chunk::new(format!("{}.{}", name, name)));
                methods.push((name.clone(), chunk_index));
            }
        }

        let class_index = self.add_constant(Value::Class {
            name: name.to_string(),
            methods: methods.clone(),
        })?;
        self.emit(Opcode::CreateClass, Some(class_index), 0);
        self.define_variable(name)?;

        Ok(())
    }

    fn compile_trait(
        &mut self,
        name: &str,
        methods: &[TraitMethod],
    ) -> Result<(), CompilerError> {
        Ok(())
    }
}

impl Default for Compiler {
    fn default() -> Self {
        Self::new()
    }
}
