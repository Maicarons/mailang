# MaìLang 下一步更新方案（深度研究报告）

> **项目链接**：[GitHub](https://github.com/Maicarons/mailang) · [v0.1.0](https://github.com/Maicarons/mailang/releases/tag/v0.1.0) · [Tags](https://github.com/Maicarons/mailang/tags)
>
> 研究日期：2026-06-17
> 研究方法：三路并行深度审计 — 代码结构扫描 + 运行时行为验证 + 竞品/趋势分析

---

## 一、核心发现：文档与现实的巨大差距

三份独立审计一致指出：**项目文档严重高估了实际完成度。**

### 1.1 真实状态矩阵

| 组件 | 文档声称 | 实际状态 | 严重度 |
|------|---------|---------|--------|
| Lexer | ✅ 完成 | ✅ 真实完整 | — |
| Parser | ✅ 完成 | ✅ 真实完整（Pratt 优先级正确） | — |
| AST | ✅ 完成 | ✅ 真实完整 + serde | — |
| Bytecode | ✅ 完成 | ✅ 真实完整（50+ 指令） | — |
| Core 管道 | ✅ 完成 | ✅ 可用（Parser→Compiler→VM） | — |
| CLI/REPL | ✅ 完成 | ✅ 可用 | — |
| **OOP** | ✅ 完成 | ❌ **Stack underflow** — 方法体未编译 | P0 |
| **Trait** | ✅ 完成 | ❌ **compile_trait 为空函数** | P0 |
| **闭包** | ✅ 完成 | ❌ **Upvalue 桩 — LoadUpvalue 永远 push Null** | P0 |
| **模式匹配** | ✅ 完成 | ❌ **MatchPattern 永远 push true** | P0 |
| **Analyzer** | ✅ 完成 | ❌ **738 行代码从未被调用** | P0 |
| **GC** | ✅ 完成 | ❌ **84 行空壳，VM 从未使用** | P0 |
| **测试** | ✅ | ❌ **0 个测试实际运行** — tests/ 在 workspace 根目录 | P0 |
| **FFI 12 语言** | ✅ | ⚠️ **仅 C 可运行**，其余为不可执行 demo | P1 |
| **no_std / 64KB** | ✅ | ❌ **无任何 `#![no_std]`**，feature flag 是摆设 | P1 |
| **Result/Option** | ✅ | ❌ 编译为 `[tag, value]` 数组，无真实语义 | P1 |
| **LSP** | ⚠️ 桩 | ❌ 仅 initialize/shutdown | P2 |

### 1.2 运行时验证结果（实测）

```
examples/hello.mai          → ✅ PASS
examples/fibonacci.mai      → ✅ PASS  
examples/lambda_demo.mai    → ✅ PASS（无捕获的 lambda）
examples/iot_sensor.mai     → ✅ PASS
examples/wasm_demo.mai      → ✅ PASS

examples/oop_demo.mai       → ❌ Error: Stack underflow
examples/trait_demo.mai     → ❌ Error: Stack underflow
examples/error_handling.mai → ❌ Parse error (Result<float, str> 无法解析)

"你好".len                  → ❌ 6 (字节数，应为 2)
0x10                        → ❌ 0 (不支持十六进制)
match wildcard _ => "..."   → ❌ 返回 null (永不匹配)
闭包捕获外部变量             → ❌ 返回 null
```

---

## 二、P0 问题详解

### 2.1 类系统完全不工作

**根因**（`compiler.rs:909-935`）：`compile_class` 创建了方法 chunk 索引，但**从不编译方法体**。

```rust
// 当前代码：只创建空 chunk
for member in members {
    if let ClassMember::Method { name, params, body, .. } = member {
        let chunk_index = self.bytecode.chunks.len();
        self.bytecode.chunks.push(Chunk::new(...));
        methods.push((name.clone(), chunk_index));
        // params 和 body 被完全忽略！
    }
}
```

**修复方案**：需要在独立的 FunctionCompiler 中编译每个方法体，然后在 VM 中实现 `CreateInstance`/`Invoke`/`this` 绑定。

### 2.2 闭包完全不工作

**根因**（`vm.rs:206-213`）：

```rust
Opcode::LoadUpvalue => {
    let _index = instruction.operand.unwrap_or(0) as usize;
    self.push(Value::Null)?;  // 永远 push Null！
}
Opcode::StoreUpvalue => {
    let _index = instruction.operand.unwrap_or(0) as usize;
    let _value = self.pop()?;  // 丢弃值！
}
```

### 2.3 模式匹配是假的

**根因**（`vm.rs` + `compiler.rs:778-780`）：

```rust
// VM 中
Opcode::MatchPattern => {
    self.push(Value::Bool(true))?;  // 永远 true！
}

// Compiler 中
_ => {
    self.emit_push_constant(Value::Bool(true), 0)?;  // 复杂模式直接 true
}
```

### 2.4 测试基础设施完全缺失

- `tests/basic.rs` 在 workspace 根目录，但 workspace 是 virtual（无 root package）
- 0 个 `#[test]` 函数存在于任何 crate 中
- CI 的 test job 每次都"通过"（0 tests = 0 failures）

---

## 三、性能问题

### 3.1 Value 深拷贝

**根因**：`Value` 是 owned enum，几乎每次访问都 `.clone()`：

```rust
// vm.rs — 每次属性访问都 clone
if name == &prop_name {
    self.push(v.clone())?;
}
```

**基准数据**（来自 benchmark/REPORT.md）：Fibonacci(30) 比 QuickJS **慢 8.2 倍**。

**修复方案**：堆类型使用 `Rc`/`Rc<RefCell>`：

```rust
enum Value {
    Null, Bool(bool), Int(i64), Float(f64), Char(char),
    Str(Rc<str>),
    Array(Rc<RefCell<Vec<Value>>>),
    Map(Rc<RefCell<HashMap<String, Value>>>),
    Instance(Rc<RefCell<InstanceObject>>),
    Closure(Rc<ClosureObject>),
}
```

### 3.2 全局变量查找

每次 `LoadGlobal` 都做 String clone + HashMap 查找。应改为编译时分配整数 ID。

---

## 四、更新路线图

### Phase A：诚实 + 修复核心（2-4 周）

**目标：让文档与现实一致，让核心语言特性真正工作**

```
任务                                    优先级  预估工时
──────────────────────────────────────────────────────
A1. 修复测试基础设施                      P0    2h
    - 将 tests/ 移到 crates/mailang-core/tests/
    - 添加 30+ 单元测试覆盖每个 crate
    - 添加集成测试覆盖所有示例

A2. 实现类系统完整编译                    P0    12h
    - compile_class 编译方法体
    - VM: CreateInstance 创建实例对象
    - VM: Invoke 调用方法
    - VM: this 绑定
    - super() 父类构造
    - 测试：oop_demo.mai 通过

A3. 实现闭包完整执行                      P0    8h
    - VM: LoadUpvalue/StoreUpvalue 真实实现
    - 闭包捕获机制（Lua 风格 open/closed upvalues）
    - 递归闭包支持
    - 测试：闭包捕获外部变量

A4. 实现模式匹配真实编译                  P0    8h
    - MatchPattern 实现字面量比较
    - Wildcard `_` 永远匹配
    - Or 模式：1 | 2 | 3
    - 守卫：x if x > 10
    - 范围：1..10
    - 测试：所有 match 示例

A5. 接入 Analyzer 到管道                  P0    4h
    - core.eval() 调用 Analyzer
    - --check / --strict 标志
    - 未定义变量、类型错误报告

A6. 修复已知 bug                         P0    4h
    - GetProperty Instance continue→break
    - Map IndexGet continue→break  
    - UTF-8 .len 使用 chars().count()
    - Elif jump 修补
    - 十六进制/八进制/二进制字面量
    - 块注释 `*/` 终止符

A7. 诚实审计文档                         P0    2h
    - README 添加成熟度矩阵（Working/Partial/Stub）
    - REPORT.md 修正为真实状态
    - 移除或标记未实现的特性
```

### Phase B：性能基础（2-3 周）

```
任务                                    优先级  预估工时
──────────────────────────────────────────────────────
B1. Value 改为 Rc 模型                    P0    8h
    - 堆类型使用 Rc/Rc<RefCell>
    - 减少 80%+ 的 clone
    - 重新基准测试

B2. 全局变量槽表                          P1    4h
    - 编译时分配整数 ID
    - VM 用 Vec<Value> 存储全局
    - 名称仅用于调试

B3. 尾调用优化                            P1    4h
    - 自递归函数 TCO
    - fib 风格工作负载受益

B4. 基准测试 CI 集成                      P1    2h
    - criterion 基准套件
    - 与 QuickJS/MicroPython 对比
    - 回归检测
```

### Phase C：IoT 可信度（3-4 周）

```
任务                                    优先级  预估工时
──────────────────────────────────────────────────────
C1. 真实 no_std                          P1    8h
    - mailang-bytecode: #![no_std]
    - mailang-vm: alloc feature
    - CI: thumbv7em / riscv32imc 构建验证

C2. 字节码文件格式                        P1    6h
    - Magic + 版本 + Chunks + 调试映射
    - CLI: mailang build / mailang run --bytecode
    - 端序/对齐规则

C3. 体积测量 CI                           P1    2h
    - 每次构建报告 thumbv7em/riscv32 大小
    - 与 MicroPython/JerryScript 对比

C4. 最小 HAL trait                        P2    4h
    - Gpio / Delay / Adc trait
    - 通过 FFI 注册宿主函数
    - 模拟设备 demo
```

### Phase D：FFI / 嵌入（2-3 周）

```
任务                                    优先级  预估工时
──────────────────────────────────────────────────────
D1. 扩展 C API                           P1    6h
    - mailang_get_global_int / set_global_str
    - mailang_register_host_fn（IoT 关键）
    - 结构化错误（code + message + line + col）
    - catch_unwind 在每个 extern "C" 边界

D2. FFI 全语言验证                        P1    8h
    - C: MSVC + MinGW + Clang
    - Python: ctypes + PyO3（官方包）
    - Node.js: N-API
    - Go: cgo（修复签名）
    - 其余：文档而非"12 FFI"

D3. WASM 验证                             P1    4h
    - cargo build --target wasm32
    - 浏览器 demo
    - 体积测量
```

### Phase E：工具链（持续）

```
任务                                    优先级  预估工时
──────────────────────────────────────────────────────
E1. LSP 核心功能                          P2    12h
    - 诊断（来自 Analyzer）
    - 补全（函数、变量、类型）
    - 跳转定义
    - 悬停提示

E2. 格式化器 mailang fmt                   P2    6h
    - Go/Zig 经验：强制风格减少争议

E3. 模块系统 v2                            P2    8h
    - 独立 chunk 编译
    - 导出表
    - 不再 AST 注入
```

---

## 五、关键技术决策

| 决策 | 选择 | 理由 |
|------|------|------|
| 执行模型 | **栈式 VM + slot locals**（修正文档） | 当前实现如此；真寄存器 VM 是 Phase F |
| Value 表示 | **Rc/Rc\<RefCell\>** | 立即消除 clone 开销；IoT 可后续换 NaN-boxing |
| GC 策略 | **RC → RC+循环检测 → 标记清除** | 分阶段，从最简单的开始 |
| 类型系统 | **渐进式**（TypeScript 风格） | 动态默认，可选标注；IoT 脚本友好 |
| 错误处理 | **Result + ? 操作符** | Rust 风格已被验证；try/catch 是语法糖 |
| 模块系统 | **文件即模块 + 显式导出** | 简单、可预测、无隐式行为 |
| FFI 基础 | **C ABI + cbindgen + UniFFI** | 通用性最强；UniFFI 自动生成多语言 |
| 测试策略 | **先修测试基础设施，再写功能** | 没有测试的功能等于没写 |

---

## 六、风险评估

| 风险 | 概率 | 影响 | 缓解 |
|------|------|------|------|
| 类系统实现复杂度超预期 | 高 | 高 | 先最小可用版本，再迭代 |
| Value→Rc 迁移影响面大 | 高 | 中 | 分类型逐步迁移，每步测试 |
| no_std 迁移工作量大 | 中 | 中 | 先 bytecod+vm，再其他 |
| FFI 跨语言兼容性 | 高 | 中 | 每种语言独立 CI 测试 |
| 文档诚实化引发"倒退"观感 | 中 | 低 | 明确标注"开发中"而非删除 |

---

## 七、成功指标

### Phase A 完成标准
- [ ] `cargo test` 实际运行 30+ 测试，全部通过
- [ ] 所有 examples/*.mai 运行成功
- [ ] 类、闭包、模式匹配在 VM 中完整工作
- [ ] Analyzer 接入管道，提供类型检查
- [ ] 无 P0 bug

### Phase B 完成标准
- [ ] Fibonacci(30) 性能提升 3x+（vs 当前 8.2x 慢于 QuickJS）
- [ ] 全局查找无 String clone
- [ ] CI 基准测试回归检测

### Phase C 完成标准
- [ ] thumbv7em / riscv32 编译成功
- [ ] 字节码可序列化/反序列化
- [ ] 实测二进制大小报告

### Phase D 完成标准
- [ ] C API 支持宿主函数注册
- [ ] 4+ 语言绑定可运行
- [ ] WASM 浏览器 demo

---

## 八、90 天路线图

```
Week 1-2:   测试基础设施 + 类系统 + 闭包（Phase A 核心）
Week 3-4:   模式匹配 + Analyzer 接入 + Bug 修复（Phase A 收尾）
Week 5-6:   Value→Rc + 全局槽表 + 基准测试（Phase B）
Week 7-8:   no_std + 字节码格式 + 体积测量（Phase C）
Week 9-10:  C API 扩展 + FFI 验证 + WASM（Phase D）
Week 11-12: LSP 核心 + 格式化器 + 文档修正（Phase E）
Week 13:    发布 v0.2.0
```

---

## 九、从竞品学到的关键教训

| 来源 | 教训 |
|------|------|
| **Lua** | 小而完整 > 大而残缺。先完成闭包和 C API，再加语法。 |
| **Rhai** | 宿主函数注册比 FFI 更实用。脚本沙箱是 IoT 刚需。 |
| **Rune** | 测试驱动的语言开发：每个特性都有测试。 |
| **Zig** | IoT 语言必须显式暴露分配器和大小预算。C ABI 优先。 |
| **Go** | 更少特性但全部完成。强制格式化。 |
| **Rust** | 工具链即语言。没有 LSP + formatter + 测试 = 不是产品。 |
| **V（反面）** | 不要让文档声称未实现的功能。 |

---

*本报告基于三路并行深度审计：代码结构扫描、运行时行为验证、竞品/趋势分析。所有发现均经实际运行验证。*
