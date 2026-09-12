use std::collections::HashMap;
use std::rc::Rc;
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

/// Info about a class already compiled, used for inheritance and super() calls.
struct CompiledClass {
    name: String,
    superclass: Option<String>,
    /// method name -> (chunk_index, arity including implicit `this`)
    methods: HashMap<String, (usize, usize)>,
    /// (property name, default value)
    properties: Vec<(String, Value)>,
}

/// Stored trait definition used when a class `implements` it.
struct CompiledTrait {
    /// Required method names (must be provided by the implementing class).
    required: Vec<String>,
    /// Default method bodies keyed by method name.
    defaults: HashMap<String, (Vec<Param>, Vec<Stmt>)>,
}

pub struct Compiler {
    bytecode: Bytecode,
    current: FunctionCompiler,
    function_compilers: Vec<FunctionCompiler>,
    globals: HashMap<String, u32>,
    loop_breaks: Vec<Vec<usize>>,
    loop_continues: Vec<Vec<usize>>,
    loop_local_counts: Vec<usize>, // track locals count at loop start for break cleanup
    class_info: HashMap<String, CompiledClass>,
    trait_info: HashMap<String, CompiledTrait>,
    current_class: Option<String>,
    /// Top-level function name -> (chunk_index, arity) for CallDirect.
    known_functions: HashMap<String, (usize, usize)>,
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
            class_info: HashMap::new(),
            trait_info: HashMap::new(),
            current_class: None,
            known_functions: HashMap::new(),
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

    /// Link independently-parsed modules, then the main program, into one Bytecode.
    ///
    /// Each module is compiled as top-level statements (its `fn`/`let`/`const`
    /// become globals). A namespace map `let <mod> = { "export": export, ... }`
    /// is emitted from the module's export table — the importer's AST is not
    /// rewritten with the module body.
    pub fn compile_linked(
        modules: &[(String, Program, Vec<String>)],
        main: &Program,
    ) -> Result<Bytecode, CompilerError> {
        let mut c = Self::new();
        for (name, program, exports) in modules {
            for stmt in &program.statements {
                c.compile_statement(stmt)?;
            }
            c.emit_namespace(name, exports)?;
        }

        let len = main.statements.len();
        for (i, stmt) in main.statements.iter().enumerate() {
            if i == len - 1 {
                if let Stmt::Expression(expr) = stmt {
                    c.compile_expression(expr)?;
                } else {
                    c.compile_statement(stmt)?;
                    c.emit_push_constant(Value::Null, 0)?;
                }
            } else {
                c.compile_statement(stmt)?;
            }
        }
        c.emit(Opcode::Halt, None, 0);
        Ok(c.bytecode)
    }

    /// Emit `let name = { "e1": e1, "e2": e2, ... }` as bytecode (no AST).
    fn emit_namespace(&mut self, name: &str, exports: &[String]) -> Result<(), CompilerError> {
        if exports.is_empty() {
            self.emit_push_constant(Value::Null, 0)?;
            let slot = self.bytecode.intern_global(name);
            self.emit(Opcode::StoreGlobal, Some(slot), 0);
            return Ok(());
        }
        for export in exports {
            let key = self.add_constant(Value::Str(export.as_str().into()))?;
            self.emit(Opcode::Push, Some(key), 0);
            let slot = self.bytecode.intern_global(export);
            self.emit(Opcode::LoadGlobal, Some(slot), 0);
        }
        self.emit(Opcode::BuildMap, Some(exports.len() as u32), 0);
        let ns_slot = self.bytecode.intern_global(name);
        self.emit(Opcode::StoreGlobal, Some(ns_slot), 0);
        Ok(())
    }

    fn emit(&mut self, opcode: Opcode, operand: Option<u32>, line: u32) {
        self.bytecode.chunks[self.current.chunk_index].emit(opcode, operand, line);
    }

