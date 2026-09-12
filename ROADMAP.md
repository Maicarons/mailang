# MaìLang 下一步更新方案（v0.2.0）

> **项目链接**：[GitHub](https://github.com/Maicarons/mailang) · [v0.1.0](https://github.com/Maicarons/mailang/releases/tag/v0.1.0)
>
> 更新日期：2026-09-12（Phase B 收尾完成后）
> 研究方法：运行时行为验证 + 代码审计 + 竞品对比

---

## 一、当前状态总结（Phase B 完成后）

### 1.1 已验证可用的功能

| 功能 | 状态 | 验证方式 |
|------|------|---------|
| 算术/字符串/布尔/null | ✅ | 59 个集成测试 |
| 变量 let/var/const | ✅ | 测试通过 |
| 函数定义与递归 | ✅ | fibonacci.mai |
| Lambda 表达式 | ✅ | `fn(x) { return x * 2 }` |
| 块体 Lambda | ✅ | `fn(x) { ... }` |
| 闭包捕获 | ✅ | counter 返回 1, 2 |
| if/elif/else | ✅ | 测试通过 |
| while/for 循环 | ✅ | 测试通过 |
| 字符串插值 | ✅ | `"{name}"` |
| 数组/字典 | ✅ | 索引读写（Rc\<RefCell\> 原地写） |
| OOP 类/继承/方法 | ✅ | oop_demo.mai |
| super() 调用 | ✅ | Dog extends Animal |
| Trait / implements | ✅ | trait_demo.mai（含默认方法注入） |
| match 字面量/通配符/范围/Ok|Err|Some | ✅ | 测试通过 |
| 泛型类型标注 Result/Option | ✅ | error_handling.mai |
| 十六进制/八进制/二进制 | ✅ | 0x10 == 16 |
| 块注释 /* */ | ✅ | 测试通过 |
| 尾调用优化 | ✅ | count(50000) 不栈溢出 |
| 全局槽表 | ✅ | LoadGlobal 无 String clone |
| UTF-8 标识符 | ✅ | `let 中文 = 42` |
| CLI/REPL | ✅ | run/eval |
| C FFI | ✅ | eval/eval_file |
| WASM + Playground | ✅ | VitePress 部署 |

### 1.2 仍需处理

| 问题 | 严重度 | 说明 |
|------|--------|------|
| Fibonacci(30) 未达 3x | P1 | 整数递归瓶颈不在 Value clone；需调用帧/指令分派优化 |
| Analyzer 未接入管道 | P1 | 代码存在但从未被调用 |
| GC 是空壳 | P2 | 84 行，VM 从未使用 |
| LSP 是桩 | P2 | 仅 initialize/shutdown |

---

## 二、Phase B 详细计划（2-3 周）

### 目标：修复剩余 P0/P1 问题，提升性能

### B1. 泛型类型标注（P0）

**问题**：`Result<float, str>` 解析失败，因为 `<` 被当作比较运算符。

**方案**：
- 在 `parse_type_annotation` 中识别 `Type<Args>` 模式
- 支持 `Result<T, E>`、`Option<T>`、`[T]` 等泛型
- 类型标注中的 `<` 不应与比较运算符冲突

**验收**：`error_handling.mai` 运行成功

### B2. Trait 系统实现（P0）

**问题**：`compile_trait` 是空函数，trait 定义被忽略。

**方案**：
- 编译 trait 方法签名到元数据
- 类实现 trait 时检查方法存在性
- 支持 trait 默认实现
- 支持 `implements` 关键字

**验收**：`trait_demo.mai` 运行成功

### B3. 范围匹配（P1）

**问题**：`1..10 => "small"` 在 match 中解析失败。

**方案**：
- Parser 的 `parse_pattern` 支持 `Pattern::Range`
- Compiler 编译范围比较（>= 且 < / <=）
- 支持 `1..=10`（包含端点）

**验收**：范围匹配测试通过 — **已完成（含 `..=`）**

### B4. Lexer 增强（P1）

**问题**：
- `0x10` 解析为 `0` 而非 `16`
- `/* */` 块注释不工作

**方案**：
- `read_number` 识别 `0x`/`0o`/`0b` 前缀
- 块注释终止符改为 `*/`

**验收**：十六进制/八进制/二进制字面量和块注释测试通过

### B5. println 内置函数注册（P1）

**问题**：`println` 在某些上下文中未注册。

**方案**：
- 确保 VM 初始化时注册所有标准库函数
- 添加缺失的内置函数

**验收**：所有示例中的 `println` 正常工作

### B6. Value → Rc 迁移（P1，性能）

**问题**：Value 深拷贝导致 Fibonacci(30) 比 QuickJS 慢 8.2 倍。

**方案**：
```rust
enum Value {
    Null, Bool(bool), Int(i64), Float(f64), Char(char),
    Str(Rc<str>),
    Array(Rc<RefCell<Vec<Value>>>),
    Map(Rc<RefCell<Vec<(Value, Value)>>>),
    Instance(Rc<RefCell<InstanceObject>>),
    Closure(Rc<ClosureObject>),
}
```

**验收**：Fibonacci(30) 性能提升 3x+

### B7. 全局变量槽表（P1，性能）

**问题**：每次 `LoadGlobal` 做 String clone + HashMap 查找。

**方案**：
- 编译时分配整数 ID
- VM 用 `Vec<Value>` 存储全局
- 名称仅用于调试

**验收**：全局查找无 String clone

### B8. 尾调用优化（P2）

**方案**：
- 自递归函数检测
- 尾调用复用当前栈帧

**验收**：深度递归不栈溢出

---

## 三、Phase C 详细计划（3-4 周）

### 目标：IoT 可信度 — **已完成**

### C1. 真实 no_std（P1） — 完成

- `mailang-bytecode`: `#![no_std]` + `alloc`；serde 仅在 `std` 下启用
- CI: thumbv7em / riscv32imc `cargo check --no-default-features`
- 说明：完整 VM 仍依赖 std（HashMap builtins / ThreadLocal HAL）

### C2. 字节码文件格式（P1） — 完成

- Magic `MAILBC01` + 版本 + 小端编码；见 `mailang-bytecode/src/format.rs`
- CLI: `mailang build file.mai [-o out.mailangbc]` / `mailang run file.mailangbc`
- 端序：全部小端；对齐：按字节流顺序写，无 padding

### C3. 体积测量 CI（P1） — 完成

- `benchmark/measure_size.py`：host CLI + 嵌入式 rlib
- CI `no_std` job 报告 rlib 大小

### C4. 最小 HAL trait（P2） — 完成

- `Gpio` / `Delay` / `Adc` + `SimulatedHal`（32 pin / 8 ADC）
- 内置：`gpio_write` / `gpio_read` / `delay_ms` / `adc_read`
- Demo: `examples/hal_sim.mai`

---

## 四、Phase D 详细计划（2-3 周）

### 目标：FFI / 嵌入 — **已完成**

### D1. 扩展 C API（P1） — 完成

- `mailang_get_global_int` / `set_global_int` / `get_global_str` / `set_global_str`
- `mailang_register_host_fn`（IoT 关键）
- 错误码：`MAILANG_OK/ERR_NULL/ERR_EVAL/ERR_UTF8/ERR_HOST/ERR_PANIC`
- `catch_unwind` 在每个 `extern "C"` 边界
- 头文件：`crates/mailang-ffi/include/mailang.h`

### D2. FFI 全语言验证（P1） — 完成

- C: MinGW gcc（`ffi/tests/test_c.c`）
- Python: ctypes（`ffi/tests/test_python.py`）
- Go: cgo（`ffi/tests/test_go.go`）
- Node.js: WASM + CLI 冒烟（`ffi/tests/test_node.js`）
- 宿主全局/回调由 core 缓存，跨 `eval` 重建 VM 后重放

### D3. WASM 验证（P1） — 完成

- `cargo build -p mailang-wasm --target wasm32-unknown-unknown --release`
- `wasm-pack build --target nodejs` → Node 可 `require`
- 体积：`mailang_wasm.wasm` ≈ 566 KiB（release）

---

## 五、Phase E 详细计划（持续）

### E1. LSP 核心功能（P2） — **基本完成**

- 诊断（来自 Analyzer）：`didOpen`/`didChange` 发布
- 补全：关键字 + 内置 + 文档内标识符
- 跳转定义：`fn`/`class`/`let`/`var`/`const`/`trait`
- 悬停：当前行预览
- 入口：`mailang lsp`

### E2. 格式化器 `mailang fmt`（P2） — **基本完成**

- AST 重印：4 空格缩进、规范化空格
- CLI：`mailang fmt file.mai` / `mailang fmt --check file.mai`

### E3. 模块系统 v2（P2） — **基本完成**

- 独立 parse + analyze 每个模块
- 导出表：顶层 `fn`/`let`/`const`（`_` 前缀不导出）
- 链接：`Compiler::compile_linked` 把模块体 + 导出命名空间表 + 主程序编进同一 Bytecode
- **不再 AST 注入**：主程序 AST 不改写
- CLI 验证：`examples/test_module_v2.mai`、`libs/time/example/basic.mai`

---

## 六、关键技术决策

| 决策 | 选择 | 理由 |
|------|------|------|
| 执行模型 | 栈式 VM + slot locals | 当前实现如此；真寄存器 VM 是 Phase F |
| Value 表示 | Rc/Rc\<RefCell\> | 立即消除 clone 开销 |
| GC 策略 | RC → RC+循环检测 → 标记清除 | 分阶段，从最简单的开始 |
| 类型系统 | 渐进式（TypeScript 风格） | 动态默认，可选标注 |
| 错误处理 | Result + ? 操作符 | Rust 风格已被验证 |
| 模块系统 | 文件即模块 + 显式导出 | 简单、可预测 |
| FFI 基础 | C ABI + cbindgen + UniFFI | 通用性最强 |
| 测试策略 | 先修测试基础设施，再写功能 | 没有测试的功能等于没写 |

---

## 七、90 天路线图（更新版）

```
Week 1-2:   泛型类型标注 + Trait 系统（Phase B 核心）
Week 3-4:   范围匹配 + Lexer 增强 + println 注册
Week 5-6:   Value→Rc + 全局槽表 + 尾调用优化
Week 7-8:   no_std + 字节码格式 + 体积测量（Phase C）
Week 9-10:  C API 扩展 + FFI 验证（Phase D）
Week 11-12: LSP 核心 + 格式化器 + 模块系统 v2（Phase E）
Week 13:    发布 v0.2.0
```

---

## 八、成功指标

### Phase B 完成标准
- [x] `error_handling.mai` 运行成功
- [x] `trait_demo.mai` 运行成功
- [x] 范围匹配 `1..10` 工作
- [x] 十六进制 `0x10` = 16
- [x] 块注释 `/* */` 工作
- [ ] Fibonacci(30) 性能提升 3x+（未达标：当前约与基线持平，整数递归瓶颈不在 clone）
- [x] 全局查找无 String clone（`Vec` 槽表直读）

### Phase C 完成标准
- [x] thumbv7em / riscv32 编译成功（`mailang-bytecode --no-default-features`）
- [x] 字节码可序列化/反序列化（`.mailangbc`）
- [x] 实测二进制大小报告（`benchmark/measure_size.py` + CI no_std job）

### Phase D 完成标准
- [x] C API 支持宿主函数注册（`mailang_register_host_fn` + 全局读写 + catch_unwind）
- [x] 4+ 语言绑定可运行：C (MinGW) / Python ctypes / Go cgo / Node.js (WASM)
- [x] WASM Node/浏览器路径：`wasm-pack` 产物约 566 KiB；`eval` 返回表达式结果

---

*本报告基于 Phase A 完成后的运行时验证，所有发现均经实际运行确认。*
