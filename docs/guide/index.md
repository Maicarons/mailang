# MaìLang 介绍

> **项目链接**：[GitHub 仓库](https://github.com/Maicarons/mailang) · [v0.1.0 发布](https://github.com/Maicarons/mailang/releases/tag/v0.1.0) · [在线 Playground](https://maicarons.github.io/mailang/playground) · [Issue 反馈](https://github.com/Maicarons/mailang/issues)

## 什么是 MaìLang？

MaìLang（麦语）是一门为 **IoT 和跨平台开发** 设计的现代编程语言，使用 Rust 编写。它结合了多种编程语言的优点，提供了简洁、高效、安全的编程体验。

**名称含义**：「麦」取自小麦（Wheat），象征着语言的朴实与普适；「语」代表语言。Maì 中的 `ì` 带有声调，致敬中文的声调系统。

## 核心特性

| 特性 | 说明 | 状态 |
|------|------|------|
| 面向对象编程 | 类、继承、构造函数、`super()` | ✅ 已实现 |
| 闭包 | upvalue 捕获，一等公民函数 | ✅ 已实现 |
| 模式匹配 | 字面量、通配符、or-pattern、守卫 | ✅ 已实现 |
| UTF-8 原生 | 标识符支持任何 Unicode 文字 | ✅ 已实现 |
| 栈式字节码 VM | 基于 slot 的高性能执行 | ✅ 已实现 |
| C FFI | 12 种语言绑定支持 | ✅ 已实现 |
| WebAssembly | 浏览器/边缘运行 | ✅ 已实现 |
| CLI + REPL | 命令行工具 | ✅ 已实现 |

## 项目资源

- **源代码**：[github.com/Maicarons/mailang](https://github.com/Maicarons/mailang)
- **版本发布**：[Releases](https://github.com/Maicarons/mailang/releases)
- **项目标签**：[Tags](https://github.com/Maicarons/mailang/tags)
- **贡献指南**：[CONTRIBUTING.md](https://github.com/Maicarons/mailang/blob/master/CONTRIBUTING.md)
- **路线图**：[ROADMAP.md](https://github.com/Maicarons/mailang/blob/master/ROADMAP.md)
- **项目报告**：[REPORT.md](https://github.com/Maicarons/mailang/blob/master/REPORT.md)

## 设计理念

### 简洁易学
MaìLang 采用混合式语法风格，借鉴了 Rust、Python、JavaScript 等语言的优点：
- 类 Rust 的变量声明和类型系统
- 类 Python 的简洁函数语法
- 类 JavaScript 的对象操作

### 高性能
采用 **基于寄存器的字节码虚拟机**，比树遍历解释器（如 Rhai）更快，比栈式虚拟机（如 Rune）更高效：
- 字节码可序列化缓存，加速启动
- 减少内存访问，适合 IoT 场景
- 寄存器分配优化

### 全平台支持
通过三级特性门控（Feature Gating），MaìLang 可运行在从 64KB IoT 芯片到桌面服务器的所有平台：

| 级别 | 内存需求 | 典型设备 | 特性 |
|------|---------|---------|------|
| `no_std` | 64KB | ARM Cortex-M0 | 仅栈上值类型 |
| `no_std + alloc` | 128KB | RISC-V, ESP32 | 引用计数，无 GC |
| `std` | 256KB+ | 树莓派, 服务器 | 完整功能 |

### UTF-8 原生支持
MaìLang 强制使用 UTF-8 编码，标识符支持任何 Unicode 文字：
```
let 中文变量 = "支持中文"
let مرحبا = "支持阿拉伯文"
let 🔥 = "支持emoji"
```

### 安全可靠
- 强类型系统，类型推导可选
- Result/Option 错误处理，无异常
- 所有权系统（可选）

## 与其他语言对比

| 特性 | MaìLang | Rhai | Rune | RustPython |
|------|---------|------|------|------------|
| 执行模型 | 寄存器 VM | 树遍历 | 栈式 VM | 栈式 VM |
| OOP 支持 | ✅ 完整 | ❌ 有限 | ❌ 有限 | ✅ 完整 |
| 嵌入式支持 | ✅ 64KB+ | ✅ | ❌ | ❌ |
| FFI | ✅ 12 语言 | ✅ Rust | ❌ | ✅ Python |
| 类型系统 | 混合 | 动态 | 动态 | 动态 |
| 模式匹配 | ✅ | ❌ | ✅ | ❌ |
| 异步支持 | 计划中 | ❌ | ✅ | ❌ |

## 适用场景

### IoT 与嵌入式
- 传感器数据处理
- 设备配置脚本
- 边缘计算逻辑

### 游戏开发
- 游戏逻辑脚本
- AI 行为树
- 配置文件解析

### 自动化
- 数据处理管道
- 系统管理脚本
- 测试自动化

### 教学
- 编程入门教学
- 语言设计学习
- 编译原理实践

## 快速开始

```bash
# 安装
cargo install mailang-cli

# REPL
mailang

# 运行文件
mailang run hello.mai

# 内联执行
mailang eval 'println("你好，MaìLang！")'
```

## 下一步

- [快速开始](/guide/getting-started) - 安装和第一个程序
- [语法指南](/guide/syntax) - 完整语法参考
- [面向对象](/guide/oop) - 类、继承、trait
- [标准库](/guide/stdlib) - 内置模块文档
- [FFI 接入](/guide/ffi) - 各语言接入指南
- [IoT 部署](/guide/iot) - 嵌入式编译与部署
