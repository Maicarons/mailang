# MaìLang (麦语) 项目报告

> **项目链接**：[GitHub](https://github.com/Maicarons/mailang) · [v0.1.0](https://github.com/Maicarons/mailang/releases/tag/v0.1.0) · [Tags](https://github.com/Maicarons/mailang/tags)
>
> **状态：Phase A 完成（OOP、闭包、模式匹配已实现，40/40 测试通过）**
> 
> 本报告如实反映项目实际状态。详细差距分析见 [ROADMAP.md](ROADMAP.md)。

## 一、项目概述

MaìLang 是一门为 IoT 和跨平台开发设计的现代编程语言，使用 Rust 编写。

## 二、实际可用功能

以下功能经过运行时验证，**确实可用**：

- ✅ 算术运算（int/float）、字符串、布尔、null
- ✅ 变量声明（let/var/const）与赋值
- ✅ 函数定义与递归调用
- ✅ 无捕获的 Lambda 表达式
- ✅ if/elif/else 条件分支
- ✅ while/for 循环（含范围迭代）
- ✅ 字符串插值 `"Hello, {name}!"`
- ✅ 数组与字典（基础索引）
- ✅ 字面量 match 表达式（无逗号分隔）
- ✅ 模块导入（命名 + 相对路径）
- ✅ UTF-8 标识符（unicode-xid）
- ✅ CLI（run/eval/REPL）
- ✅ C FFI（eval/eval_file）
- ✅ WASM 绑定（eval/eval_json）

## 三、已知不工作的功能

以下功能在文档中被提及但**实际不可用**：

- ❌ **OOP**：类解析通过，但方法体未编译，实例化 → Stack underflow
- ❌ **Trait**：`compile_trait` 为空函数
- ❌ **闭包**：Upvalue 捕获为桩，闭包返回 null
- ❌ **模式匹配**：`MatchPattern` 永远返回 true，通配符 `_` 不工作
- ❌ **Result/Option**：编译为数组，无真实语义
- ❌ **Analyzer**：738 行代码从未被调用
- ❌ **GC**：84 行空壳，VM 从未使用
- ❌ **测试**：0 个测试实际运行（tests/ 在 workspace 根目录未被拾取）
- ❌ **十六进制/八进制/二进制字面量**
- ❌ **块注释 `*/` 终止符**（使用 `**` 而非 `*/`）
- ❌ **no_std / 嵌入式目标**：无 `#![no_std]`，feature flag 无效

## 四、Crate 状态

| Crate | 状态 | 说明 |
|-------|------|------|
| mailang-lexer | ✅ 真实完整 | Unicode-aware，支持所有 token 类型 |
| mailang-ast | ✅ 真实完整 | 完整数据定义 + serde |
| mailang-parser | ✅ 真实完整 | 递归下降 + Pratt，优先级正确 |
| mailang-bytecode | ✅ 真实完整 | 50+ 指令定义 |
| mailang-compiler | ⚠️ 70% | 核心表达式/循环/函数可用；OOP/trait/闭包不完整 |
| mailang-vm | ⚠️ 60% | 栈式执行可用；类/闭包/模式匹配为桩 |
| mailang-stdlib | ⚠️ 40% | 基础函数可用；缺文件/网络/集合操作 |
| mailang-core | ✅ 可用 | 管道集成；跳过 Analyzer |
| mailang-cli | ✅ 真实完整 | run/eval/REPL |
| mailang-ffi | ⚠️ 50% | eval 可用；缺 get_var/set_var/call |
| mailang-wasm | ⚠️ 可用 | eval/eval_json 可用 |
| mailang-module | ⚠️ 部分 | 文件加载可用；AST 注入方式 |
| mailang-analyzer | ❌ 未接入 | 代码存在但从未调用 |
| mailang-gc | ❌ 空壳 | 无真实 GC |
| mailang-lsp | ❌ 桩 | 仅 initialize/shutdown |
| mailang-macros | ❌ 桩 | 仅 passthrough |

## 五、性能基准

来自 `benchmark/REPORT.md` 的实测数据：

| 基准 | MaìLang | QuickJS | Python | 差距 |
|------|---------|---------|--------|------|
| Fibonacci(30) | 582ms | 71ms | 1,200ms | 8.2x 慢于 QuickJS |
| 字符串操作 | 中等 | 快 | 慢 | ~2x 慢 |
| 循环 | 中等 | 快 | 慢 | ~3x 慢 |

**主要瓶颈**：Value 深拷贝（每次访问都 `.clone()`）。

## 六、技术决策

| 决策 | 选择 | 说明 |
|------|------|------|
| 实现语言 | Rust | 性能、安全、跨平台 |
| 执行模型 | 栈式 VM + slot locals | **修正**：之前文档声称"寄存器 VM" |
| 内存管理 | RC 设计（GC 未实现） | 分阶段实现 |
| 编码 | 强制 UTF-8 | 全语言支持 |
| FFI 基础 | C ABI + cbindgen | 通用性最强 |
| 许可证 | Apache-2.0 | 单一许可证 |

## 七、下一步计划

详见 [ROADMAP.md](ROADMAP.md)，核心优先级：

1. **Phase A**（2-4周）：修复测试基础设施、实现类系统/闭包/模式匹配、接入 Analyzer、诚实审计文档
2. **Phase B**（2-3周）：Value→Rc 消除 clone、全局槽表、基准测试
3. **Phase C**（3-4周）：真实 no_std、字节码文件格式、体积测量
4. **Phase D**（2-3周）：扩展 C API、FFI 验证、WASM
5. **Phase E**（持续）：LSP、格式化器、模块系统 v2

---

**当前状态：早期原型。核心管道可用，高级特性为桩。**
