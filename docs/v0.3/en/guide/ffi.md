# FFI Guide

> **Project Links**: [GitHub](https://github.com/Maicarons/mailang) · [FFI Source](https://github.com/Maicarons/mailang/tree/master/crates/mailang-ffi) · [Bindings](https://github.com/Maicarons/mailang/tree/master/bindings)

## Overview

MaìLang supports 12 programming languages through its C FFI layer.

## C/C++

```c
#include "mailang.h"

int main() {
    MailangInterpreter* interp = mailang_create();
    MailangResult result = mailang_eval(interp, "1 + 2");
    if (result.code == 0) {
        printf("Result: %s\n", result.output);
        mailang_free_string(result.output);
    }
    mailang_destroy(interp);
    return 0;
}
```

## Python

```python
from mailang import MailangInterpreter

interp = MailangInterpreter()
result = interp.eval("1 + 2")
print(f"Result: {result}")
```

## JavaScript/Node.js

```javascript
const { MailangInterpreter } = require('mailang');
const interp = new MailangInterpreter();
console.log(interp.eval('1 + 2'));
```

## Rust

```rust
use mailang_core::MailangInterpreter;

fn main() {
    let mut interp = MailangInterpreter::new();
    println!("{}", interp.eval("1 + 2").unwrap());
}
```

## Supported Languages

| Language | FFI Mechanism | Tool |
|----------|--------------|------|
| C/C++ | Native linking | cbindgen |
| Python | CPython C API | PyO3 |
| JavaScript | Node-API | napi-rs |
| Java | JNI | jni crate |
| Go | cgo | C FFI |
| Lua | Lua C API | mlua |
| Ruby | FFI gem | UniFFI |
| Swift | C bridging | UniFFI |
| PHP | ext-ffi | C FFI |
| C# | P/Invoke | C FFI |
| Kotlin | JNI | UniFFI |
| WASM | wasm-bindgen | wasm-bindgen |

## Next Steps

- [IoT Deployment](/v0.3/en/guide/iot) - Embedded compilation
