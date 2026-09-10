# MaìLang (麦语) — 全新编程语言设计与实现规划

## Context (背景)

创建一门全新的编程语言 **MaìLang (麦语)**，使用 Rust 编写。要求：
- 支持面向对象编程
- 解释器可运行在全平台（含 IoT 设备）
- 通过 FFI 链接库支持 10+ 语言调用
- 强制 UTF-8 编码，支持任何文字书写
- VitePress 文档，中英双语
- 完整 GitHub 项目结构

参考项目：Rhai (IoT/DSL)、RustPython (Python兼容)、Rune (异步/多线程)、Boa (规范性)、Gluon (类型推导)

---

## 一、语言设计

### 1.1 语言名称与标识
- **名称**: MaìLang (麦语)
- **文件扩展名**: `.mai` (短格式) / `.mailang` (完整格式)
- **Logo/标识**: 码 (Unicode: U+7801)

### 1.2 语法风格（混合式，参考多语言优点）

```
// ===== 变量与类型 =====
let x = 42              // 不可变变量
var y = 100             // 可变变量
const PI = 3.14159      // 常量
let name: str = "麦语"  // 显式类型标注
let nums: [int] = [1, 2, 3]  // 数组

// ===== 函数 =====
fn add(a: int, b: int) -> int {
    return a + b
}
fn greet(name = "世界") -> str {   // 默认参数
    return "你好，{name}！"
}

// ===== 面向对象 =====
class Animal {
    // 属性
    let name: str
    let age: int

    // 构造函数
    fn init(name: str, age: int) {
        this.name = name
        this.age = age
    }

    // 方法
    fn speak() -> str {
        return "{this.name} 发出声音"
    }
}

class Dog extends Animal {
    let breed: str

    fn init(name: str, age: int, breed: str) {
        super(name, age)
        this.breed = breed
    }

    // 方法重写
    override fn speak() -> str {
        return "{this.name} 汪汪叫！"
    }
}

// ===== 接口 =====
trait Printable {
    fn to_string() -> str
    fn print() {
        println(this.to_string())  // 默认实现
    }
}

class Point implements Printable {
    let x: float
    let y: float

    fn to_string() -> str {
        return "({this.x}, {this.y})"
    }
}

// ===== 控制流 =====
if x > 10 {
    println("大于10")
} elif x > 5 {
    println("大于5")
} else {
    println("小于等于5")
}

// match 表达式（模式匹配）
let result = match x {
    0 => "零",
    1..10 => "一到九",
    _ => "其他"
}

// for 循环
for item in [1, 2, 3] {
    println(item)
}
for i in 0..10 {
    println(i)
}
while condition {
    // ...
}

// ===== 错误处理 =====
fn divide(a: float, b: float) -> Result<float, str> {
    if b == 0.0 {
        return Err("除数不能为零")
    }
    return Ok(a / b)
}

let val = divide(10.0, 3.0) match {
    Ok(v) => v,
    Err(e) => {
        println("错误: {e}")
        0.0
    }
}

// ===== 模块系统 =====
import math.{sqrt, PI}
import utils as u

module my_module {
    pub fn hello() {
        println("Hello from my_module")
    }
}

// ===== Lambda / 闭包 =====
let square = fn(x) -> x * x
let nums = [1, 2, 3, 4, 5]
let doubled = nums.map(fn(x) -> x * 2)

// ===== UTF-8 原生支持 =====
let 中文变量 = "支持中文标识符"
let مرحبا = "支持阿拉伯文"
let 🔥 = "支持emoji"
println("你好世界 🌍")
```

### 1.3 类型系统
- **基本类型**: `int` (i64), `float` (f64), `bool`, `str`, `char` (Unicode codepoint)
- **复合类型**: `[T]` (数组), `{K: V}` (字典), `(T1, T2, ...)` (元组)
- **特殊类型**: `null`, `Result<T, E>`, `Option<T>`
- **类型推导**: 支持（类似 Rust/Gluon，let 绑定可省略类型）
- **动态/静态混合**: 默认类型推导，可显式标注

---

## 二、解释器架构

### 2.1 整体架构

```
Source Code (.mai)
    │
    ▼
┌─────────────┐
│   Lexer     │  Unicode-aware tokenizer
│  (词法分析)   │  UTF-8 → Tokens
└─────┬───────┘
      │
      ▼
┌─────────────┐
│   Parser    │  Recursive descent + Pratt
│  (语法分析)   │  Tokens → AST
└─────┬───────┘
      │
      ▼
┌─────────────┐
│  Analyzer   │  语义分析、类型检查
│  (语义分析)   │  AST → Typed AST
└─────┬───────┘
      │
      ▼
┌─────────────┐
│  Compiler   │  字节码编译器
│  (编译器)     │  Typed AST → Bytecode
└─────┬───────┘
      │
      ▼
┌─────────────┐
│     VM      │  基于寄存器的字节码虚拟机
│  (虚拟机)     │  执行 Bytecode
└─────────────┘
```

