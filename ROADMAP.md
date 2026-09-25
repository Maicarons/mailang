# MaìLang 下一步更新方案（v0.2.6+ / 规划 Phase L → v0.3.0）

> **项目链接**：[GitHub](https://github.com/Maicarons/mailang) · [v0.2.6](https://github.com/Maicarons/mailang/releases/tag/v0.2.6)
>
> 更新日期：Phase B–K 已落地；本文末尾 **Phase L** 为下一步发展方案
> 研究方法：运行时行为验证 + 代码审计 + 测试实测 + 文档交叉核对

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

### 1.2 下一阶段入口（Phase G）

| 问题 | 严重度 | 说明 |
|------|--------|------|
| `?` 操作符未实现 | P0 | Lexer 有 `Token::Question`，解析/编译未接 |
| Array/Map 方法缺失 | P0 | `a.push(3)` 无法调用 |
| 类型诊断偏弱 | P1 | 标注类型未参与调用检查 |
| Playground wasm 未含最新 OOP 修复 | P1 | 需 `build:wasm` 刷新 |
| crates.io 未发布 | P1 | 缺 `CARGO_REGISTRY_TOKEN` |

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
Week 13:    发布 v0.2.2
```

---

## 八、成功指标

### Phase B 完成标准
- [x] `error_handling.mai` 运行成功
- [x] `trait_demo.mai` 运行成功
- [x] 范围匹配 `1..10` 工作
- [x] 十六进制 `0x10` = 16
- [x] 块注释 `/* */` 工作
- [x] Fibonacci(30) 性能提升 3x+（F1–F5 后约 **4.25x**，~137 ms）
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

## 九、Phase F 升级计划（性能 / v0.3）

### 目标：Fib(30) 达到基线 3x+（≤ ~195 ms）

| 项 | 内容 | 预期收益 |
|----|------|----------|
| F1 | 胖 Value 改 `Rc` 包装 + 热路径 Int/Call | **完成** |
| F2 | 立即数融合：`Add/Sub/Mul/Eq/Ne/Lt/Le/Gt/Ge` + Imm | **完成** |
| F3 | `Instruction` 改为 `Copy` + `line: u32` | **完成** |
| F4 | `CallDirect`（含无占位槽） | **完成** |
| F5 | release `panic = "abort"` | **完成** |
| F6 | GC：Rc 图循环检测与断环 | **完成** |
| F7 | Analyzer span + LSP 精确诊断 | **完成** |

**验收**：`bench_fib30.mai` release 平均 ≤ 195 ms。**当前 ~137 ms（~4.25x 基线）— 已达标。**
**Phase F 全部关闭。**

---

## 十、Phase G 升级计划（语言完整度 / DX / 生态 → v0.3）

> 调研日期：2026-09-12。Phase B–F 已闭环；下列项来自运行时探测与代码审计，按投入产出排序。

### G1. 错误传播 `?` 操作符（P0） — **完成**

- **验收**：`Ok`/`Some` 解包；`Err`/`None` 提前 return（`test_try_operator_*`）

### G2. 集合方法 API（P0） — **完成**

- Array：`push` `pop` `insert` `contains` `join` `reverse` `clear`
- Map：`keys` `values` `has` `remove` `clear`
- String：`trim` `split` `replace` `starts_with` `ends_with` `contains` `to_upper` `to_lower` `repeat`

### G3. Analyzer 类型检查强化（P1） — **基本完成**

- 已声明函数：arity + 参数类型兼容（Int⊂Float、Any/Unknown）
- 内置/宿主函数不做 arity 强制（动态）

### G4. 字符串与 IO 标准库补全（P1） — **完成**

- `read_file` / `write_file`

### G5. Playground / WASM 产品化（P1） — 部分

- CLI 行为已修；playground 需本地 `npm run build:wasm` 刷新产物

### G6. 工程化收尾（P1） — 部分

- crates.io：见下方「发布到 crates.io」
- 版本：工作区跟随最新 tag

### G7. 解释器深度（P2，可选） — 未做

- 寄存器 VM / 专用 Int 栈；数组/元组解构

### G5. Playground / WASM 产品化（P1）

- 重建 wasm-pack 产物（含继承构造修复）
- 示例一键切换（OOP / match / traits / IoT HAL）
- 错误信息展示行列

### G6. 工程化收尾（P1）

| 项 | 说明 |
|----|------|
| crates.io | 配置 `CARGO_REGISTRY_TOKEN` 后重跑 publish |
| 版本一致 | README 文档版 vs 包版本统一策略（文档跟最新 tag） |
| Dependabot | 已清零；保持 weekly 分组 |
| 头文件 | CI 校验 `mailang.h` 与 `mailang-ffi` 同步（cbindgen） |

### G7. 解释器深度（P2，可选）

- 寄存器 VM / 专用 Int 栈（继续压 Fib）
- 模式匹配：数组/元组解构
- 异步 / 协程（仅当有明确 IoT 场景）

### 建议节奏（v0.3）

```
Week 1:   G1 `?` + G2 集合方法（语言可用性质变）
Week 2:   G3 类型诊断 + G4 字符串/文件 API
Week 3:   G5 playground + G6 工程化
Week 4:   回归、文档、发布 v0.3.0
```

**v0.3 验收（建议）**
- [x] `Ok/Err` 可用 `?` 传播（G1）
- [x] Array/Map 方法齐全且有测试（G2）
- [x] 标注类型不匹配在 eval/LSP 可见（G3）
- [ ] Playground wasm 与 CLI 行为一致（G5 部分）
- [ ] CI 全绿 + Release 带二进制 +（可选）crates.io（G6 部分）

---

## 十一、Phase L 下一步发展方案（调研 → v0.3.0）

> 调研日期：Phase H–K 代码审计 + `cargo test` / `MAILANG_VM=register` 实测。
> 核心结论：**语言能力已超前于文档与发布；最大技术债是双后端不一致；最大产品缺口是 IoT 仍停留在模拟 HAL。**

### 11.1 实测基线（研究结论）

| 维度 | 实测结果 | 说明 |
|------|----------|------|
| 栈式 VM 集成测试 | **102/102 通过** | 含解构 / 默认参数 / postfix match / trait extends / `?` / GC |
| 全工作区测试 | **~218 全绿** | analyzer 19 · parser 13 · module 25 · stdlib 15 · compiler generics 8 · core 15+102 等 |
| 寄存器 VM 同套件 | **73/102（29 失败）** | `MatchPattern→Nop`、类/`this`/`super`/trait、默认参数、GC、部分 `?`/TCO |
| 包注册表 | **已实现** | `publish/install/search/yank/registry` + FS/HTTP 源 + `.mpkg` |
| 泛型单态化 | **已实现** | turbofish `id::<int>` → chunk `id$int`，有缓存；类字段仍擦除 |
| GC | **Rc 断环 + MarkSweepHeap** | 已接入栈 VM；`gc_stats` / `collect_cycles` |
| 性能 | Fib(30) ~137 ms（~4.25×） | Phase F 达标后未再压测 |
| 无 `todo!`/`unimplemented!` | **通过** | crates 内无未接桩宏 |

### 11.2 文档滞后（必须先还的债）

下列声明与代码不符，会直接伤害可信度：

| 位置 | 过时声明 | 实际 |
|------|----------|------|
| `README.md` / `README_zh.md` | No package registry | 注册表已实现 |
| `README.md` | Generics are annotation-only | 已有 monomorphization |
| `AGENTS.md` | No package registry；GC 仅 Rc+手动断环 | 两者均已落地 |
| `ROADMAP.md` 正文 | 停在 Phase G 规划 | CHANGELOG 已有 H–K |
| `REPORT.md` | Phase A 时代 | 严重过期 |
| `benchmark/REPORT.md` | Phase B 数据 | 需按 release 重跑 |

### 11.3 真实缺口（按投入产出排序）

| ID | 项 | 优先级 | 工作量 | 理由 |
|----|----|--------|--------|------|
| **L0** | 文档真相对齐（README / AGENTS / REPORT / bench） | **P0** | 0.5–1 天 | 对外门面；不改代码也能立刻提升可信度 |
| **L1** | 发布 Unreleased H–K → **v0.3.0**（CI 绿 + tag + 二进制） | **P0** | 1–2 天 | 功能已堆在 Unreleased，不发等于没有 |
| **L2** | **Register VM 对齐栈 VM**（目标 102/102） | **P0** | 1–2 周 | 最大技术债；双后端语义分叉会越拖越贵 |
| **L3** | IoT 真实路径：ESP32 端到端 + 驱动包从 stub 到契约可用 | **P1** | 1–2 周 | 项目定位护城河；目前只有文档和 SimulatedHal |
| **L4** | 足迹真实数据：`measure_size` + vs MicroPython 诚实表 | **P1** | 2–3 天 | IoT 叙事需要数字，不能继续 placeholder |
| **L5** | Parser 错误恢复（token-sync `recover_from_error`） | **P1** | 3–5 天 | REPL / LSP / fmt 体验质变 |
| **L6** | Playground WASM 刷新 + 示例一键切换 + 错误行列 | **P2** | 2–3 天 | G5 收尾 |
| **L7** | crates.io 发布（`CARGO_REGISTRY_TOKEN` + multi-crate） | **P2** | 1 天 | 工程收尾，依赖 token |
| **L8** | 泛型类单态化 **或** 文档明确「字段擦除」 | **P2** | 视选择 | 当前类泛型是 erased layout |
| **L9** | `async` / 协程 | **P3** | 大 | 无明确 IoT 场景前不启动（与既往决策一致） |
| **L10** | 寄存器 VM 性能对照（Fib/循环 vs 栈 VM） | **P3** | 2 天 | L2 完成后再比，否则无意义 |
| **—** | `mailang-macros` 仍为 passthrough | **P3** | 视需求 | 无调用方；勿为「完整」而造宏 |

### 11.4 战略取舍（三选一）

| 方案 | 路径 | 优点 | 风险 |
|------|------|------|------|
| **A. 先还债（推荐）** | L0 → L1 → L2 → L3 | 可信度与一致性优先；发布立刻可感知 | IoT 差异化晚 2–3 周 |
| B. 先 IoT 差异化 | L3 → L4 → L1 | 定位故事最快成形 | 寄存器 VM 债继续膨胀，后端分叉 |
| C. 先性能/语言深度 | L10 / L8 / L9 | 技术指标好看 | 偏离 IoT 定位；async 无场景支撑 |

**建议采用 A**：一致性是可信度基础，IoT 是定位护城河。L2 可与 L3 后半段并行（驱动契约不依赖寄存器 VM）。

### 11.5 Phase L 详细计划

#### L0. 文档真相对齐（0.5–1 天）

- README / README_zh：删「无注册表 / 泛型仅标注」；改为「FS 注册表 + turbofish 单态化（类字段擦除）」
- AGENTS.md Current Status：补注册表、mark-sweep、泛型单态化；剩余 gap 改为 L 表
- REPORT.md：升到 Phase K 诚实状态表（或链接 CHANGELOG + 本文）
- `benchmark/REPORT.md`：按当前 release 重跑 Fib/循环/调用/插值，标注 `--vm=stack|register`

**验收**：对外文档不再声称「没有已实现的能力」。

#### L1. 发布 v0.3.0（1–2 天）

- CHANGELOG：将 Unreleased H–K 合并进 `[0.3.0]`
- 工作区版本对齐 tag；`scripts/release.sh` 重跑
- CI 全绿 + Release 附件含 `mailang` 二进制
- （可选，同卡 L7）配置 token 后 `cargo publish`

**验收**：GitHub Release v0.3.0 可下载；文档版本号一致。

#### L2. Register VM 对齐（1–2 周，P0 核心）

按失败簇拆解（当前 29 fail）：

1. **Match lowering**：`MatchPattern` 不得再 `→ Nop`；补字面量/范围/Ok|Err|Some/或/守卫/数组与元组解构
2. **OOP 分发**：类创建、`this`/`super`、方法继承、`super.method`、trait 默认方法注入
3. **默认参数 prologue**：与栈 VM 同语义（提供/缺省两条路径栈平衡）
4. **`?` 与 TCO**：`TryQ` 返回路径、尾调用参数基址/表达式栈重叠
5. **GC 路径**：寄存器帧作为根参与 mark-sweep / `collect_cycles`

**验收**：`MAILANG_VM=register cargo test -p mailang-core --test integration` = **102/102**。
在此之前 **保持默认 `--vm=stack`**，register 标注 experimental。

#### L3. IoT 真实路径（1–2 周）

- ESP32：按 `docs/guide/esp32-blink.md` 打通一条 **可复现** 真机或 QEMU/模拟器记录（串口输出）
- 驱动包：`libs/bme280` / `libs/mqtt` 从「空 stub」升为 **host-fn 契约 + 模拟实现 + 测试**（真硬件仍由宿主注入）
- 明确边界文档：哪些 API 必须 host 提供，哪些纯脚本

**验收**：`examples/iot_blink.mai` 有真实运行证据；驱动包测试进 CI。

#### L4. 足迹真实数据（2–3 天）

- 重跑 `benchmark/measure_size.py`（host CLI + bytecode rlib + wasm）
- `docs/guide/footprint.md` 填入实测数；MicroPython 对比仅在 **同目标、同功能** 时写比值，否则继续标注不可比

**验收**：无 placeholder 数字。

#### L5. Parser 错误恢复（3–5 天）

- `Parser::recover_from_error`：语句/声明边界 token-sync
- 一次 parse 收集 **多条** 诊断（LSP/REPL 可见）
- 带错误测试：坏文件仍尽量产出部分 AST

**验收**：故意含 2+ 语法错误的源文件一次报出多点；LSP 不再卡死。

#### L6–L10（按表推进，不阻塞 v0.3.0）

### 11.6 建议节奏（Phase L）

```
Week 1:     L0 文档真相 + L1 发布 v0.3.0（立刻可交付）
Week 2-3:   L2 Register VM 对齐（Match → OOP → 默认参数 → ?/TCO/GC）
Week 3-4:   L3 IoT 真实路径（可与 L2 后半并行）+ L4 足迹数据
Week 5:     L5 错误恢复 + L6 Playground + L7 crates.io
Week 6+:    L8/L10 视需要；L9 async 仅在有场景时立项
```

### 11.7 Phase L 验收（→ v0.3.0 / v0.3.x）

- [ ] 文档与代码一致（无「已有却称没有」）
- [ ] v0.3.0 Release 含 H–K 全部变更 + 二进制
- [ ] `MAILANG_VM=register` 集成测试 **102/102**
- [ ] ESP32 blink 有真实运行证据；驱动包契约测试进 CI
- [ ] 足迹报告无 placeholder
- [ ] Parser 一次报多错；LSP 可用
- [ ] （可选）crates.io 可安装

### 11.8 明确不做（本阶段）

- 完整 async 运行时 / 协程调度（无 IoT 场景前冻结）
- 寄存器 VM 作为默认后端（对齐前不切换）
- 遥程包注册表 SaaS（保持 FS + 静态 HTTP 托管模型）
- `mailang-macros` 为完整而造复杂宏（无需求不写）

---
