# 标准库

> **项目链接**：[GitHub](https://github.com/Maicarons/mailang) · [stdlib 源码](https://github.com/Maicarons/mailang/tree/master/crates/mailang-stdlib)

## 概述

MaìLang 标准库以内置全局函数 + 内置方法的形式提供，无需导入即可使用。下文只列出当前运行时真实支持的 API。

## 输入输出

```
// 打印到标准输出（带换行）
println("Hello, MaìLang!")
println("数字: {42}")
println("多项: {1}, {2}, {3}")

// 打印到标准输出（不带换行）
print("请输入: ")

// 从标准输入读取一行
let name = input("请输入姓名: ")
let password = input()  // 无提示
```

## 文件读写（全局函数）

```
// 读取文件全部内容 → str
let content = read_file("config.json")
println(content)

// 写入文件（覆盖）→ null
write_file("output.txt", "Hello, World!")
```

**说明**：没有 `io.read_file` / `io.write_file` / `io.append_file` / `io.exists` / `io.remove_file`。文件 API 仅有全局的 `read_file` 与 `write_file`。

## 数学函数（全局）

```
abs(-5)            // 5
sqrt(16.0)         // 4.0
sin(0.0)           // 0.0
cos(0.0)           // 1.0
floor(3.7)         // 3
ceil(3.2)          // 4
round(3.5)         // 4
min(1, 2, 3)       // 1
max(1, 2, 3)       // 3
```

**说明**：当前没有 `math.` 模块命名空间，也没有 `random` / `pow` / `log` / 三角反函数等更完整的数学库。

## 字符串方法

```
let s = "Hello, MaìLang!"

s.len              // 15（字符数，属性）
s.trim()           // 去除首尾空白
s.to_upper()       // 转大写（别名 to_uppercase）
s.to_lower()       // 转小写（别名 to_lowercase）
s.starts_with("Hello") // true
s.ends_with("!")       // true
s.contains("Maì")      // true
s.split(",")           // 按分隔符分割 → 数组
s.replace("World", "MaìLang")  // 替换（全部匹配）
s.repeat(3)            // 重复 n 次
```

## 数组方法

```
var arr = [3, 1, 4]

arr.len              // 3（属性）
arr.push(5)          // 追加
arr.pop()            // 弹出并返回末尾元素
arr.insert(0, 0)     // 在指定下标插入
arr.contains(4)      // 是否包含
arr.join(",")        // 连接为字符串
arr.reverse()        // 反转
arr.clear()          // 清空
```

## 字典方法

```
var map = {"name": "MaìLang"}

map["name"]          // 读取
map["version"] = "0.2.6"  // 写入
map.has("name")      // 是否包含键
map.keys()           // 所有键 → 数组
map.values()         // 所有值 → 数组
map.remove("name")   // 删除键
map.clear()          // 清空
```

## 类型转换（全局）

```
to_string(42)        // "42"
parse_int("42")      // 42
parse_float("3.14")  // 3.14
len(arr_or_str)      // 长度（也可用 .len 属性）
```

## 时间（全局）

```
time_now()           // 当前时间戳（毫秒）
time_now_secs()      // 当前时间戳（秒）
time_year()
time_month()
time_day()
time_hour()
time_minute()
time_second()
time_date()          // "YYYY-MM-DD"
time_datetime()      // "YYYY-MM-DD HH:MM:SS"
time_elapsed()       // 从给定起点经过的毫秒数
time_sleep(ms)       // 休眠
```

## IoT HAL（全局，默认模拟实现）

```
gpio_write(pin, level)
gpio_read(pin)
adc_read(channel)
delay_ms(ms)
```

默认由宿主模拟实现；嵌入式部署可通过 FFI 注册真实 HAL。

## 错误处理

```
// Result 类型
fn divide(a: float, b: float) -> Result<float, str> {
    if b == 0.0 {
        return Err("Division by zero")
    }
    return Ok(a / b)
}

// Option 类型
fn find(arr: [int], target: int) -> Option<int> {
    for item in arr {
        if item == target {
            return Some(item)
        }
    }
    return None
}
```

## 下一步

- [FFI 接入](/guide/ffi) - 各语言接入指南
- [IoT 部署](/guide/iot) - 嵌入式编译与部署
- [API 参考](/reference/) - 完整 API 文档