### 2.2 执行模型选择
**基于寄存器的字节码 VM** (参考 Boa)
- 比树遍历（Rhai）快，比栈式 VM（Rune）更高效
- 字节码可序列化缓存，加速启动
- 适合 IoT 场景（减少内存访问）

### 2.3 内存管理
- **标准模式** (`std`): 引用计数 + 循环检测 GC
- **嵌入式模式** (`no_std + alloc`): 仅引用计数（无 GC，手动管理循环）
- **最小模式** (`no_std`): 无堆分配，仅栈上值类型

### 2.4 Crate 架构（Cargo Workspace）

```
mailang/
├── crates/
│   ├── mailang-lexer/       # 词法分析器
│   ├── mailang-parser/      # 语法分析器 → AST
│   ├── mailang-ast/         # AST 定义
│   ├── mailang-analyzer/    # 语义分析、类型检查
│   ├── mailang-compiler/    # 字节码编译器
│   ├── mailang-bytecode/    # 字节码定义
│   ├── mailang-vm/          # 虚拟机执行器
│   ├── mailang-gc/          # 垃圾回收器
│   ├── mailang-stdlib/      # 标准库
│   ├── mailang-core/        # 核心库（整合以上所有）
│   ├── mailang-cli/         # 命令行工具 (REPL + 文件执行)
│   ├── mailang-ffi/         # C FFI 层 (extern "C" API)
│   ├── mailang-wasm/        # WebAssembly 绑定
│   ├── mailang-lsp/         # Language Server Protocol
│   └── mailang-macros/      # 过程宏（Rust 侧 API 简化）
```

---

## 三、IoT/嵌入式支持策略

### 3.1 三级特性门控

```toml
# Cargo.toml features
[features]
default = ["std", "gc", "io", "net", "fs"]
std = ["alloc"]                    # 标准库支持
alloc = []                         # 堆分配支持（无 std）
gc = ["alloc"]                     # 垃圾回收
io = ["std"]                       # 文件/网络 I/O
net = ["std"]                      # 网络
fs = ["std"]                       # 文件系统
math = []                          # 数学库（可选）
serde = ["dep:serde"]             # 序列化支持
```

### 3.2 嵌入式目标架构

| 目标 | Rust Target | 最低内存 | 说明 |
|------|-------------|---------|------|
| ARM Cortex-M0 | `thumbv6m-none-eabi` | 64KB | 最小 IoT 芯片 |
| ARM Cortex-M4 | `thumbv7em-none-eabihf` | 256KB | 常见 MCU |
| RISC-V | `riscv32imc-unknown-none-elf` | 128KB | ESP32-C3 等 |
| ESP32 (Xtensa) | `xtensa-esp32-none-elf` | 512KB | ESP32 系列 |
| WASM | `wasm32-unknown-unknown` | N/A | 浏览器/边缘 |
| Linux ARM | `aarch64-unknown-linux-gnu` | 16MB | 树莓派等 |
| Linux MIPS | `mipsel-unknown-linux-gnu` | 16MB | 路由器等 |

### 3.3 二进制体积优化

```toml
[profile.release]
opt-level = "z"
lto = "fat"
codegen-units = 1
panic = "abort"
strip = "symbols"
```

---

## 四、FFI 多语言绑定（12 种语言）

### 4.1 架构设计

```
┌──────────────────────────────────────────────┐
│           Language-Specific Wrappers          │
│  PyO3  │ Neon │ JNI │ UniFFI │ wasm-bindgen  │
├──────────────────────────────────────────────┤
│           C ABI Layer (mailang-ffi)           │
│     extern "C" + cbindgen → mailang.h         │
├──────────────────────────────────────────────┤
│           mailang-core (Rust Library)         │
└──────────────────────────────────────────────┘
```

### 4.2 C FFI API 设计

```c
// mailang.h (由 cbindgen 自动生成)

typedef struct MailangInterpreter MailangInterpreter;

typedef struct {
    int32_t code;       // 0=成功, 非0=错误
    char* output;       // 输出字符串（调用方需释放）
} MailangResult;

// 生命周期
MailangInterpreter* mailang_create(void);
void mailang_destroy(MailangInterpreter* interp);

// 执行
MailangResult mailang_eval(MailangInterpreter* interp, const char* code);
MailangResult mailang_eval_file(MailangInterpreter* interp, const char* path);

// 变量操作
MailangResult mailang_get_var(MailangInterpreter* interp, const char* name);
void mailang_set_var(MailangInterpreter* interp, const char* name, const char* value_json);

// 函数调用
MailangResult mailang_call(MailangInterpreter* interp, const char* func, const char* args_json);

// 错误处理
const char* mailang_last_error(MailangInterpreter* interp);

// 内存管理
void mailang_free_string(char* ptr);
```

