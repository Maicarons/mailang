# FFI 接入指南

> **项目链接**：[GitHub](https://github.com/Maicarons/mailang) · [FFI 源码](https://github.com/Maicarons/mailang/tree/master/crates/mailang-ffi) · [绑定示例](https://github.com/Maicarons/mailang/tree/master/bindings)

## 概述

MaìLang 通过 C FFI 层支持 12 种编程语言调用。所有语言绑定都基于统一的 C ABI，确保跨语言的一致性。

## 架构

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

## C/C++ 接入

### 编译

```bash
# Linux/macOS
gcc -o demo main.c -I. -L/path/to/lib -lmailang_ffi

# Windows (MinGW)
gcc -o demo.exe main.c -I. -L/path/to/lib -lmailang_ffi -lws2_32 -luserenv -lntdll
```

### 头文件

`mailang.h` 由 cbindgen 自动生成，包含以下 API：

```c
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

// 错误处理
const char* mailang_last_error(MailangInterpreter* interp);

// 内存管理
void mailang_free_string(char* ptr);
```

### 示例代码

```c
#include <stdio.h>
#include "mailang.h"

int main(void) {
    MailangInterpreter* interp = mailang_create();
    
    MailangResult result;
    int ret = mailang_eval(interp, "1 + 2", &result);
    if (ret == 0 && result.code == 0) {
        printf("Result: %s\n", result.output);
        mailang_free_string(result.output);
    }
    
    mailang_destroy(interp);
    return 0;
}
```

## Python 接入

### 方式一：PyO3 绑定（推荐）

```bash
pip install mailang
```

```python
from mailang import MailangInterpreter

interp = MailangInterpreter()
result = interp.eval('1 + 2')
print(result)  # 3
```

### 方式二：ctypes 调用

```python
import ctypes

lib = ctypes.CDLL('mailang_ffi.dll')
interp = lib.mailang_create()
# ... 调用 FFI 函数
lib.mailang_destroy(interp)
```

## JavaScript/Node.js 接入

### napi-rs 绑定

```bash
npm install mailang
```

```javascript
const { MailangInterpreter } = require('mailang');

const interp = new MailangInterpreter();
console.log(interp.eval('1 + 2'));  // 3
```

## WebAssembly 接入

### 浏览器

```bash
cargo build --release -p mailang-wasm --target wasm32-unknown-unknown
```

```javascript
import init, { WasmInterpreter } from './mailang_wasm.js';

async function main() {
    await init();
    const interp = new WasmInterpreter();
    console.log(interp.eval('1 + 2'));  // "3"
}
main();
```

## 其他语言

### Java/Kotlin (JNI)

```java
System.loadLibrary("mailang_ffi");
long interp = mailang_create();
// ... 调用 FFI
mailang_destroy(interp);
```

### C# (.NET P/Invoke)

```csharp
[DllImport("mailang_ffi")]
static extern IntPtr mailang_create();

var interp = mailang_create();
// ... 调用 FFI
mailang_destroy(interp);
```

### Go (cgo)

```go
/*
#cgo LDFLAGS: -lmailang_ffi
#include "mailang.h"
*/
import "C"

interp := C.mailang_create()
defer C.mailang_destroy(interp)
```

### Ruby (FFI gem)

```ruby
require 'ffi'

module Mailang
  extend FFI::Library
  ffi_lib 'mailang_ffi'
  attach_function :mailang_create, [], :pointer
  attach_function :mailang_destroy, [:pointer], :void
end
```

### Swift (C bridging)

```swift
import CMaìLang  // or use bridging header

let interp = mailang_create()
defer { mailang_destroy(interp) }
```

### PHP (ext-ffi)

```php
$ffi = FFI::cdef('...', 'mailang_ffi.dll');
$interp = $ffi->mailang_create();
$ffi->mailang_destroy($interp);
```

### Lua (mlua)

```lua
local mailang = require("mailang")
local interp = mailang.create()
interp:eval("1 + 2")
interp:destroy()
```

## 构建 FFI 库

```bash
# 构建 C 动态库
cargo build --release -p mailang-ffi

# 生成 C 头文件
cbindgen --config cbindgen.toml --crate mailang-ffi --output mailang.h

# 构建 WASM
cargo build --release -p mailang-wasm --target wasm32-unknown-unknown
```
