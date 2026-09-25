use crate::error::CompilerError;
use mailang_ast::*;
use mailang_bytecode::*;
use std::collections::HashMap;
use std::rc::Rc;

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
#[allow(dead_code)]
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

/// Generic function definition kept for monomorphization.
#[derive(Clone)]
struct GenericFunction {
    type_params: Vec<String>,
    params: Vec<Param>,
    body: Vec<Stmt>,
}

pub struct Compiler {
    bytecode: Bytecode,
    current: FunctionCompiler,
    function_compilers: Vec<FunctionCompiler>,
    #[allow(dead_code)] // interned names live in bytecode.global_names
    globals: HashMap<String, u32>,
    loop_breaks: Vec<Vec<usize>>,
    loop_continues: Vec<Vec<usize>>,
    loop_local_counts: Vec<usize>, // track locals count at loop start for break cleanup
    class_info: HashMap<String, CompiledClass>,
    trait_info: HashMap<String, CompiledTrait>,
    current_class: Option<String>,
    /// Top-level function name -> (chunk_index, arity) for CallDirect.
    known_functions: HashMap<String, (usize, usize)>,
    /// Generic function templates, keyed by source name.
    generic_functions: HashMap<String, GenericFunction>,
    /// Cached monomorphizations: specialized name -> (chunk_index, arity).
    specializations: HashMap<String, (usize, usize)>,
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
            generic_functions: HashMap::new(),
            specializations: HashMap::new(),
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