### 4.3 12 种语言绑定方案

| # | 语言 | FFI 机制 | 工具 | Demo 目录 |
|---|------|---------|------|-----------|
| 1 | **C/C++** | 原生链接 | cbindgen 生成头文件 | `bindings/c/` |
| 2 | **Python** | CPython C API | PyO3 + maturin | `bindings/python/` |
| 3 | **JavaScript/Node.js** | Node-API | napi-rs | `bindings/nodejs/` |
| 4 | **TypeScript** | 同 JS + .d.ts | napi-rs 自动生成 | `bindings/typescript/` |
| 5 | **Java** | JNI | jni crate | `bindings/java/` |
| 6 | **Kotlin** | JNI | jni crate + UniFFI | `bindings/kotlin/` |
| 7 | **C# / .NET** | P/Invoke | C FFI 直接调用 | `bindings/csharp/` |
| 8 | **Go** | cgo | C FFI + cgo | `bindings/go/` |
| 9 | **Ruby** | FFI gem | UniFFI 或 C FFI | `bindings/ruby/` |
| 10 | **Swift** | C bridging | UniFFI | `bindings/swift/` |
| 11 | **PHP** | ext-ffi | C FFI + FFI::load() | `bindings/php/` |
| 12 | **Lua** | Lua C API | mlua crate | `bindings/lua/` |

### 4.4 每个 Demo 的标准内容

每个语言 Demo 包含：
1. `README.md` — 该语言的接入说明
2. 构建脚本/配置文件
3. `demo.mai` — 示例 MaìLang 脚本
4. `main.*` — 调用 MaìLang 的宿主语言代码
5. 运行结果截图/输出

---

## 五、GitHub 项目结构

```
mailang/
├── .github/
│   ├── workflows/
│   │   ├── ci.yml                    # PR/push: fmt, clippy, test
│   │   ├── release.yml               # release-please + crates.io
│   │   ├── cross.yml                 # 交叉编译测试 (ARM, RISC-V)
│   │   ├── docs.yml                  # VitePress 构建与部署
│   │   └── bindings.yml              # 各语言绑定测试
│   ├── ISSUE_TEMPLATE/
│   │   ├── bug_report.yml
│   │   ├── feature_request.yml
│   │   └── config.yml
│   ├── PULL_REQUEST_TEMPLATE.md
│   ├── dependabot.yml
│   └── CODEOWNERS
│
├── crates/                           # Rust 代码
│   ├── mailang-lexer/
│   ├── mailang-parser/
│   ├── mailang-ast/
│   ├── mailang-analyzer/
│   ├── mailang-compiler/
│   ├── mailang-bytecode/
│   ├── mailang-vm/
│   ├── mailang-gc/
│   ├── mailang-stdlib/
│   ├── mailang-core/
│   ├── mailang-cli/
│   ├── mailang-ffi/
│   ├── mailang-wasm/
│   ├── mailang-lsp/
│   └── mailang-macros/
│
├── bindings/                         # 12 种语言的绑定 Demo
│   ├── c/
│   ├── python/
│   ├── nodejs/
│   ├── typescript/
│   ├── java/
│   ├── kotlin/
│   ├── csharp/
│   ├── go/
│   ├── ruby/
│   ├── swift/
│   ├── php/
│   └── lua/
│
├── docs/                             # VitePress 文档
│   ├── .vitepress/
│   │   └── config.ts
│   ├── guide/                        # 中文（根语言）
│   │   ├── index.md
│   │   ├── getting-started.md
│   │   ├── syntax.md
│   │   ├── oop.md
│   │   ├── stdlib.md
│   │   ├── ffi.md
│   │   └── iot.md
│   ├── reference/                    # 中文参考手册
│   │   ├── index.md
│   │   ├── types.md
│   │   ├── operators.md
│   │   ├── builtins.md
│   │   └── errors.md
│   ├── en/                           # 英文文档
│   │   ├── guide/
│   │   └── reference/
│   └── api/                          # API 文档（自动生成）
│
├── examples/                         # MaìLang 示例程序
│   ├── hello.mai
│   ├── fibonacci.mai
│   ├── oop_demo.mai
│   ├── http_server.mai
│   └── iot_blink.mai
│
├── tests/                            # 集成测试
│   ├── snapshots/
│   └── integration/
│
├── scripts/                          # 构建辅助脚本
│   ├── build-bindings.sh
│   ├── build-wasm.sh
│   └── release.sh
│
├── Cargo.toml                        # Workspace 根
├── Cargo.lock
├── rust-toolchain.toml
├── Cross.toml
├── cbindgen.toml
├── LICENSE-MIT
├── LICENSE-APACHE
├── README.md
├── README_zh.md
├── CHANGELOG.md
├── CONTRIBUTING.md
└── CODE_OF_CONDUCT.md
```