    fn emit_push_constant(&mut self, value: Value, line: u32) -> Result<(), CompilerError> {
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
                // Tail-call optimization: `return f(args)` reuses the current frame.
                if let Some(Expr::Call { callee, args }) = value.as_ref() {
                    let is_super = matches!(&**callee, Expr::Identifier(name) if name == "super");
                    if !is_super {
                        self.compile_expression(callee)?;
                        for arg in args {
                            self.compile_expression(arg)?;
                        }
                        self.emit(Opcode::TailCall, Some(args.len() as u32), 0);
                        return Ok(());
                    }
                }
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
                        let len_const = self.add_constant(Value::Str("len".into()))?;
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
                    Literal::Str(s) => Value::Str(s.clone().into()),
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
                    let slot = self.bytecode.intern_global(name);
                    self.emit(Opcode::LoadGlobal, Some(slot), 0);
                }
            }
            Expr::BinaryOp { op, left, right } => {
                // Fused immediate ops: `x - 1`, `n <= 1`, etc.
                if let Expr::Literal(Literal::Int(n)) = &**right {
                    let n = *n;
                    if let Some(imm) = i32::try_from(n).ok() {
                        let fused = match op {
                            BinaryOp::Add => Some(Opcode::AddImm),
                            BinaryOp::Sub => Some(Opcode::SubImm),
                            BinaryOp::Mul => Some(Opcode::MulImm),
                            BinaryOp::Eq => Some(Opcode::EqImm),
                            BinaryOp::Ne => Some(Opcode::NeImm),
                            BinaryOp::Lt => Some(Opcode::LtImm),
                            BinaryOp::Le => Some(Opcode::LeImm),
                            BinaryOp::Gt => Some(Opcode::GtImm),
                            BinaryOp::Ge => Some(Opcode::GeImm),
                            _ => None,
                        };
                        if let Some(opc) = fused {
                            self.compile_expression(left)?;
                            self.emit(opc, Some(imm as u32), 0);
                            return Ok(());
                        }
                    }
                }
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
                // `super(...)` in a method calls the superclass constructor.
                if let Expr::Identifier(name) = &**callee {
                    if name == "super" {
                        return self.compile_super_call(args);
                    }
                    // CallDirect: known top-level function, not shadowed by a local.
                    if self.resolve_local(name).is_none() {
                        if let Some(&(chunk, arity)) = self.known_functions.get(name) {
                            if arity == args.len() && chunk <= 0xFFFF && arity <= 0xFFFF {
                                // Placeholder slot so Return's truncate math stays valid.
                                self.emit_push_constant(Value::Null, 0)?;
                                for arg in args {
                                    self.compile_expression(arg)?;
                                }
                                let packed = ((chunk as u32) << 16) | (arity as u32);
                                self.emit(Opcode::CallDirect, Some(packed), 0);
                                return Ok(());
                            }
                        }
                    }
                }
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
                let method_index = self.add_constant(Value::Str(method.clone().into()))?;
                // Pack arg count in upper 16 bits, method-name constant index in lower 16.
                let packed = ((args.len() as u32) << 16) | (method_index & 0xFFFF);
                self.emit(Opcode::Invoke, Some(packed), 0);
            }
            Expr::PropertyAccess { object, property } => {
                self.compile_expression(object)?;
                let prop_index = self.add_constant(Value::Str(property.clone().into()))?;
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
                    // Pattern test: leaves [scrutinee, match_bool] on stack
                    self.compile_pattern_test(&arm.pattern)?;
                    let jump_next = self.emit_jump(Opcode::JumpIfFalse, 0);
                    // Matched the pattern: pop the bool, leaving [scrutinee]
                    self.emit(Opcode::Pop, None, 0);

                    // Guard: if present, evaluate and test
                    if let Some(guard) = &arm.guard {
                        self.compile_expression(guard)?;
                        let jump_guard_fail = self.emit_jump(Opcode::JumpIfFalse, 0);
                        self.emit(Opcode::Pop, None, 0);
                        // Guard passed — fall through to body
                        // Compile body with scrutinee popped
                        self.emit(Opcode::Pop, None, 0);
                        self.compile_expression(&arm.body)?;
                        let end_jump = self.emit_jump(Opcode::Jump, 0);
                        end_jumps.push(end_jump);
                        // Guard failed path
                        self.patch_jump(jump_guard_fail)?;
                        self.emit(Opcode::Pop, None, 0);
                        // Continue to next arm with [scrutinee]
                    } else {
                        // No guard — compile body with scrutinee popped
                        self.emit(Opcode::Pop, None, 0);
                        self.compile_expression(&arm.body)?;
                        let end_jump = self.emit_jump(Opcode::Jump, 0);
                        end_jumps.push(end_jump);
                    }

                    // Pattern didn't match: pop the bool, leaving [scrutinee]
                    self.patch_jump(jump_next)?;
                    self.emit(Opcode::Pop, None, 0);
                }
                // No arm matched: pop scrutinee, push null
                self.emit(Opcode::Pop, None, 0);
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
                match &**target {
                    Expr::PropertyAccess { object, property } => {
                        // Stack order for SetProperty: [object, value] (value on top)
                        self.compile_expression(object)?;
                        self.compile_expression(value)?;
                        let prop_index = self.add_constant(Value::Str(property.clone().into()))?;
                        self.emit(Opcode::SetProperty, Some(prop_index), 0);
                        // SetProperty leaves the new object on stack; store it back
                        if let Expr::Identifier(name) = &**object {
                            if let Some(local) = self.resolve_local(name) {
                                self.emit(Opcode::StoreLocal, Some(local), 0);
                            } else {
                                let slot = self.bytecode.intern_global(name);
                                self.emit(Opcode::StoreGlobal, Some(slot), 0);
                            }
                        } else {
                            self.emit(Opcode::Pop, None, 0);
                        }
                    }
                    Expr::Index { object, index } => {
                        self.compile_expression(object)?;
                        self.compile_expression(index)?;
                        self.compile_expression(value)?;
                        self.emit(Opcode::IndexSet, None, 0);
                        self.emit(Opcode::Pop, None, 0);
                    }
                    _ => {
                        self.compile_expression(value)?;
                        self.compile_assignment_target(target)?;
                    }
                }
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
                self.emit(Opcode::WrapOk, None, 0);
            }
            Expr::Err(value) => {
                self.compile_expression(value)?;
                self.emit(Opcode::WrapErr, None, 0);
            }
            Expr::Some(value) => {
                self.compile_expression(value)?;
                self.emit(Opcode::WrapSome, None, 0);
            }
            Expr::None => {
                self.emit_push_constant(Value::Null, 0)?;
            }
            Expr::StringInterpolation(parts) => {
                // Compile first part
                if let Some(first) = parts.first() {
                    match first {
                        StringPart::Text(text) => {
                            let index = self.add_constant(Value::Str(text.clone().into()))?;
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
                            let index = self.add_constant(Value::Str(text.clone().into()))?;
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
                } else if let Some(upvalue) = self.resolve_upvalue(name) {
                    self.emit(Opcode::StoreUpvalue, Some(upvalue), 0);
                } else {
                    let slot = self.bytecode.intern_global(name);
                    self.emit(Opcode::StoreGlobal, Some(slot), 0);
                }
            }
            Expr::PropertyAccess { object, property } => {
                self.compile_expression(object)?;
                let prop_index = self.add_constant(Value::Str(property.clone().into()))?;
                self.emit(Opcode::SetProperty, Some(prop_index), 0);
                // SetProperty leaves the (possibly new) object on the stack.
                // Store it back into the binding so mutations stick for locals/globals.
                match &**object {
                    Expr::Identifier(name) => {
                        if let Some(local) = self.resolve_local(name) {
                            self.emit(Opcode::StoreLocal, Some(local), 0);
                        } else {
                            let slot = self.bytecode.intern_global(name);
                            self.emit(Opcode::StoreGlobal, Some(slot), 0);
                        }
                    }
                    _ => {
                        // Cannot write back through a complex lvalue; drop the result.
                        self.emit(Opcode::Pop, None, 0);
                    }
                }
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

    /// Compile a pattern test.
    /// Precondition: scrutinee is on top of the stack.
    /// Postcondition: [scrutinee, match_bool] — the scrutinee is preserved,
    /// and a boolean indicating whether the pattern matched is pushed on top.
    fn compile_pattern_test(&mut self, pattern: &Pattern) -> Result<(), CompilerError> {
        match pattern {
            Pattern::Literal(lit) => {
                let value = match lit {
                    Literal::Int(n) => Value::Int(*n),
                    Literal::Float(n) => Value::Float(*n),
                    Literal::Bool(b) => Value::Bool(*b),
                    Literal::Str(s) => Value::Str(s.clone().into()),
                    Literal::Char(c) => Value::Char(*c),
                    Literal::Null => Value::Null,
                };
                // [s] -> [s, s] -> [s, s, lit] -> [s, bool]
                self.emit(Opcode::Dup, None, 0);
                let index = self.add_constant(value)?;
                self.emit(Opcode::Push, Some(index), 0);
                self.emit(Opcode::Eq, None, 0);
            }
            Pattern::Wildcard => {
                // Always matches: [s] -> [s, true]
                self.emit_push_constant(Value::Bool(true), 0)?;
            }
            Pattern::Identifier(name) => {
                // Always matches AND binds the scrutinee to the variable.
                // [s] -> [s, s] -> [s] (one copy stored) -> [s, true]
                self.emit(Opcode::Dup, None, 0);
                let slot = self.bytecode.intern_global(name);
                self.emit(Opcode::StoreGlobal, Some(slot), 0);
                self.emit_push_constant(Value::Bool(true), 0)?;
            }
            Pattern::Or(patterns) => {
                // Match if ANY sub-pattern matches.
                // Uses short-circuit: test each pattern, JumpIfTrue to "matched".
                if patterns.is_empty() {
                    self.emit_push_constant(Value::Bool(false), 0)?;
                    return Ok(());
                }
                let mut or_true_jumps = Vec::new();
                let last = patterns.len() - 1;
                for (i, sub) in patterns.iter().enumerate() {
                    // [s] -> test sub-pattern -> [s, bool]
                    self.compile_pattern_test(sub)?;
                    if i < last {
                        // If true, jump to or_matched (bool stays on stack via peek)
                        let j = self.emit_jump(Opcode::JumpIfTrue, 0);
                        or_true_jumps.push(j);
                        // Not matched: pop the false, try next sub-pattern
                        self.emit(Opcode::Pop, None, 0);
                    }
                    // Last sub-pattern: leave [s, bool] as the Or result
                }
                let jump_end = self.emit_jump(Opcode::Jump, 0);
                // or_matched: stack is [s, true] (JumpIfTrue peeks)
                for j in or_true_jumps {
                    self.patch_jump(j)?;
                }
                self.patch_jump(jump_end)?;
            }
            Pattern::Guard(inner, guard_expr) => {
                // Test inner pattern, then evaluate guard as the result.
                self.compile_pattern_test(inner)?;
                // [s, inner_bool]
                let jump_fail = self.emit_jump(Opcode::JumpIfFalse, 0);
                self.emit(Opcode::Pop, None, 0);
                // Inner matched — evaluate guard; its result is the overall result.
                self.compile_expression(guard_expr)?;
                // [s, guard_bool]
                let jump_end = self.emit_jump(Opcode::Jump, 0);
                // Inner didn't match
                self.patch_jump(jump_fail)?;
                self.emit(Opcode::Pop, None, 0);
                self.emit_push_constant(Value::Bool(false), 0)?;
                self.patch_jump(jump_end)?;
            }
            Pattern::Range(start_expr, end_expr, inclusive) => {
                // Stack on entry: [scrutinee]
                // Test: start <= scrutinee && (scrutinee < end  or  scrutinee <= end)
                // Result: [scrutinee, bool]

                // Test 1: scrutinee >= start
                self.emit(Opcode::Dup, None, 0);           // [scrutinee, scrutinee]
                self.compile_expression(start_expr)?;       // [scrutinee, scrutinee, start]
                self.emit(Opcode::Ge, None, 0);            // [scrutinee, bool1]
                let jump_fail = self.emit_jump(Opcode::JumpIfFalse, 0); // peek bool1

                // bool1 is true: pop it, test second condition
                self.emit(Opcode::Pop, None, 0);           // [scrutinee]
                self.emit(Opcode::Dup, None, 0);           // [scrutinee, scrutinee]
                self.compile_expression(end_expr)?;         // [scrutinee, scrutinee, end]
                if *inclusive {
                    self.emit(Opcode::Le, None, 0);        // [scrutinee, bool2]
                } else {
                    self.emit(Opcode::Lt, None, 0);        // [scrutinee, bool2]
                }
                let jump_done = self.emit_jump(Opcode::Jump, 0);

                // Fail path: [scrutinee, false] already (JumpIfFalse peeks)
                self.patch_jump(jump_fail)?;

                self.patch_jump(jump_done)?;
            }
            Pattern::Tuple(_) | Pattern::Array(_) => {
                // Not yet fully implemented — always match for now
                self.emit_push_constant(Value::Bool(true), 0)?;
            }
            Pattern::Ok(inner) => self.compile_variant_pattern(Opcode::UnwrapOk, inner)?,
            Pattern::Err(inner) => self.compile_variant_pattern(Opcode::UnwrapErr, inner)?,
            Pattern::Some(inner) => self.compile_variant_pattern(Opcode::UnwrapSome, inner)?,
        }
        Ok(())
    }

    /// Compile `Ok(pat)` / `Err(pat)` / `Some(pat)`.
    /// Unwrap* leaves [inner, true] on match, or [s, false] on failure.
    fn compile_variant_pattern(
        &mut self,
        unwrap: Opcode,
        inner: &Pattern,
    ) -> Result<(), CompilerError> {
        // [s]
        self.emit(Opcode::Dup, None, 0); // [s, s]
        self.emit(unwrap, None, 0); // [s, inner, true] or [s, false]

        let jump_fail = self.emit_jump(Opcode::JumpIfFalse, 0);
        // Matched: [s, inner, true]
        self.emit(Opcode::Pop, None, 0); // [s, inner]

        match inner {
            Pattern::Wildcard => {
                self.emit(Opcode::Pop, None, 0); // [s]
            }
            Pattern::Identifier(name) => {
                let slot = self.bytecode.intern_global(name);
                self.emit(Opcode::StoreGlobal, Some(slot), 0); // [s]
            }
            Pattern::Literal(lit) => {
                let value = match lit {
                    Literal::Int(n) => Value::Int(*n),
                    Literal::Float(n) => Value::Float(*n),
                    Literal::Bool(b) => Value::Bool(*b),
                    Literal::Str(s) => Value::Str(s.clone().into()),
                    Literal::Char(c) => Value::Char(*c),
                    Literal::Null => Value::Null,
                };
                let index = self.add_constant(value)?;
                self.emit(Opcode::Push, Some(index), 0); // [s, inner, lit]
                self.emit(Opcode::Eq, None, 0); // [s, bool]
                // If inner equality fails, overall match fails (bool already on stack).
                let jump_end = self.emit_jump(Opcode::Jump, 0);
                self.patch_jump(jump_fail)?;
                self.emit(Opcode::Pop, None, 0); // pop false from unwrap
                self.emit_push_constant(Value::Bool(false), 0)?;
                self.patch_jump(jump_end)?;
                return Ok(());
            }
            other => {
                // Nested patterns: recurse on the unwrapped value.
                // Stack is [s, inner]; compile nested test -> [s, inner, bool]
                self.compile_pattern_test(other)?;
                let jump_nested_fail = self.emit_jump(Opcode::JumpIfFalse, 0);
                self.emit(Opcode::Pop, None, 0); // pop bool
                self.emit(Opcode::Pop, None, 0); // pop inner
                self.emit_push_constant(Value::Bool(true), 0)?;
                let jump_end = self.emit_jump(Opcode::Jump, 0);
                self.patch_jump(jump_nested_fail)?;
                self.emit(Opcode::Pop, None, 0); // pop bool
                self.emit(Opcode::Pop, None, 0); // pop inner
                self.emit_push_constant(Value::Bool(false), 0)?;
                self.patch_jump(jump_end)?;

                let jump_outer_end = self.emit_jump(Opcode::Jump, 0);
                self.patch_jump(jump_fail)?;
                self.emit(Opcode::Pop, None, 0);
                self.emit_push_constant(Value::Bool(false), 0)?;
                self.patch_jump(jump_outer_end)?;
                return Ok(());
            }
        }

        self.emit_push_constant(Value::Bool(true), 0)?;
        let jump_end = self.emit_jump(Opcode::Jump, 0);
        self.patch_jump(jump_fail)?;
        // Failed: [s, false] already
        self.patch_jump(jump_end)?;
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
                name: name.into(),
                depth: self.current.scope_depth,
                captured: false,
            });
        } else {
            let slot = self.bytecode.intern_global(name);
            self.emit(Opcode::StoreGlobal, Some(slot), 0);
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
        // Register before compiling the body so recursive calls can CallDirect.
        self.known_functions
            .insert(name.to_string(), (chunk_index, params.len()));

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

        let func_index = self.add_constant(Value::Function(Rc::new(FunctionObj {
            name: name.into(),
            arity: params.len(),
            chunk_index,
        })))?;
        self.emit(Opcode::Push, Some(func_index), 0);

        // Capture upvalues if the function closes over outer variables
        if !upvalues.is_empty() {
            for uv in &upvalues {
                if uv.is_local {
                    self.emit(Opcode::LoadLocal, Some(uv.index as u32), 0);
                } else {
                    self.emit(Opcode::LoadUpvalue, Some(uv.index as u32), 0);
                }
            }
            self.emit(Opcode::MakeClosure, Some(upvalues.len() as u32), 0);
        }

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
        match body {
            Expr::Block(stmts) => {
                for stmt in stmts {
                    self.compile_statement(stmt)?;
                }
                // If no explicit return executed, return null
                self.emit_push_constant(Value::Null, 0)?;
            }
            _ => {
                self.compile_expression(body)?;
            }
        }
        self.emit(Opcode::Return, None, 0);

        let upvalues = self.current.upvalues.clone();
        let old_compiler = self.function_compilers.pop().unwrap();
        self.current = old_compiler;

        let func_index = self.add_constant(Value::Function(Rc::new(FunctionObj {
            name: "lambda".into(),
            arity: params.len(),
            chunk_index,
        })))?;
        self.emit(Opcode::Push, Some(func_index), 0);

        // Capture upvalues if the lambda closes over outer variables
        if !upvalues.is_empty() {
            for uv in &upvalues {
                if uv.is_local {
                    self.emit(Opcode::LoadLocal, Some(uv.index as u32), 0);
                } else {
                    self.emit(Opcode::LoadUpvalue, Some(uv.index as u32), 0);
                }
            }
            self.emit(Opcode::MakeClosure, Some(upvalues.len() as u32), 0);
        }

        Ok(())
    }

    fn compile_class(
        &mut self,
        name: &str,
        superclass: &Option<String>,
        traits: &[String],
        members: &[ClassMember],
    ) -> Result<(), CompilerError> {
        let mut methods: Vec<(String, usize)> = Vec::new();
        let mut method_info: HashMap<String, (usize, usize)> = HashMap::new();
        let mut properties: Vec<(String, Value)> = Vec::new();

        // Collect property declarations (with literal defaults when available).
        for member in members {
            if let ClassMember::Property {
                name: prop_name,
                default,
                ..
            } = member
            {
                let default_value = match default {
                    Some(Expr::Literal(lit)) => match lit {
                        Literal::Int(n) => Value::Int(*n),
                        Literal::Float(n) => Value::Float(*n),
                        Literal::Bool(b) => Value::Bool(*b),
                        Literal::Str(s) => Value::Str(s.clone().into()),
                        Literal::Char(c) => Value::Char(*c),
                        Literal::Null => Value::Null,
                    },
                    _ => Value::Null,
                };
                properties.push((prop_name.clone(), default_value));
            }
        }

        // Register class info BEFORE compiling methods so `super()` can find the superclass.
        self.class_info.insert(
            name.to_string(),
            CompiledClass {
                name: name.into(),
                superclass: superclass.clone(),
                methods: HashMap::new(),
                properties: properties.clone(),
            },
        );

        // Class-provided method names (used for trait checks / default injection).
        let mut declared_methods: HashMap<String, (Vec<Param>, Vec<Stmt>)> = HashMap::new();
        for member in members {
            match member {
                ClassMember::Method {
                    name: m_name,
                    params,
                    body,
                    ..
                } => {
                    declared_methods.insert(m_name.clone(), (params.clone(), body.clone()));
                }
                ClassMember::Constructor { params, body, .. } => {
                    declared_methods.insert("init".to_string(), (params.clone(), body.clone()));
                }
                ClassMember::Property { .. } => {}
            }
        }

        // Validate implemented traits and collect default methods to inject.
        let mut injected_defaults: Vec<(String, Vec<Param>, Vec<Stmt>)> = Vec::new();
        for trait_name in traits {
            let info = self.trait_info.get(trait_name).ok_or_else(|| {
                CompilerError::Internal(format!("Unknown trait '{}'", trait_name))
            })?;
            for req in &info.required {
                if !declared_methods.contains_key(req) {
                    return Err(CompilerError::Internal(format!(
                        "Class '{}' does not implement required method '{}' from trait '{}'",
                        name, req, trait_name
                    )));
                }
            }
            for (m_name, (params, body)) in &info.defaults {
                if !declared_methods.contains_key(m_name) {
                    injected_defaults.push((m_name.clone(), params.clone(), body.clone()));
                }
            }
        }

        // Compile each method body into its own chunk. `this` is implicit local 0.
        let prev_class = self.current_class.replace(name.to_string());
        for member in members {
            let (method_name, params, body) = match member {
                ClassMember::Method {
                    name: m_name,
                    params,
                    body,
                    ..
                } => (m_name.clone(), params.clone(), body.clone()),
                ClassMember::Constructor {
                    params,
                    super_args,
                    body,
                } => {
                    let mut full_body = Vec::new();
                    if let Some(sargs) = super_args {
                        // Prepend `super(sargs...)` so the parent constructor runs first.
                        full_body.push(Stmt::Expression(Expr::Call {
                            callee: Box::new(Expr::Identifier("super".to_string())),
                            args: sargs.clone(),
                        }));
                    }
                    full_body.extend(body.iter().cloned());
                    ("init".to_string(), params.clone(), full_body)
                }
                ClassMember::Property { .. } => continue,
            };

            let (chunk_index, arity) =
                self.compile_method(name, &method_name, &params, &body)?;
            methods.push((method_name.clone(), chunk_index));
            method_info.insert(method_name, (chunk_index, arity));
        }

        // Inject trait default methods not overridden by the class.
        for (method_name, params, body) in injected_defaults {
            let (chunk_index, arity) =
                self.compile_method(name, &method_name, &params, &body)?;
            methods.push((method_name.clone(), chunk_index));
            method_info.insert(method_name, (chunk_index, arity));
        }

        self.current_class = prev_class;

        // Update class metadata with compiled method info.
        if let Some(ci) = self.class_info.get_mut(name) {
            ci.methods = method_info;
        }

        let class_const = self.add_constant(Value::Class(Rc::new(ClassObj {
            name: name.into(),
            methods: Rc::new(methods.clone()),
            superclass: superclass.as_ref().map(|s| Rc::from(s.as_str())),
            properties: Rc::new(properties),
        })))?;
        self.emit(Opcode::CreateClass, Some(class_const), 0);
        self.define_variable(name)?;

        Ok(())
    }

    /// Compile a method body into a fresh chunk. Local 0 is always `this`.
    /// Returns (chunk_index, arity including `this`).
    fn compile_method(
        &mut self,
        class_name: &str,
        method_name: &str,
        params: &[Param],
        body: &[Stmt],
    ) -> Result<(usize, usize), CompilerError> {
        let chunk_name = format!("{}.{}", class_name, method_name);
        let chunk_index = self.bytecode.chunks.len();
        self.bytecode.chunks.push(Chunk::new(chunk_name));

        let compiler = FunctionCompiler {
            chunk_index,
            locals: Vec::new(),
            upvalues: Vec::new(),
            scope_depth: 0,
        };
        let old_compiler = std::mem::replace(&mut self.current, compiler);
        self.function_compilers.push(old_compiler);

        self.begin_scope();
        // Implicit receiver is always local 0.
        self.define_variable("this")?;
        for param in params {
            self.define_variable(&param.name)?;
        }
        for stmt in body {
            self.compile_statement(stmt)?;
        }

        // Constructors return `this` so `let x = Foo(...)` yields the instance.
        if method_name == "init" {
            self.emit(Opcode::LoadLocal, Some(0), 0);
        } else {
            self.emit_push_constant(Value::Null, 0)?;
        }
        self.emit(Opcode::Return, None, 0);

        self.current = self.function_compilers.pop().unwrap();

        let arity = params.len() + 1; // + implicit `this`
        Ok((chunk_index, arity))
    }

    /// Compile `super(args...)`: call the superclass `init` with the current `this`,
    /// then store the returned (updated) instance back into local 0.
    fn compile_super_call(&mut self, args: &[Expr]) -> Result<(), CompilerError> {
        let current_name = self
            .current_class
            .clone()
            .ok_or_else(|| CompilerError::Internal("`super` used outside of a class".to_string()))?;
        let parent_name = self
            .class_info
            .get(&current_name)
            .and_then(|c| c.superclass.clone())
            .ok_or_else(|| {
                CompilerError::Internal(format!(
                    "`super` used in class '{}' which has no superclass",
                    current_name
                ))
            })?;
        let (init_chunk, init_arity) = self
            .class_info
            .get(&parent_name)
            .and_then(|p| p.methods.get("init").copied())
            .ok_or_else(|| {
                CompilerError::Internal(format!(
                    "superclass '{}' has no `init` method",
                    parent_name
                ))
            })?;

        // Push parent init as a Function value, then `this` + user args.
        self.emit_push_constant(
            Value::Function(Rc::new(FunctionObj {
                name: format!("{}.init", parent_name).into(),
                arity: init_arity,
                chunk_index: init_chunk,
            })),
            0,
        )?;
        self.emit(Opcode::LoadLocal, Some(0), 0);
        for arg in args {
            self.compile_expression(arg)?;
        }
        self.emit(Opcode::Call, Some((args.len() + 1) as u32), 0);
        // Parent init returns the updated instance; store it back as `this`.
        self.emit(Opcode::StoreLocal, Some(0), 0);
        // Leave a value so an enclosing expression-statement Pop stays balanced.
        self.emit_push_constant(Value::Null, 0)?;
        Ok(())
    }

    fn compile_trait(
        &mut self,
        name: &str,
        methods: &[TraitMethod],
    ) -> Result<(), CompilerError> {
        if self.trait_info.contains_key(name) {
            return Err(CompilerError::DuplicateTrait(name.to_string()));
        }
        let mut required = Vec::new();
        let mut defaults = HashMap::new();
        for method in methods {
            match method {
                TraitMethod::Required { name: m_name, .. } => {
                    required.push(m_name.clone());
                }
                TraitMethod::Default {
                    name: m_name,
                    params,
                    body,
                    ..
                } => {
                    defaults.insert(m_name.clone(), (params.clone(), body.clone()));
                }
            }
        }
        self.trait_info.insert(
            name.to_string(),
            CompiledTrait {
                required,
                defaults,
            },
        );
        Ok(())
    }
}

impl Default for Compiler {
    fn default() -> Self {
        Self::new()
    }
}