    #[allow(dead_code)]
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
                self.emit(Opcode::Pop, None, 0);
                self.current.locals.pop();
                count += 1;
            } else {
                break;
            }
        }
        count
    }

    fn resolve_local(&self, name: &str) -> Option<u32> {
        // `self` is an alias for the implicit receiver (`this`, local 0).
        let name = if name == "self" { "this" } else { name };
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
            return self.add_upvalue(local, true);
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
                mutable: _,
                value,
                pattern,
                ..
            } => {
                if let Some(pat) = pattern {
                    if let Some(val) = value {
                        self.compile_expression(val)?;
                    } else {
                        self.emit_push_constant(Value::Null, 0)?;
                    }
                    let n = self.compile_bindings_from_stack(pat)?;
                    // Bindings are already named locals (or discarded).
                    let _ = n;
                } else {
                    if let Some(val) = value {
                        self.compile_expression(val)?;
                    } else {
                        self.emit_push_constant(Value::Null, 0)?;
                    }
                    self.define_variable(name)?;
                }
            }
            Stmt::Const { name, value, .. } => {
                self.compile_expression(value)?;
                self.define_variable(name)?;
            }
            Stmt::FunctionDef {
                name,
                type_params,
                params,
                return_type: _,
                body,
            } => {
                if !type_params.is_empty() {
                    // Keep the template for monomorphization; also emit the
                    // type-erased body so non-turbofish calls still work.
                    self.generic_functions.insert(
                        name.clone(),
                        GenericFunction {
                            type_params: type_params.clone(),
                            params: params.to_vec(),
                            body: body.to_vec(),
                        },
                    );
                }
                self.compile_function(name, params, body)?;
            }
            Stmt::ClassDef {
                name,
                type_params,
                superclass,
                traits,
                members,
            } => {
                // Generic classes: one erased layout (fields dynamic). Field
                // type params are checked by the analyzer but not specialized.
                let _ = type_params;
                self.compile_class(name, superclass, traits, members)?;
            }
            Stmt::TraitDef {
                name,
                methods,
                supertraits,
            } => {
                self.compile_trait(name, supertraits, methods)?;
            }
            Stmt::ModuleDef { name: _, body } => {
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
                    _ => {
                        self.emit(Opcode::Pop, None, 0);
                    }
                }
            }
            Stmt::Return(value) => {
                // Tail-call optimization: `return f(args)` reuses the current frame.
                // Skip TCO when turbofish is present so monomorphization applies.
                if let Some(Expr::Call {
                    callee,
                    args,
                    type_args,
                }) = value.as_ref()
                {
                    let is_super = matches!(&**callee, Expr::Identifier(name) if name == "super");
                    if !is_super && type_args.is_empty() {
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

                    let _jump_end = self.emit_jump(Opcode::Jump, 0);
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
                    if let Ok(imm) = i32::try_from(*n) {
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
            Expr::Call {
                callee,
                args,
                type_args,
            } => {
                // `super(...)` in a method calls the superclass constructor.
                if let Expr::Identifier(name) = &**callee {
                    if name == "super" {
                        return self.compile_super_call(args);
                    }
                    // Monomorphize generic calls with known concrete type args.
                    if self.resolve_local(name).is_none() {
                        if let Some(spec) = self.resolve_specialization(name, args, type_args)? {
                            for arg in args {
                                self.compile_expression(arg)?;
                            }
                            let packed = ((spec.0 as u32) << 16) | (spec.1 as u32);
                            self.emit(Opcode::CallDirect, Some(packed), 0);
                            return Ok(());
                        }
                        // CallDirect: known top-level function, not shadowed by a local.
                        if let Some(&(chunk, arity)) = self.known_functions.get(name) {
                            if arity == args.len() && chunk <= 0xFFFF && arity <= 0xFFFF {
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
                // `super.method(...)` invokes the superclass implementation.
                if let Expr::Identifier(obj_name) = &**object {
                    if obj_name == "super" {
                        return self.compile_super_method_call(method, args);
                    }
                }
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
                self.begin_scope();
                self.compile_expression(scrutinee)?;
                let scrut_slot = self.current.locals.len() as u32;
                self.current.locals.push(Local {
                    name: "$match".into(),
                    depth: self.current.scope_depth,
                    captured: false,
                });

                let mut end_jumps = Vec::new();
                for arm in arms {
                    self.begin_scope();
                    self.compile_pattern_test(&arm.pattern)?;
                    let jump_next = self.emit_jump(Opcode::JumpIfFalse, 0);
                    self.emit(Opcode::Pop, None, 0);

                    let n_bind = self.compile_pattern_bindings(&arm.pattern, scrut_slot)?;

                    if let Some(guard) = &arm.guard {
                        self.compile_expression(guard)?;
                        let jump_guard_fail = self.emit_jump(Opcode::JumpIfFalse, 0);
                        self.emit(Opcode::Pop, None, 0);
                        self.compile_expression(&arm.body)?;
                        self.end_scope_keeping_result(n_bind);
                        end_jumps.push(self.emit_jump(Opcode::Jump, 0));
                        self.patch_jump(jump_guard_fail)?;
                        self.emit(Opcode::Pop, None, 0);
                        for _ in 0..n_bind {
                            self.emit(Opcode::Pop, None, 0);
                        }
                        for _ in 0..n_bind {
                            self.current.locals.pop();
                        }
                        self.current.scope_depth = self.current.scope_depth.saturating_sub(1);
                    } else {
                        self.compile_expression(&arm.body)?;
                        self.end_scope_keeping_result(n_bind);
                        end_jumps.push(self.emit_jump(Opcode::Jump, 0));
                    }

                    self.patch_jump(jump_next)?;
                    self.emit(Opcode::Pop, None, 0);
                    self.current.scope_depth = self.current.scope_depth.saturating_sub(1);
                }
                self.emit_push_constant(Value::Null, 0)?;
                self.end_scope_keeping_result(1);
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
            Expr::Assign { target, value } => match &**target {
                Expr::PropertyAccess { object, property } => {
                    self.compile_expression(object)?;
                    self.compile_expression(value)?;
                    let prop_index = self.add_constant(Value::Str(property.clone().into()))?;
                    self.emit(Opcode::SetProperty, Some(prop_index), 0);
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
            },
            Expr::CompoundAssign { op, target, value } => {
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
            Expr::Try(inner) => {
                self.compile_expression(inner)?;
                self.emit(Opcode::Try, None, 0);
            }
            Expr::StringInterpolation(parts) => {
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

    /// Structural pattern test (no bindings). Leaves [scrutinee, bool].
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
                self.emit(Opcode::Dup, None, 0);
                let index = self.add_constant(value)?;
                self.emit(Opcode::Push, Some(index), 0);
                self.emit(Opcode::Eq, None, 0);
            }
            Pattern::Wildcard | Pattern::Identifier(_) => {
                self.emit_push_constant(Value::Bool(true), 0)?;
            }
            Pattern::Or(patterns) => {
                if patterns.is_empty() {
                    self.emit_push_constant(Value::Bool(false), 0)?;
                    return Ok(());
                }
                let mut or_true_jumps = Vec::new();
                let last = patterns.len() - 1;
                for (i, sub) in patterns.iter().enumerate() {
                    self.compile_pattern_test(sub)?;
                    if i < last {
                        let j = self.emit_jump(Opcode::JumpIfTrue, 0);
                        or_true_jumps.push(j);
                        self.emit(Opcode::Pop, None, 0);
                    }
                }
                let jump_end = self.emit_jump(Opcode::Jump, 0);
                for j in or_true_jumps {
                    self.patch_jump(j)?;
                }
                self.patch_jump(jump_end)?;
            }
            Pattern::Guard(inner, guard_expr) => {
                self.compile_pattern_test(inner)?;
                let jump_fail = self.emit_jump(Opcode::JumpIfFalse, 0);
                self.emit(Opcode::Pop, None, 0);
                self.compile_expression(guard_expr)?;
                let jump_end = self.emit_jump(Opcode::Jump, 0);
                self.patch_jump(jump_fail)?;
                self.emit(Opcode::Pop, None, 0);
                self.emit_push_constant(Value::Bool(false), 0)?;
                self.patch_jump(jump_end)?;
            }
            Pattern::Range(start_expr, end_expr, inclusive) => {
                self.emit(Opcode::Dup, None, 0);
                self.compile_expression(start_expr)?;
                self.emit(Opcode::Ge, None, 0);
                let jump_fail = self.emit_jump(Opcode::JumpIfFalse, 0);
                self.emit(Opcode::Pop, None, 0);
                self.emit(Opcode::Dup, None, 0);
                self.compile_expression(end_expr)?;
                if *inclusive {
                    self.emit(Opcode::Le, None, 0);
                } else {
                    self.emit(Opcode::Lt, None, 0);
                }
                let jump_done = self.emit_jump(Opcode::Jump, 0);
                self.patch_jump(jump_fail)?;
                self.patch_jump(jump_done)?;
            }
            Pattern::Tuple(elems) | Pattern::Array(elems) => {
                self.compile_seq_pattern_test(elems)?;
            }
            Pattern::Ok(inner) => self.compile_variant_test(Opcode::UnwrapOk, inner)?,
            Pattern::Err(inner) => self.compile_variant_test(Opcode::UnwrapErr, inner)?,
            Pattern::Some(inner) => self.compile_variant_test(Opcode::UnwrapSome, inner)?,
        }
        Ok(())
    }

    fn compile_seq_pattern_test(&mut self, elems: &[Pattern]) -> Result<(), CompilerError> {
        let n = elems.len() as i64;
        self.emit(Opcode::Dup, None, 0);
        let len_name = self.add_constant(Value::Str("len".into()))?;
        self.emit(Opcode::GetProperty, Some(len_name), 0);
        self.emit_push_constant(Value::Int(n), 0)?;
        self.emit(Opcode::Eq, None, 0);
        let jump_fail_len = self.emit_jump(Opcode::JumpIfFalse, 0);
        self.emit(Opcode::Pop, None, 0);

        let mut elem_fail_jumps = Vec::new();
        for (i, sub) in elems.iter().enumerate() {
            if matches!(sub, Pattern::Wildcard | Pattern::Identifier(_)) {
                continue;
            }
            self.emit(Opcode::Dup, None, 0);
            self.emit_push_constant(Value::Int(i as i64), 0)?;
            self.emit(Opcode::IndexGet, None, 0);
            self.compile_pattern_test(sub)?;
            let j = self.emit_jump(Opcode::JumpIfFalse, 0);
            elem_fail_jumps.push(j);
            self.emit(Opcode::Pop, None, 0);
            self.emit(Opcode::Pop, None, 0);
        }
        self.emit_push_constant(Value::Bool(true), 0)?;
        let mut to_end = Vec::new();
        to_end.push(self.emit_jump(Opcode::Jump, 0));

        // fail_len: stack is [s, false]
        self.patch_jump(jump_fail_len)?;
        to_end.push(self.emit_jump(Opcode::Jump, 0));

        // fail_elem: stack is [s, elem, false]
        for j in elem_fail_jumps {
            self.patch_jump(j)?;
            self.emit(Opcode::Pop, None, 0);
            self.emit(Opcode::Pop, None, 0);
            self.emit_push_constant(Value::Bool(false), 0)?;
            to_end.push(self.emit_jump(Opcode::Jump, 0));
        }

        let jump_end = self.current_chunk().instructions.len();
        for j in to_end {
            self.patch_jump_at(j, jump_end)?;
        }
        Ok(())
    }

    fn compile_variant_test(
        &mut self,
        unwrap: Opcode,
        inner: &Pattern,
    ) -> Result<(), CompilerError> {
        self.emit(Opcode::Dup, None, 0);
        self.emit(unwrap, None, 0);
        let jump_fail = self.emit_jump(Opcode::JumpIfFalse, 0);
        self.emit(Opcode::Pop, None, 0);

        if matches!(inner, Pattern::Wildcard | Pattern::Identifier(_)) {
            self.emit(Opcode::Pop, None, 0);
            self.emit_push_constant(Value::Bool(true), 0)?;
            let jump_end = self.emit_jump(Opcode::Jump, 0);
            self.patch_jump(jump_fail)?;
            let jump_end2 = self.emit_jump(Opcode::Jump, 0);
            let end = self.current_chunk().instructions.len();
            self.patch_jump_at(jump_end, end)?;
            self.patch_jump_at(jump_end2, end)?;
            return Ok(());
        }

        self.compile_pattern_test(inner)?;
        let jump_nested_fail = self.emit_jump(Opcode::JumpIfFalse, 0);
        self.emit(Opcode::Pop, None, 0);
        self.emit(Opcode::Pop, None, 0);
        self.emit_push_constant(Value::Bool(true), 0)?;
        let jump_end = self.emit_jump(Opcode::Jump, 0);

        self.patch_jump(jump_nested_fail)?;
        self.emit(Opcode::Pop, None, 0);
        self.emit(Opcode::Pop, None, 0);
        self.emit_push_constant(Value::Bool(false), 0)?;
        let jump_end2 = self.emit_jump(Opcode::Jump, 0);

        self.patch_jump(jump_fail)?;
        // unwrap fail left [s, false]
        let jump_end3 = self.emit_jump(Opcode::Jump, 0);

        let end = self.current_chunk().instructions.len();
        self.patch_jump_at(jump_end, end)?;
        self.patch_jump_at(jump_end2, end)?;
        self.patch_jump_at(jump_end3, end)?;
        Ok(())
    }

    fn compile_pattern_bindings(
        &mut self,
        pattern: &Pattern,
        scrut_slot: u32,
    ) -> Result<usize, CompilerError> {
        self.emit(Opcode::LoadLocal, Some(scrut_slot), 0);
        self.compile_bindings_from_stack(pattern)
    }

    fn compile_bindings_from_stack(&mut self, pattern: &Pattern) -> Result<usize, CompilerError> {
        match pattern {
            Pattern::Identifier(name) => {
                self.define_variable(name)?;
                Ok(1)
            }
            Pattern::Wildcard | Pattern::Literal(_) | Pattern::Range(..) | Pattern::Or(_) => {
                self.emit(Opcode::Pop, None, 0);
                Ok(0)
            }
            Pattern::Guard(inner, _) => self.compile_bindings_from_stack(inner),
            Pattern::Array(elems) | Pattern::Tuple(elems) => {
                if self.current.scope_depth > 0 {
                    // Claim the sequence as a temporary local so element extracts
                    // land in the contiguous local region.
                    self.define_variable("$seq")?;
                    let seq_slot = self.current.locals.len().saturating_sub(1) as u32;
                    let mut total = 1usize; // includes the $seq temp
                    for (i, sub) in elems.iter().enumerate() {
                        if matches!(sub, Pattern::Wildcard | Pattern::Literal(_)) {
                            continue;
                        }
                        self.emit(Opcode::LoadLocal, Some(seq_slot), 0);
                        self.emit_push_constant(Value::Int(i as i64), 0)?;
                        self.emit(Opcode::IndexGet, None, 0);
                        total += self.compile_bindings_from_stack(sub)?;
                    }
                    Ok(total)
                } else {
                    // Global scope: park the sequence in a temp global so each
                    // element can be StoreGlobal'd via define_variable.
                    let tmp = self.bytecode.intern_global("$seq_tmp");
                    self.emit(Opcode::Dup, None, 0);
                    self.emit(Opcode::StoreGlobal, Some(tmp), 0);
                    let mut total = 0usize;
                    for (i, sub) in elems.iter().enumerate() {
                        if matches!(sub, Pattern::Wildcard | Pattern::Literal(_)) {
                            continue;
                        }
                        self.emit(Opcode::LoadGlobal, Some(tmp), 0);
                        self.emit_push_constant(Value::Int(i as i64), 0)?;
                        self.emit(Opcode::IndexGet, None, 0);
                        total += self.compile_bindings_from_stack(sub)?;
                    }
                    self.emit(Opcode::Pop, None, 0);
                    Ok(total)
                }
            }
            Pattern::Ok(inner) | Pattern::Err(inner) | Pattern::Some(inner) => {
                let unwrap = match pattern {
                    Pattern::Ok(_) => Opcode::UnwrapOk,
                    Pattern::Err(_) => Opcode::UnwrapErr,
                    _ => Opcode::UnwrapSome,
                };
                self.emit(unwrap, None, 0);
                self.emit(Opcode::Pop, None, 0);
                self.compile_bindings_from_stack(inner)
            }
        }
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

    /// Fill missing trailing parameters with their default expressions.
    /// `local_offset` is 1 for methods (implicit `this` is local 0).
    fn emit_default_prologue(&mut self, params: &[Param]) -> Result<(), CompilerError> {
        self.emit_default_prologue_offset(params, 0)
    }

    fn emit_default_prologue_offset(
        &mut self,
        params: &[Param],
        local_offset: u32,
    ) -> Result<(), CompilerError> {
        let mut seen_default = false;
        for (i, param) in params.iter().enumerate() {
            match &param.default {
                Some(default_expr) => {
                    seen_default = true;
                    let slot = local_offset + i as u32;
                    // Skip when the caller supplied this argument: argc > i
                    // (methods pass `this` + args, so compare against i + local_offset - 0:
                    //  argc includes `this`, so provided user args are argc - local_offset).
                    self.emit(Opcode::Argc, None, 0);
                    if local_offset > 0 {
                        self.emit_push_constant(Value::Int(local_offset as i64), 0)?;
                        self.emit(Opcode::Sub, None, 0);
                    }
                    self.emit_push_constant(Value::Int(i as i64), 0)?;
                    self.emit(Opcode::Gt, None, 0);
                    // If provided (argc > i), skip the default fill.
                    let jump_skip = self.emit_jump(Opcode::JumpIfTrue, 0);
                    self.emit(Opcode::Pop, None, 0);
                    self.compile_expression(default_expr)?;
                    self.emit(Opcode::StoreLocal, Some(slot), 0);
                    let jump_done = self.emit_jump(Opcode::Jump, 0);
                    self.patch_jump(jump_skip)?;
                    self.emit(Opcode::Pop, None, 0);
                    self.patch_jump(jump_done)?;
                }
                None => {
                    if seen_default {
                        return Err(CompilerError::Internal(
                            "required parameters cannot follow optional parameters".to_string(),
                        ));
                    }
                }
            }
        }
        Ok(())
    }

    /// Keep `result` on the stack while popping `n` locals introduced by the
    /// current scope (used by match arms that bind pattern names).
    fn end_scope_keeping_result(&mut self, n: usize) {
        if n == 0 {
            self.current.scope_depth = self.current.scope_depth.saturating_sub(1);
            return;
        }
        // Move result into the first binding slot, then drop the remaining bindings.
        let first = self.current.locals.len().saturating_sub(n);
        self.emit(Opcode::StoreLocal, Some(first as u32), 0);
        for _ in 1..n {
            self.emit(Opcode::Pop, None, 0);
        }
        for _ in 0..n {
            self.current.locals.pop();
        }
        self.current.scope_depth = self.current.scope_depth.saturating_sub(1);
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
        self.compile_function_inner(name, params, body, true)
    }

    fn compile_function_inner(
        &mut self,
        name: &str,
        params: &[Param],
        body: &[Stmt],
        bind_global: bool,
    ) -> Result<(), CompilerError> {
        let chunk_index = self.bytecode.chunks.len();
        self.bytecode.chunks.push(Chunk::new(name.to_string()));
        // Register before compiling the body so recursive calls can CallDirect.
        self.known_functions
            .insert(name.to_string(), (chunk_index, params.len()));
        self.specializations
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
        self.emit_default_prologue(params)?;
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
            required: params.iter().filter(|p| p.default.is_none()).count(),
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

        if bind_global {
            self.define_variable(name)?;
        }

        Ok(())
    }

    /// Mangle concrete type args into a specialization suffix (`int$str`).
    fn type_mangle(t: &TypeAnnotation) -> String {
        match t {
            TypeAnnotation::Int => "int".into(),
            TypeAnnotation::Float => "float".into(),
            TypeAnnotation::Bool => "bool".into(),
            TypeAnnotation::Str => "str".into(),
            TypeAnnotation::Char => "char".into(),
            TypeAnnotation::Array(_) => "array".into(),
            TypeAnnotation::Map(..) => "map".into(),
            TypeAnnotation::Tuple(_) => "tuple".into(),
            TypeAnnotation::Result(..) => "result".into(),
            TypeAnnotation::Option(_) => "option".into(),
            TypeAnnotation::Custom(n) => n.clone(),
            TypeAnnotation::Param(n) => n.clone(),
            TypeAnnotation::Apply(name, args) => {
                if args.is_empty() {
                    name.clone()
                } else {
                    format!(
                        "{}_{}",
                        name,
                        args.iter()
                            .map(Self::type_mangle)
                            .collect::<Vec<_>>()
                            .join("_")
                    )
                }
            }
            TypeAnnotation::Infer => "any".into(),
        }
    }

    fn is_concrete_type(t: &TypeAnnotation) -> bool {
        match t {
            TypeAnnotation::Param(_) | TypeAnnotation::Infer => false,
            TypeAnnotation::Array(i) => Self::is_concrete_type(i),
            TypeAnnotation::Map(k, v) => Self::is_concrete_type(k) && Self::is_concrete_type(v),
            TypeAnnotation::Tuple(ts) => ts.iter().all(Self::is_concrete_type),
            TypeAnnotation::Result(a, b) => Self::is_concrete_type(a) && Self::is_concrete_type(b),
            TypeAnnotation::Option(i) => Self::is_concrete_type(i),
            TypeAnnotation::Apply(_, args) => args.iter().all(Self::is_concrete_type),
            _ => true,
        }
    }

    fn substitute_type(
        t: &TypeAnnotation,
        map: &HashMap<String, TypeAnnotation>,
    ) -> TypeAnnotation {
        match t {
            TypeAnnotation::Param(name) => map.get(name).cloned().unwrap_or_else(|| t.clone()),
            TypeAnnotation::Array(i) => {
                TypeAnnotation::Array(Box::new(Self::substitute_type(i, map)))
            }
            TypeAnnotation::Map(k, v) => TypeAnnotation::Map(
                Box::new(Self::substitute_type(k, map)),
                Box::new(Self::substitute_type(v, map)),
            ),
            TypeAnnotation::Tuple(ts) => {
                TypeAnnotation::Tuple(ts.iter().map(|x| Self::substitute_type(x, map)).collect())
            }
            TypeAnnotation::Result(a, b) => TypeAnnotation::Result(
                Box::new(Self::substitute_type(a, map)),
                Box::new(Self::substitute_type(b, map)),
            ),
            TypeAnnotation::Option(i) => {
                TypeAnnotation::Option(Box::new(Self::substitute_type(i, map)))
            }
            TypeAnnotation::Apply(name, args) => TypeAnnotation::Apply(
                name.clone(),
                args.iter().map(|x| Self::substitute_type(x, map)).collect(),
            ),
            other => other.clone(),
        }
    }

    /// Rewrite type-carrying positions in statements (annotations + turbofish).
    fn substitute_stmts(stmts: &[Stmt], map: &HashMap<String, TypeAnnotation>) -> Vec<Stmt> {
        stmts
            .iter()
            .map(|s| Self::substitute_stmt(s, map))
            .collect()
    }

    fn substitute_stmt(stmt: &Stmt, map: &HashMap<String, TypeAnnotation>) -> Stmt {
        match stmt {
            Stmt::Let {
                name,
                mutable,
                type_annotation,
                value,
                pattern,
            } => Stmt::Let {
                name: name.clone(),
                mutable: *mutable,
                type_annotation: type_annotation
                    .as_ref()
                    .map(|t| Self::substitute_type(t, map)),
                value: value.as_ref().map(|e| Self::substitute_expr(e, map)),
                pattern: pattern.clone(),
            },
            Stmt::Const {
                name,
                type_annotation,
                value,
            } => Stmt::Const {
                name: name.clone(),
                type_annotation: type_annotation
                    .as_ref()
                    .map(|t| Self::substitute_type(t, map)),
                value: Self::substitute_expr(value, map),
            },
            Stmt::FunctionDef {
                name,
                type_params,
                params,
                return_type,
                body,
            } => {
                // Nested generic defs: drop the params being substituted.
                let inner: Vec<String> = type_params
                    .iter()
                    .filter(|p| !map.contains_key(*p))
                    .cloned()
                    .collect();
                Stmt::FunctionDef {
                    name: name.clone(),
                    type_params: inner,
                    params: params
                        .iter()
                        .map(|p| Param {
                            name: p.name.clone(),
                            type_annotation: p
                                .type_annotation
                                .as_ref()
                                .map(|t| Self::substitute_type(t, map)),
                            default: p.default.as_ref().map(|e| Self::substitute_expr(e, map)),
                        })
                        .collect(),
                    return_type: return_type.as_ref().map(|t| Self::substitute_type(t, map)),
                    body: Self::substitute_stmts(body, map),
                }
            }
            Stmt::Return(v) => Stmt::Return(v.as_ref().map(|e| Self::substitute_expr(e, map))),
            Stmt::Expression(e) => Stmt::Expression(Self::substitute_expr(e, map)),
            Stmt::If {
                condition,
                then_branch,
                elif_branches,
                else_branch,
            } => Stmt::If {
                condition: Self::substitute_expr(condition, map),
                then_branch: Self::substitute_stmts(then_branch, map),
                elif_branches: elif_branches
                    .iter()
                    .map(|(c, b)| {
                        (
                            Self::substitute_expr(c, map),
                            Self::substitute_stmts(b, map),
                        )
                    })
                    .collect(),
                else_branch: else_branch.as_ref().map(|b| Self::substitute_stmts(b, map)),
            },
            Stmt::For {
                variable,
                iterable,
                body,
            } => Stmt::For {
                variable: variable.clone(),
                iterable: Self::substitute_expr(iterable, map),
                body: Self::substitute_stmts(body, map),
            },
            Stmt::While { condition, body } => Stmt::While {
                condition: Self::substitute_expr(condition, map),
                body: Self::substitute_stmts(body, map),
            },
            Stmt::ClassDef {
                name,
                type_params,
                superclass,
                traits,
                members,
            } => Stmt::ClassDef {
                name: name.clone(),
                type_params: type_params.clone(),
                superclass: superclass.clone(),
                traits: traits.clone(),
                members: members
                    .iter()
                    .map(|m| Self::substitute_member(m, map))
                    .collect(),
            },
            other => other.clone(),
        }
    }

    fn substitute_member(m: &ClassMember, map: &HashMap<String, TypeAnnotation>) -> ClassMember {
        match m {
            ClassMember::Property {
                name,
                mutable,
                type_annotation,
                default,
            } => ClassMember::Property {
                name: name.clone(),
                mutable: *mutable,
                type_annotation: type_annotation
                    .as_ref()
                    .map(|t| Self::substitute_type(t, map)),
                default: default.as_ref().map(|e| Self::substitute_expr(e, map)),
            },
            ClassMember::Method {
                name,
                is_override,
                params,
                return_type,
                body,
            } => ClassMember::Method {
                name: name.clone(),
                is_override: *is_override,
                params: params
                    .iter()
                    .map(|p| Param {
                        name: p.name.clone(),
                        type_annotation: p
                            .type_annotation
                            .as_ref()
                            .map(|t| Self::substitute_type(t, map)),
                        default: p.default.as_ref().map(|e| Self::substitute_expr(e, map)),
                    })
                    .collect(),
                return_type: return_type.as_ref().map(|t| Self::substitute_type(t, map)),
                body: Self::substitute_stmts(body, map),
            },
            ClassMember::Constructor {
                params,
                super_args,
                body,
            } => ClassMember::Constructor {
                params: params
                    .iter()
                    .map(|p| Param {
                        name: p.name.clone(),
                        type_annotation: p
                            .type_annotation
                            .as_ref()
                            .map(|t| Self::substitute_type(t, map)),
                        default: p.default.as_ref().map(|e| Self::substitute_expr(e, map)),
                    })
                    .collect(),
                super_args: super_args
                    .as_ref()
                    .map(|a| a.iter().map(|e| Self::substitute_expr(e, map)).collect()),
                body: Self::substitute_stmts(body, map),
            },
        }
    }

    fn substitute_expr(expr: &Expr, map: &HashMap<String, TypeAnnotation>) -> Expr {
        match expr {
            Expr::Call {
                callee,
                args,
                type_args,
            } => Expr::Call {
                callee: Box::new(Self::substitute_expr(callee, map)),
                args: args.iter().map(|a| Self::substitute_expr(a, map)).collect(),
                type_args: type_args
                    .iter()
                    .map(|t| Self::substitute_type(t, map))
                    .collect(),
            },
            Expr::MethodCall {
                object,
                method,
                args,
            } => Expr::MethodCall {
                object: Box::new(Self::substitute_expr(object, map)),
                method: method.clone(),
                args: args.iter().map(|a| Self::substitute_expr(a, map)).collect(),
            },
            Expr::BinaryOp { op, left, right } => Expr::BinaryOp {
                op: op.clone(),
                left: Box::new(Self::substitute_expr(left, map)),
                right: Box::new(Self::substitute_expr(right, map)),
            },
            Expr::UnaryOp { op, operand } => Expr::UnaryOp {
                op: op.clone(),
                operand: Box::new(Self::substitute_expr(operand, map)),
            },
            Expr::PropertyAccess { object, property } => Expr::PropertyAccess {
                object: Box::new(Self::substitute_expr(object, map)),
                property: property.clone(),
            },
            Expr::Index { object, index } => Expr::Index {
                object: Box::new(Self::substitute_expr(object, map)),
                index: Box::new(Self::substitute_expr(index, map)),
            },
            Expr::Array(elems) => Expr::Array(
                elems
                    .iter()
                    .map(|e| Self::substitute_expr(e, map))
                    .collect(),
            ),
            Expr::Tuple(elems) => Expr::Tuple(
                elems
                    .iter()
                    .map(|e| Self::substitute_expr(e, map))
                    .collect(),
            ),
            Expr::Map(entries) => Expr::Map(
                entries
                    .iter()
                    .map(|(k, v)| (Self::substitute_expr(k, map), Self::substitute_expr(v, map)))
                    .collect(),
            ),
            Expr::Ok(e) => Expr::Ok(Box::new(Self::substitute_expr(e, map))),
            Expr::Err(e) => Expr::Err(Box::new(Self::substitute_expr(e, map))),
            Expr::Some(e) => Expr::Some(Box::new(Self::substitute_expr(e, map))),
            Expr::Try(e) => Expr::Try(Box::new(Self::substitute_expr(e, map))),
            Expr::Assign { target, value } => Expr::Assign {
                target: Box::new(Self::substitute_expr(target, map)),
                value: Box::new(Self::substitute_expr(value, map)),
            },
            Expr::CompoundAssign { op, target, value } => Expr::CompoundAssign {
                op: op.clone(),
                target: Box::new(Self::substitute_expr(target, map)),
                value: Box::new(Self::substitute_expr(value, map)),
            },
            Expr::If {
                condition,
                then_branch,
                else_branch,
            } => Expr::If {
                condition: Box::new(Self::substitute_expr(condition, map)),
                then_branch: Box::new(Self::substitute_expr(then_branch, map)),
                else_branch: else_branch
                    .as_ref()
                    .map(|e| Box::new(Self::substitute_expr(e, map))),
            },
            Expr::Lambda { params, body } => Expr::Lambda {
                params: params
                    .iter()
                    .map(|p| Param {
                        name: p.name.clone(),
                        type_annotation: p
                            .type_annotation
                            .as_ref()
                            .map(|t| Self::substitute_type(t, map)),
                        default: p.default.as_ref().map(|e| Self::substitute_expr(e, map)),
                    })
                    .collect(),
                body: Box::new(Self::substitute_expr(body, map)),
            },
            Expr::Block(stmts) => Expr::Block(Self::substitute_stmts(stmts, map)),
            other => other.clone(),
        }
    }

    /// Best-effort concrete type of a call argument for specialization inference.
    fn simple_type_of_expr(expr: &Expr) -> Option<TypeAnnotation> {
        match expr {
            Expr::Literal(Literal::Int(_)) => Some(TypeAnnotation::Int),
            Expr::Literal(Literal::Float(_)) => Some(TypeAnnotation::Float),
            Expr::Literal(Literal::Bool(_)) => Some(TypeAnnotation::Bool),
            Expr::Literal(Literal::Str(_)) => Some(TypeAnnotation::Str),
            Expr::Literal(Literal::Char(_)) => Some(TypeAnnotation::Char),
            Expr::StringInterpolation(_) => Some(TypeAnnotation::Str),
            Expr::Array(_) => Some(TypeAnnotation::Array(Box::new(TypeAnnotation::Infer))),
            Expr::Map(_) => Some(TypeAnnotation::Map(
                Box::new(TypeAnnotation::Infer),
                Box::new(TypeAnnotation::Infer),
            )),
            Expr::Tuple(_) => Some(TypeAnnotation::Tuple(Vec::new())),
            Expr::Ok(_) | Expr::Err(_) => Some(TypeAnnotation::Result(
                Box::new(TypeAnnotation::Infer),
                Box::new(TypeAnnotation::Infer),
            )),
            Expr::Some(_) | Expr::None => {
                Some(TypeAnnotation::Option(Box::new(TypeAnnotation::Infer)))
            }
            _ => None,
        }
    }

    /// Resolve a call to a monomorphized chunk when type args are concrete.
    /// Returns `(chunk_index, arity)` of the specialization.
    fn resolve_specialization(
        &mut self,
        name: &str,
        args: &[Expr],
        type_args: &[TypeAnnotation],
    ) -> Result<Option<(usize, usize)>, CompilerError> {
        if !self.generic_functions.contains_key(name) {
            return Ok(None);
        }
        let concrete: Option<Vec<TypeAnnotation>> = if !type_args.is_empty() {
            if type_args.iter().all(Self::is_concrete_type) {
                Some(type_args.to_vec())
            } else {
                None
            }
        } else {
            self.infer_type_args(name, args)
        };
        let Some(targs) = concrete else {
            return Ok(None);
        };
        let suffix = targs
            .iter()
            .map(Self::type_mangle)
            .collect::<Vec<_>>()
            .join("$");
        let spec_name = format!("{}${}", name, suffix);
        if let Some(entry) = self.specializations.get(&spec_name).copied() {
            return Ok(Some(entry));
        }
        if let Some(entry) = self.known_functions.get(&spec_name).copied() {
            self.specializations.insert(spec_name.clone(), entry);
            return Ok(Some(entry));
        }
        self.monomorphize(name, &targs, &spec_name)?;
        Ok(self.specializations.get(&spec_name).copied())
    }

    /// Infer type args from argument expression types (one param type per type param).
    fn infer_type_args(&self, name: &str, args: &[Expr]) -> Option<Vec<TypeAnnotation>> {
        let generic = self.generic_functions.get(name)?;
        let mut found: HashMap<String, TypeAnnotation> = HashMap::new();
        for (i, param) in generic.params.iter().enumerate() {
            let Some(TypeAnnotation::Param(tp)) = &param.type_annotation else {
                continue;
            };
            let Some(arg) = args.get(i) else {
                continue;
            };
            let Some(ty) = Self::simple_type_of_expr(arg) else {
                continue;
            };
            match found.get(tp) {
                Some(prev) => {
                    if prev != &ty {
                        return None;
                    }
                }
                None => {
                    found.insert(tp.clone(), ty);
                }
            }
        }
        if found.len() != generic.type_params.len() {
            return None;
        }
        Some(
            generic
                .type_params
                .iter()
                .map(|p| found.get(p).cloned().unwrap())
                .collect(),
        )
    }

    /// Compile a specialized chunk `name$int` (cached).
    fn monomorphize(
        &mut self,
        name: &str,
        type_args: &[TypeAnnotation],
        spec_name: &str,
    ) -> Result<(), CompilerError> {
        if self.specializations.contains_key(spec_name) {
            return Ok(());
        }
        let generic = self
            .generic_functions
            .get(name)
            .ok_or_else(|| CompilerError::UndefinedVariable(name.to_string()))?
            .clone();
        if generic.type_params.len() != type_args.len() {
            return Err(CompilerError::UndefinedVariable(spec_name.to_string()));
        }
        let map: HashMap<String, TypeAnnotation> = generic
            .type_params
            .iter()
            .cloned()
            .zip(type_args.iter().cloned())
            .collect();
        let params: Vec<Param> = generic
            .params
            .iter()
            .map(|p| Param {
                name: p.name.clone(),
                type_annotation: p
                    .type_annotation
                    .as_ref()
                    .map(|t| Self::substitute_type(t, &map)),
                default: p.default.clone(),
            })
            .collect();
        let body = Self::substitute_stmts(&generic.body, &map);
        self.compile_function_inner(spec_name, &params, &body, false)
    }

    fn compile_lambda(&mut self, params: &[Param], body: &Expr) -> Result<(), CompilerError> {
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
        self.emit_default_prologue(params)?;
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
            required: params.iter().filter(|p| p.default.is_none()).count(),
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
        let mut pending_prop_defaults: Vec<(String, Expr)> = Vec::new();

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
                    // Non-literal defaults are applied in the constructor prologue.
                    _ => Value::Null,
                };
                properties.push((prop_name.clone(), default_value));
                if let Some(d) = default {
                    if !matches!(d, Expr::Literal(_)) {
                        pending_prop_defaults.push((prop_name.clone(), d.clone()));
                    }
                }
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
                            type_args: Vec::new(),
                        }));
                    }
                    // Apply non-literal property defaults before user body.
                    for (pname, dexpr) in &pending_prop_defaults {
                        full_body.push(Stmt::Expression(Expr::Assign {
                            target: Box::new(Expr::PropertyAccess {
                                object: Box::new(Expr::Identifier("this".to_string())),
                                property: pname.clone(),
                            }),
                            value: Box::new(dexpr.clone()),
                        }));
                    }
                    full_body.extend(body.iter().cloned());
                    ("init".to_string(), params.clone(), full_body)
                }
                ClassMember::Property { .. } => continue,
            };

            let (chunk_index, arity) = self.compile_method(name, &method_name, &params, &body)?;
            methods.push((method_name.clone(), chunk_index));
            method_info.insert(method_name, (chunk_index, arity));
        }

        // Inject trait default methods not overridden by the class.
        for (method_name, params, body) in injected_defaults {
            let (chunk_index, arity) = self.compile_method(name, &method_name, &params, &body)?;
            methods.push((method_name.clone(), chunk_index));
            method_info.insert(method_name, (chunk_index, arity));
        }

        // Subclass without its own constructor inherits the parent's `init`
        // so `Child(args)` still initializes inherited fields. Flatten all
        // parent methods not overridden here so Invoke finds them.
        if let Some(parent) = superclass.as_ref() {
            if let Some(pinfo) = self.class_info.get(parent) {
                let parent_methods = pinfo.methods.clone();
                for (m_name, (chunk, arity)) in parent_methods {
                    if !method_info.contains_key(&m_name) {
                        methods.push((m_name.clone(), chunk));
                        method_info.insert(m_name, (chunk, arity));
                    }
                }
            }
        }

        // Class with non-literal property defaults but no constructor: synthesize init.
        if !declared_methods.contains_key("init") && !pending_prop_defaults.is_empty() {
            let mut full_body = Vec::new();
            if let Some(parent) = superclass.as_ref() {
                if self
                    .class_info
                    .get(parent)
                    .and_then(|p| p.methods.get("init"))
                    .is_some()
                {
                    full_body.push(Stmt::Expression(Expr::Call {
                        callee: Box::new(Expr::Identifier("super".to_string())),
                        args: vec![],
                        type_args: Vec::new(),
                    }));
                }
            }
            for (pname, dexpr) in &pending_prop_defaults {
                full_body.push(Stmt::Expression(Expr::Assign {
                    target: Box::new(Expr::PropertyAccess {
                        object: Box::new(Expr::Identifier("this".to_string())),
                        property: pname.clone(),
                    }),
                    value: Box::new(dexpr.clone()),
                }));
            }
            let (chunk_index, arity) = self.compile_method(name, "init", &[], &full_body)?;
            methods.push(("init".to_string(), chunk_index));
            method_info.insert("init".to_string(), (chunk_index, arity));
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
        // Implicit receiver is always local 0. A leading `self` parameter is the
        // same binding (alias), not an extra argument.
        self.define_variable("this")?;
        let mut eff_params: &[Param] = params;
        if params.first().map(|p| p.name == "self").unwrap_or(false) {
            eff_params = &params[1..];
        }
        for param in eff_params {
            self.define_variable(&param.name)?;
        }
        self.emit_default_prologue_offset(eff_params, 1)?;
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

        // Record arity so register lowering sizes expression temps above this+params.
        {
            let user_params = if params.first().map(|p| p.name == "self").unwrap_or(false) {
                params.len() - 1
            } else {
                params.len()
            };
            let arity = user_params + 1;
            let required = params.iter().filter(|p| p.default.is_none()).count() + 1;
            self.bytecode.chunks[chunk_index]
                .constants
                .push(Value::Function(Rc::new(FunctionObj {
                    name: format!("{}.{}", class_name, method_name).into(),
                    arity,
                    required,
                    chunk_index,
                })));
        }















        let extra = if params.first().map(|p| p.name == "self").unwrap_or(false) {
            0
        } else {
            0
        };
        // `this` is always local 0; a leading `self` param is an alias, not an arg.
        let user_params = if params.first().map(|p| p.name == "self").unwrap_or(false) {
            params.len() - 1
        } else {
            params.len()
        };
        let arity = user_params + 1 + extra; // + implicit `this`
        Ok((chunk_index, arity))
    }

    /// Compile `super(args...)`: call the superclass `init` with the current `this`,
    /// then store the returned (updated) instance back into local 0.
    fn compile_super_call(&mut self, args: &[Expr]) -> Result<(), CompilerError> {
        let current_name = self.current_class.clone().ok_or_else(|| {
            CompilerError::Internal("`super` used outside of a class".to_string())
        })?;
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
                required: init_arity,
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

    /// Compile `super.method(...)`: call the superclass method with the current `this`.
    fn compile_super_method_call(
        &mut self,
        method: &str,
        args: &[Expr],
    ) -> Result<(), CompilerError> {
        let current_name = self.current_class.clone().ok_or_else(|| {
            CompilerError::Internal("`super` used outside of a class".to_string())
        })?;
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
        let (chunk, arity) = self
            .class_info
            .get(&parent_name)
            .and_then(|p| p.methods.get(method).copied())
            .ok_or_else(|| {
                CompilerError::Internal(format!(
                    "superclass '{}' has no method '{}'",
                    parent_name, method
                ))
            })?;
        self.emit_push_constant(
            Value::Function(Rc::new(FunctionObj {
                name: format!("{}.{}", parent_name, method).into(),
                arity,
                required: arity,
                chunk_index: chunk,
            })),
            0,
        )?;
        self.emit(Opcode::LoadLocal, Some(0), 0);
        for arg in args {
            self.compile_expression(arg)?;
        }
        self.emit(Opcode::Call, Some((args.len() + 1) as u32), 0);
        Ok(())
    }

    fn compile_trait(
        &mut self,
        name: &str,
        supertraits: &[String],
        methods: &[TraitMethod],
    ) -> Result<(), CompilerError> {
        if self.trait_info.contains_key(name) {
            return Err(CompilerError::DuplicateTrait(name.to_string()));
        }
        let mut required = Vec::new();
        let mut defaults = HashMap::new();
        // Inherit required methods and defaults from supertraits first.
        for st in supertraits {
            let info = self
                .trait_info
                .get(st)
                .ok_or_else(|| CompilerError::Internal(format!("Unknown supertrait '{}'", st)))?;
            for req in &info.required {
                if !required.contains(req) {
                    required.push(req.clone());
                }
            }
            for (m_name, body) in &info.defaults {
                defaults
                    .entry(m_name.clone())
                    .or_insert_with(|| body.clone());
            }
        }
        for method in methods {
            match method {
                TraitMethod::Required { name: m_name, .. } => {
                    if !required.contains(m_name) {
                        required.push(m_name.clone());
                    }
                    // A required declaration overrides a supertrait default.
                    defaults.remove(m_name);
                }
                TraitMethod::Default {
                    name: m_name,
                    params,
                    body,
                    ..
                } => {
                    defaults.insert(m_name.clone(), (params.clone(), body.clone()));
                    required.retain(|r| r != m_name);
                }
            }
        }
        self.trait_info
            .insert(name.to_string(), CompiledTrait { required, defaults });
        Ok(())
    }
}

impl Default for Compiler {
    fn default() -> Self {
        Self::new()
    }
}
