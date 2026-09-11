# WASM 集成指南

> **项目链接**：[GitHub](https://github.com/Maicarons/mailang) · [WASM 源码](https://github.com/Maicarons/mailang/tree/master/crates/mailang-wasm) · [Playground](https://maicarons.github.io/mailang/playground)

## 概述

MaìLang 支持编译为 WebAssembly (WASM)，可在浏览器和 Node.js 中运行。本指南介绍如何构建、使用和优化 MaìLang 的 WASM 版本。

## 构建 WASM

### 前置条件

```bash
# 安装 Rust WASM 目标
rustup target add wasm32-unknown-unknown

# 安装 wasm-pack
cargo install wasm-pack
```

### 构建命令

```bash
# 构建 WASM 包（浏览器版）
wasm-pack build --target web --release -p mailang-wasm

# 构建 WASM 包（Node.js 版）
wasm-pack build --target nodejs --release -p mailang-wasm

# 构建 WASM 包（ bundler 版，用于 Vite/Webpack）
wasm-pack build --target bundler --release -p mailang-wasm
```

构建产物位于 `crates/mailang-wasm/pkg/` 目录。

## 在浏览器中使用

### 基本用法

```html
<!DOCTYPE html>
<html>
<head>
    <title>MaìLang WASM</title>
</head>
<body>
    <script type="module">
        import init, { WasmInterpreter } from './pkg/mailang_wasm.js';

        async function main() {
            // 初始化 WASM 模块
            await init();

            // 创建解释器
            const interp = new WasmInterpreter();

            // 执行代码
            const result = interp.eval('1 + 2');
            console.log(result);  // "3"

            // 执行多行代码
            const output = interp.eval(`
                fn greet(name) {
                    return "你好，{name}！"
                }
                println(greet("MaìLang"))
            `);
            console.log(output);

            // 释放资源
            interp.free();
        }

        main();
    </script>
</body>
</html>
```

### 结构化结果

```javascript
// 使用 eval_json 获取结构化结果
const jsonStr = interp.eval_json('1 + 2');
const result = JSON.parse(jsonStr);

if (result.ok) {
    console.log('输出:', result.output);
} else {
    console.error('错误:', result.error);
}
```

### 重置解释器

```javascript
// 重置解释器状态（清除全局变量等）
interp.reset();
```

## 在 Node.js 中使用

```javascript
const { WasmInterpreter } = require('./pkg/mailang_wasm.js');

const interp = new WasmInterpreter();

console.log(interp.eval('println("Hello from Node.js!")'));

interp.free();
```

## 在 Vue/React 中使用

### Vue 3 组合式 API

```vue
<script setup>
import { ref, onMounted } from 'vue';
import init, { WasmInterpreter } from './pkg/mailang_wasm.js';

const output = ref('');
const error = ref('');
let interp = null;

onMounted(async () => {
    await init();
    interp = new WasmInterpreter();
});

function runCode(code) {
    try {
        const result = interp.eval(code);
        if (result.startsWith('Error:')) {
            error.value = result;
            output.value = '';
        } else {
            output.value = result;
            error.value = '';
        }
    } catch (e) {
        error.value = e.message;
    }
}
</script>
```

### React Hook

```jsx
import { useState, useEffect } from 'react';
import init, { WasmInterpreter } from './pkg/mailang_wasm.js';

function useMailang() {
    const [interp, setInterp] = useState(null);

    useEffect(() => {
        init().then(() => {
            setInterp(new WasmInterpreter());
        });
    }, []);

    const eval_ = (code) => {
        if (!interp) return 'Loading...';
        return interp.eval(code);
    };

    return { eval: eval_, ready: !!interp };
}
```

## API 参考

### WasmInterpreter

#### 构造函数

```javascript
const interp = new WasmInterpreter();
```

创建一个新的解释器实例。

#### eval(code: string): string

执行 MaìLang 代码并返回结果。

- 成功时返回结果字符串
- 失败时返回 `"Error: ..."` 格式的错误信息

#### eval_json(code: string): string

执行代码并返回 JSON 格式的结果。

```json
// 成功
{"ok": true, "output": "3"}

// 失败
{"ok": false, "error": "Type error: ..."}
```

#### reset(): void

重置解释器状态，清除所有全局变量。

#### free(): void

释放解释器占用的 WASM 内存。

#### static version(): string

返回 MaìLang 版本号。

#### static features(): string

返回支持的特性列表。

## Playground

MaìLang 提供了在线 Playground，基于 Vue + WASM 构建：

```bash
# 构建 Playground
cd playground
npm install
npm run build:wasm  # 构建 WASM
npm run dev          # 启动开发服务器
```

Playground 功能：
- 代码编辑器（CodeMirror，语法高亮）
- 实时执行
- 内置示例
- 错误提示

## 性能优化

### WASM 体积优化

```toml
# Cargo.toml
[profile.release]
opt-level = "z"        # 优化体积
lto = "fat"            # 链接时优化
codegen-units = 1
strip = "symbols"
```

### 预期体积

| 配置 | WASM 体积 |
|------|----------|
| 完整版 | ~2MB |
| 精简版 | ~800KB |

### 启动优化

```javascript
// 预加载 WASM 模块
const wasmReady = init();

async function run(code) {
    await wasmReady;  // 确保 WASM 已加载
    return interp.eval(code);
}
```

## 限制

- 无文件系统访问（浏览器安全限制）
- 无网络请求
- 无 `input()` 函数（无标准输入）
- 内存受限（默认 256MB WASM 内存上限）

## 下一步

- [IoT 部署](/guide/iot) - 嵌入式设备部署
- [FFI 接入](/guide/ffi) - 其他语言绑定
- [在线 Playground](/playground) - 在线体验
