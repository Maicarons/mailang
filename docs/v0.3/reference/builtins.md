# 内置函数

> **项目链接**：[GitHub](https://github.com/Maicarons/mailang) · [stdlib 源码](https://github.com/Maicarons/mailang/tree/master/crates/mailang-stdlib)

## 概述

MaìLang 提供全局内置函数，以及数组 / 字典 / 字符串上的内置方法。下面只列出当前运行时真实支持的 API。

## 输入输出

### println(...)

打印一行到标准输出。

```
println("Hello, World!")
println("数字: {42}")
println("多项: {1}, {2}, {3}")
```

### print(...)

打印到标准输出（不换行）。

```
print("请输入: ")
```

### input(prompt?)

从标准输入读取一行。

```
let name = input("请输入姓名: ")
let line = input()
```

## 文件读写

### read_file(path)

读取文件全部内容，返回字符串。

### write_file(path, contents)

写入文件（覆盖），返回 null。

```
let content = read_file("config.json")
write_file("output.txt", "Hello, World!")
```

## 数学函数（全局）

```
abs(x)           // 绝对值
sqrt(x)          // 平方根
sin(x)           // 正弦
cos(x)           // 余弦
floor(x)         // 向下取整
ceil(x)          // 向上取整
round(x)         // 四舍五入
min(a, b, ...)   // 最小值
max(a, b, ...)   // 最大值
```

## 字符串

长度用属性 `s.len`（或全局 `len(s)`）。

### 方法

```
s.trim()                 // 去除首尾空白
s.to_upper()             // 转大写（别名 to_uppercase）
s.to_lower()             // 转小写（别名 to_lowercase）
s.starts_with(prefix)    // 是否以指定前缀开始
s.ends_with(suffix)      // 是否以指定后缀结束
s.contains(sub)          // 是否包含子串
s.split(delimiter)       // 分割字符串 → 数组
s.replace(old, new)      // 替换匹配（全部）
s.repeat(n)              // 重复字符串
```

## 数组

长度用属性 `arr.len`（或全局 `len(arr)`）。

### 方法

```
arr.push(x)          // 添加到末尾
arr.pop()            // 删除并返回末尾元素
arr.insert(i, x)     // 在指定位置插入
arr.contains(x)      // 是否包含
arr.join(sep)        // 连接为字符串
arr.reverse()        // 反转
arr.clear()          // 清空
```

## 字典

### 方法

```
map.has(key)         // 是否包含键
map.keys()           // 所有键 → 数组
map.values()         // 所有值 → 数组
map.remove(key)      // 删除键值对
map.clear()          // 清空
```

索引访问：`map[key]` 读、`map[key] = val` 写。

## 类型转换（全局）

```
to_string(x)         // 转为字符串
parse_int(s)         // 字符串解析为整数
parse_float(s)       // 字符串解析为浮点数
len(x)               // 字符串 / 数组长度
```

## 时间（全局）

```
time_now()               // 当前时间戳（毫秒）
time_now_secs()          // 当前时间戳（秒）
time_year()              // 年
time_month()             // 月
time_day()               // 日
time_hour()              // 时
time_minute()            // 分
time_second()            // 秒
time_date()              // "YYYY-MM-DD"
time_datetime()          // "YYYY-MM-DD HH:MM:SS"
time_elapsed(start)      // 从 start 起经过的毫秒
time_sleep(ms)           // 休眠
```

## IoT HAL（全局，默认模拟实现）

```
gpio_write(pin, level)
gpio_read(pin)
adc_read(channel)
delay_ms(ms)
```

## 下一步

- [错误码](/v0.3/reference/errors) - 错误代码说明
- [类型系统](/v0.3/reference/types) - 类型详解
