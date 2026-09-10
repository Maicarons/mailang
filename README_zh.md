# MaìLang (麦语)

一个为 IoT 和跨平台开发设计的现代编程语言，使用 Rust 编写。

## 特性

- **面向对象编程** - 类、继承、trait 和多态
- **UTF-8 原生支持** - 完整 Unicode 支持，标识符和字符串可使用任何文字
- **基于寄存器的字节码 VM** - 高性能寄存器式虚拟机
- **IoT 就绪** - 三级特性门控，支持嵌入式设备（64KB+）
- **12 种语言 FFI** - 从 C、Python、JavaScript、Java、Go 等调用 MaìLang
- **模式匹配** - 强大的 match 表达式
- **错误处理** - Result/Option 类型安全错误处理
- **Lambda/闭包** - 一等公民函数

## 快速开始

```bash
# 安装
cargo install mailang-cli

# 运行文件
mailang run hello.mai

# REPL 交互
mailang

# 内联代码执行
mailang eval 'println("你好，世界！")'
```

## 示例

```
// hello.mai
fn greet(name = "世界") -> str {
    return "你好，{name}！"
}

println(greet("麦语"))
```

## 项目结构

```
mailang/
├── crates/                    # Rust 代码
│   ├── mailang-lexer/         # 词法分析器
│   ├── mailang-parser/        # 语法分析器
│   ├── mailang-ast/           # AST 定义
│   ├── mailang-analyzer/      # 语义分析
│   ├── mailang-compiler/      # 字节码编译器
│   ├── mailang-bytecode/      # 字节码定义
│   ├── mailang-vm/            # 虚拟机
│   ├── mailang-gc/            # 垃圾回收器
│   ├── mailang-stdlib/        # 标准库
│   ├── mailang-core/          # 核心集成
│   ├── mailang-cli/           # 命令行工具
│   ├── mailang-ffi/           # C FFI 层
│   ├── mailang-wasm/          # WebAssembly 绑定
│   ├── mailang-lsp/           # Language Server Protocol
│   └── mailang-macros/        # 过程宏
├── bindings/                  # 12 种语言绑定示例
├── docs/                      # VitePress 文档
├── examples/                  # 示例程序
└── tests/                     # 集成测试
```

## 构建

```bash
# 默认构建
cargo build

# 发布构建
cargo build --release

# 嵌入式构建
cargo build --no-default-features --target thumbv7em-none-eabihf
```

## 许可证

Apache 2.0 许可证。