---

## 六、VitePress 文档结构

### 6.1 语言配置

```ts
// docs/.vitepress/config.ts
export default defineConfig({
  locales: {
    root: {
      label: '简体中文',
      lang: 'zh-CN',
      themeConfig: { /* 中文导航/侧边栏 */ }
    },
    en: {
      label: 'English',
      lang: 'en',
      themeConfig: { /* English nav/sidebar */ }
    }
  }
})
```

### 6.2 文档目录

| 章节 | 中文路径 | 英文路径 | 内容 |
|------|---------|---------|------|
| 介绍 | `/guide/` | `/en/guide/` | 语言概述、设计理念 |
| 快速开始 | `/guide/getting-started` | `/en/guide/getting-started` | 安装、Hello World |
| 语法指南 | `/guide/syntax` | `/en/guide/syntax` | 完整语法参考 |
| 面向对象 | `/guide/oop` | `/en/guide/oop` | 类、继承、接口、trait |
| 标准库 | `/guide/stdlib` | `/en/guide/stdlib` | 内置模块文档 |
| FFI 接入 | `/guide/ffi` | `/en/guide/ffi` | 各语言接入指南 |
| IoT 部署 | `/guide/iot` | `/en/guide/iot` | 嵌入式编译与部署 |
| API 参考 | `/reference/` | `/en/reference/` | 类型、运算符、内置函数 |

---

## 七、实施计划（分阶段）

### Phase 1: 项目骨架 + 核心解释器（第 1-4 周）
1. 初始化 Cargo workspace，创建所有 crate 骨架
2. 实现 Lexer（Unicode-aware tokenizer）
3. 实现 Parser（递归下降 + Pratt parsing）→ AST
4. 实现基础 Compiler → 字节码
5. 实现基础 VM（算术、变量、函数调用）
6. 实现 CLI（REPL + 文件执行）
7. 编写基础测试

### Phase 2: OOP + 标准库（第 5-8 周）
8. 实现类系统（class, extends, this）
9. 实现接口/trait 系统
10. 实现标准库核心模块（io, math, string, collections）
11. 实现 GC（引用计数 + 循环检测）
12. 实现模式匹配
13. 实现错误处理（Result/Option）

### Phase 3: FFI + 多语言绑定（第 9-12 周）
14. 实现 C FFI 层（mailang-ffi）
15. 配置 cbindgen 自动生成头文件
16. 编写 12 种语言的绑定 Demo
17. 实现 WASM 绑定（mailang-wasm）

### Phase 4: 文档 + CI/CD（第 13-16 周）
18. 搭建 VitePress 文档站点
19. 编写中文文档（全部章节）
20. 编写英文文档（全部章节）
21. 配置 GitHub Actions CI/CD
22. 配置交叉编译（ARM, RISC-V）
23. 优化二进制体积

### Phase 5: LSP + 生态（第 17-20 周）
24. 实现 LSP server
25. VS Code 扩展
26. 包管理器设计
27. 发布 v0.1.0

---

## 八、关键技术决策

| 决策 | 选择 | 理由 |
|------|------|------|
| 实现语言 | Rust | 性能、安全、跨平台、嵌入式支持 |
| 执行模型 | 寄存器字节码 VM | 性能优于树遍历，效率优于栈式 |
| 内存管理 | RC + GC 混合 | IoT 用 RC，桌面用 GC |
| 编码 | 强制 UTF-8 | 全语言支持，无 locale 依赖 |
| FFI 基础 | C ABI | 通用性最强，所有语言都支持 |
| 文档框架 | VitePress | 快速、i18n 支持、Vue 生态 |
| 许可证 | MIT + Apache 2.0 | Rust 生态惯例 |

---

## 九、验证方案

1. **单元测试**: 每个 crate 独立测试，覆盖率 > 80%
2. **集成测试**: `tests/` 目录下的端到端测试
3. **快照测试**: 字节码输出的快照比较
4. **交叉编译验证**: CI 中用 `cross` 测试 ARM/RISC-V 目标
5. **绑定测试**: 每种语言的 Demo 必须可编译运行
6. **文档测试**: VitePress 构建无错误，所有链接有效
7. **基准测试**: 与 Rhai、Rune 的性能对比
