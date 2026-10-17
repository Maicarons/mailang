# MaìLang (麦语) 项目报告

> **项目链接**：[GitHub](https://github.com/Maicarons/mailang) · [Releases](https://github.com/Maicarons/mailang/releases) · [Tags](https://github.com/Maicarons/mailang/tags)
>
> **状态：Phase N（栈式 VM 与寄存器 VM 均 135/135；`benchmark/compare_backends.ps1` 64/64 一致）**
>
> 本报告如实反映项目实际状态。阶段规划见 [ROADMAP.md](ROADMAP.md)，变更见 [CHANGELOG.md](CHANGELOG.md)。

## 一、项目概述

MaìLang 是一门为 IoT 和跨平台开发设计的现代编程语言，使用 Rust 编写。

## 二、实际可用功能（运行时验证）

- ✅ 算术运算（int/float）、字符串、布尔、null
- ✅ 变量声明（let/var/const）与 **可变性强制**（`let`/`const` 拒绝赋值）
- ✅ 函数定义、递归、默认参数、尾调用优化
- ✅ Lambda / 闭包捕获
- ✅ if/elif/else、while/for、范围迭代
- ✅ 字符串插值 `"Hello, {name}!"`
- ✅ 数组/字典/元组；集合与字符串方法（push/keys/trim/split 等）
- ✅ 模式匹配：字面量/范围/`Ok`/`Err`/`Some`/或/守卫 + **数组/元组解构**；后缀 `expr match { }`
- ✅ `let`/`var` 解构；match 绑定作用域隔离
- ✅ OOP：类/继承/方法/`super()`/`super.method()`/继承构造与方法
- ✅ Trait：`implements`、默认方法、`trait extends`
- ✅ Result/Option + **`?` 错误传播**
- ✅ **泛型函数 monomorphization**（turbofish `id::<int>` → `id$int`）；泛型类字段擦除
- ✅ 模块系统 v2 + `mailang.toml` 路径依赖（`mailang deps`）
- ✅ **包注册表**（FS / 静态 HTTP）：`publish` / `install` / `search` / `yank` / `registry`
- ✅ GC：Rc + `collect_cycles()` + **MarkSweepHeap**（栈 VM；`gc_stats()`）
- ✅ **解析器错误恢复**（`recover_from_error` / `parse_program_recovering`；LSP 多语法错误）
- ✅ 字节码 `.mailangbc`；模拟 HAL（gpio/delay/adc）
- ✅ C FFI / WASM / CLI(run/eval/build/fmt/deps/registry/lsp) / LSP / 格式化器；CLI 在 Windows 上内置 `--host=sim|tui|gui` 真实后端（`libs/tui`、`libs/gui` 两个 UI 包）
- ✅ UTF-8 标识符；十六进制/八进制/二进制字面量；块注释

**测试**：栈式 VM（默认）与寄存器 VM（`MAILANG_VM=register` / `--vm=register`）集成均为 **135/135**；全工作区 **253** 项测试全绿；`benchmark/compare_backends.ps1` 64/64 一致，`benchmark/known_backend_mismatches.txt` 为空。

## 三、已知限制

- ❌ **`async`** — 无 async/await 运行时或语法
- ⚠️ **寄存器 VM**（`MAILANG_VM=register` / `--vm=register`）— 与栈式 VM 在每个 shipped 程序上一致（135/135 集成 + `compare_backends.ps1` 64/64）且 `bench_fib30` 与之接近（~1.0x，远低于 ≤2.0x 门禁），但默认后端仍为栈式 VM
- ⚠️ **泛型类** — 字段擦除布局，无按类型 monomorphization
- ⚠️ **IoT HAL** — 默认模拟实现；真实硬件需通过 FFI 注册 HAL（不声称真实硬件验证）
- ⚠️ **公共托管注册表** — 仅本地目录 / 静态 HTTP 镜像
- ⚠️ **mailang-macros** — 仍为 passthrough 桩（无调用方）
- ✅ **栈式后端若干参数形状报错（M8b）已修**（Phase N 之后）：`CallDirect` 帧没有 callee 槽位，而 `Opcode::TailCall` 一律按 `Call` 布局把 `callee + 实参` 搬到 `stack_base - 1`，压到调用者的活槽位；内建尾调用的收尾还直接 `truncate(stack_base - 1)` 多丢一格。此前表现为 5 参函数里 `y + h - 1` 抛 `Cannot add these types`、把参数继续下传报 `len expects …`、`println(f(x))`（`f` 以尾调用收尾）报 `Stack underflow in Call`，以及同一形状两后端算出不同的值。现两个后端在全部四种形状上一致，5 个回归测试，见 `ROADMAP.md` 13.9
- ✅ **寄存器后端不接管 `libs/tui`+`libs/gui` 宿主钩子（M8a）已修**（Phase N 之后）：`RegOp::TailCall` 对同名函数跳过了 `host_fns`，`host_*` 套件在 `--vm=register` 下失败；现两个后端都 7/7，见 `ROADMAP.md` 13.8
- ✅ **分配密集的循环不再泄漏** — Phase N N4 把 census 阈值改为 `max(MIN, live*2)`（原为无上限 ×1.5）并修好寄存器 VM 的新生节点不是 GC 根的问题；同一分配循环 200 万/400 万次峰值 4.9 MB（栈式）/ 5.1 MB（寄存器），不再是 151/333 MB。门禁 `benchmark/measure_memory.ps1`

## 四、Crate 状态

| Crate | 状态 | 说明 |
|-------|------|------|
| mailang-lexer | ✅ 完整 | Unicode-aware |
| mailang-ast | ✅ 完整 | 含泛型类型节点 |
| mailang-parser | ✅ 完整 | 递归下降 + Pratt；**含错误恢复**（`parse_program_recovering`） |
| mailang-bytecode | ✅ 完整 | 栈 IR + 寄存器 IR |
| mailang-compiler | ✅ 完整 | 含泛型 monomorphization |
| mailang-vm | ✅ 完整 | 栈式（默认）+ 寄存器后端，均 135/135 |
| mailang-stdlib | ✅ 可用 | io/math/string/collections/json/time/HAL/sys |
| mailang-core | ✅ 管道集成 | Parser → Compiler → VM |
| mailang-cli | ✅ 完整 | run/eval/build/fmt/deps/registry/lsp |
| mailang-ffi | ✅ 可用 | host-fn 注册 + 全局读写 |
| mailang-wasm | ✅ 可用 | eval / eval_json |
| mailang-module | ✅ 完整 | 模块 v2 + 路径依赖 + 注册表 |
| mailang-analyzer | ✅ 已接入 | 诊断 / arity / 简单类型 |
| mailang-gc | ✅ 已接入 | Rc 断环 + MarkSweepHeap |
| mailang-lsp | ✅ 基本可用 | 诊断/补全/跳转/悬停 |
| mailang-macros | ⚠️ 桩 | passthrough，无真实宏 |

## 五、性能

| 指标 | 结果 |
|------|------|
| Fibonacci(30) | ~137 ms（约 4.25× Phase A 基线加速） |

详见 [benchmark/REPORT.md](benchmark/REPORT.md)。

## 六、技术决策

| 决策 | 选择 | 说明 |
|------|------|------|
| 实现语言 | Rust | 性能、安全、跨平台 |
| 默认执行模型 | 栈式 VM + slot locals | 寄存器 VM 为可选后端（特性对齐，默认栈式） |
| 内存管理 | Rc + mark-sweep | 增量 GC 未实现 |
| 泛型 | 函数 monomorphization | 类字段擦除 |
| 错误处理 | Result + `?` | Rust 风格 |
| 模块/包 | 文件即模块 + FS 注册表 | 可静态 HTTP 托管 |
| FFI | C ABI + cbindgen | 通用性最强 |
| 许可证 | Apache-2.0 | 单一许可证 |

## 七、下一步

见 [ROADMAP.md](ROADMAP.md) **Phase N**：双后端一致基线已落地（D1–D4 全修，`compare_backends.ps1` 64/64）；后续为 IoT 真实路径与足迹数据、寄存器 VM 的长期维护与默认后端切换评估。

---

**当前状态：栈式 VM 与寄存器 VM 均通过 135/135 集成测试，且在 `examples/`+`libs/` 的全部 64 个 `.mai` 程序上逐字节一致；IoT 默认模拟 HAL。**
