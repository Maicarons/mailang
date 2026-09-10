# 标准库

## 概述

MaìLang 标准库提供了常用的功能模块，无需额外安装即可使用。

## io 模块

### 输入输出

```
// 打印到标准输出（带换行）
println("Hello, MaìLang!")
println("数字: {42}")
println("多项: {1}, {2}, {3}")

// 打印到标准输出（不带换行）
print("请输入: ")

// 从标准输入读取
let name = input("请输入姓名: ")
let password = input()  // 无提示
```

### 文件操作

```
// 读取文件
let content = io.read_file("config.json")
println(content)

// 写入文件
io.write_file("output.txt", "Hello, World!")

// 追加写入
io.append_file("log.txt", "新的日志\n")

// 检查文件是否存在
if io.exists("config.json") {
    println("配置文件存在")
}

// 删除文件
io.remove_file("temp.txt")
```

## math 模块

### 基本数学函数

```
// 绝对值
math.abs(-5)       // 5
math.abs(-3.14)    // 3.14

// 平方根
math.sqrt(16.0)    // 4.0
math.sqrt(2.0)     // 1.4142135...

// 三角函数
math.sin(0.0)      // 0.0
math.cos(0.0)      // 1.0
math.tan(0.0)      // 0.0

// 反三角函数
math.asin(0.0)     // 0.0
math.acos(1.0)     // 0.0
math.atan(0.0)     // 0.0

// 指数和对数
math.exp(1.0)      // 2.71828... (e)
math.log(2.71828)  // 1.0
math.log2(8.0)     // 3.0
math.log10(100.0)  // 2.0
math.pow(2.0, 3.0) // 8.0

// 取整
math.floor(3.7)    // 3
math.ceil(3.2)     // 4
math.round(3.5)    // 4
math.trunc(3.7)    // 3

// 最大最小值
math.max(1, 2, 3)  // 3
math.min(1, 2, 3)  // 1

// 常量
math.PI            // 3.141592653589793
math.E             // 2.718281828459045
math.INFINITY      // 无穷大
math.NAN           // 非数字
```

### 随机数

```
// 0.0 到 1.0 之间的随机浮点数
let r = math.random()

// 指定范围的随机整数
let dice = math.random_int(1, 6)

// 随机选择
let colors = ["红", "绿", "蓝"]
let color = math.random_choice(colors)
```

## string 模块

### 字符串操作

```
let s = "Hello, MaìLang!"

// 长度
s.len()           // 15

// 大小写转换
s.upper()         // "HELLO, MAÌLANG!"
s.lower()         // "hello, maìlang!"

// 查找
s.contains("Maì")    // true
s.starts_with("Hello") // true
s.ends_with("!")     // true
s.find("Maì")        // 7 (索引)
s.rfind("l")         // 13 (从后向前)

// 截取
s.slice(0, 5)     // "Hello"
s.slice(7)        // "MaìLang!"

// 替换
s.replace("World", "MaìLang")  // "Hello, MaìLang!"
s.replace_all("l", "L")        // "HeLLo, MaìLang!"

// 分割与连接
"a,b,c".split(",")     // ["a", "b", "c"]
["a", "b", "c"].join("-") // "a-b-c"

// 去除空白
"  hello  ".trim()       // "hello"
"  hello  ".trim_start() // "hello  "
"  hello  ".trim_end()   // "  hello"

// 重复
"ha".repeat(3)           // "hahaha"

// 填充
"42".pad_start(5, "0")   // "00042"
"42".pad_end(5, ".")     // "42..."

// 类型转换
"42".to_int()            // 42
"3.14".to_float()        // 3.14
42.to_string()           // "42"
3.14.to_string()         // "3.14"
```

## collections 模块

### 数组操作

```
var arr = [3, 1, 4, 1, 5, 9, 2, 6]

// 添加元素
arr.push(5)           // [3, 1, 4, 1, 5, 9, 2, 6, 5]
arr.insert(0, 0)      // [0, 3, 1, 4, 1, 5, 9, 2, 6, 5]

// 删除元素
arr.pop()              // 返回 5, 数组变为 [0, 3, 1, 4, 1, 5, 9, 2, 6]
arr.remove(0)          // 返回 0, 数组变为 [3, 1, 4, 1, 5, 9, 2, 6]

// 查找
arr.contains(4)        // true
arr.index_of(4)        // 2
arr.last_index_of(1)   // 3

// 排序
arr.sort()             // [1, 1, 2, 3, 4, 5, 6, 9]
arr.reverse()          // [9, 6, 5, 4, 3, 2, 1, 1]

// 切片
arr.slice(0, 3)        // [9, 6, 5]
arr.slice(2)           // [5, 4, 3, 2, 1, 1]

// 其他
arr.len()              // 8
arr.is_empty()         // false
arr.first()            // 9
arr.last()             // 1
arr.sum()              // 31
arr.min()              // 1
arr.max()              // 9
arr.average()          // 3.875

// 高阶函数
arr.map(fn(x) -> x * 2)           // [18, 12, 10, 8, 6, 4, 2, 2]
arr.filter(fn(x) -> x > 3)        // [9, 6, 5, 4]
arr.reduce(0, fn(acc, x) -> acc + x) // 31
arr.find(fn(x) -> x > 5)          // 9
arr.every(fn(x) -> x > 0)         // true
arr.some(fn(x) -> x > 8)          // true
```

### 字典操作

```
var map = {"name": "MaìLang", "version": "0.1.0"}

// 添加/修改
map["author"] = "MaìLang Team"

// 访问
map["name"]           // "MaìLang"
map.get("name")       // "MaìLang"
map.get_or("year", 2024) // 2024 (如果不存在)

// 检查
map.has_key("name")   // true
map.has_value("MaìLang") // true

// 删除
map.remove("version")

// 获取键值对
map.keys()            // ["name", "author"]
map.values()          // ["MaìLang", "MaìLang Team"]
map.entries()         // [["name", "MaìLang"], ["author", "MaìLang Team"]]

// 大小
map.len()             // 2
map.is_empty()        // false
```

## 转换函数

```
// 字符串转数字
parse_int("42")       // 42
parse_float("3.14")   // 3.14

// 数字转字符串
to_string(42)         // "42"
to_string(3.14)       // "3.14"

// 类型检查
is_int(42)            // true
is_float(3.14)        // true
is_str("hello")       // true
is_bool(true)         // true
is_null(null)         // true
is_array([1, 2])      // true
is_map({"a": 1})      // true
```

## 系统模块

```
// 获取当前时间戳（秒）
let timestamp = sys.time()

// 获取环境变量
let home = sys.env("HOME")
let path = sys.env("PATH")

// 命令行参数
let args = sys.args()

// 退出程序
sys.exit(0)
sys.exit(1)  // 非零表示错误

// 平台信息
sys.os()      // "windows", "linux", "macos"
sys.arch()    // "x86_64", "aarch64"
```

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
