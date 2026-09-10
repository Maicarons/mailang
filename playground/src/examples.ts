export const examples = [
  {
    name: 'Hello World',
    description: '第一个 MaìLang 程序',
    code: `// Hello World
println("你好，MaìLang！🌍")
`,
  },
  {
    name: 'Variables',
    description: '变量声明和基本类型',
    code: `// 变量声明
let x = 42
let name = "MaìLang"
let pi = 3.14159
let is_fun = true

println("x = {x}")
println("name = {name}")
println("pi = {pi}")
println("is_fun = {is_fun}")
`,
  },
  {
    name: 'Arithmetic',
    description: '算术运算和数学函数',
    code: `// 基本算术
println("加法: {1 + 2}")
println("乘法: {3 * 4}")
println("幂运算: {2 ** 10}")
println("取余: {17 % 5}")

// 数学函数
println("平方根: {sqrt(144)}")
println("绝对值: {abs(-42)}")
println("最大值: {max(10, 20)}")
`,
  },
  {
    name: 'Functions',
    description: '函数定义和调用',
    code: `// 函数定义
fn greet(name) {
    return "你好，{name}！"
}

fn add(a, b) {
    return a + b
}

fn factorial(n) {
    if n <= 1 {
        return 1
    }
    return n * factorial(n - 1)
}

println(greet("MaìLang"))
println("3 + 4 = {add(3, 4)}")
println("10! = {factorial(10)}")
`,
  },
  {
    name: 'Fibonacci',
    description: '递归斐波那契数列',
    code: `// 斐波那契数列
fn fibonacci(n) {
    if n <= 1 {
        return n
    }
    return fibonacci(n - 1) + fibonacci(n - 2)
}

println("斐波那契数列:")
for i in 0..10 {
    println("  fib({i}) = {fibonacci(i)}")
}
`,
  },
  {
    name: 'Loops',
    description: 'for 和 while 循环',
    code: `// for 循环
println("for 循环:")
for i in 0..5 {
    println("  i = {i}")
}

// while 循环
println("while 循环:")
var count = 0
while count < 5 {
    println("  count = {count}")
    count = count + 1
}
`,
  },
  {
    name: 'Arrays',
    description: '数组操作',
    code: `// 数组
let nums = [1, 2, 3, 4, 5]
println("数组: {nums}")
println("长度: {len(nums)}")
println("第一个: {nums[0]}")
println("最后一个: {nums[4]}")
`,
  },
  {
    name: 'Lambda',
    description: 'Lambda 表达式和闭包',
    code: `// Lambda 表达式
let square = fn(x) -> x * x
let double = fn(x) -> x * 2

let nums = [1, 2, 3, 4, 5]

println("平方:")
for n in nums {
    println("  {n}² = {square(n)}")
}

println("加倍:")
for n in nums {
    println("  {n} × 2 = {double(n)}")
}
`,
  },
  {
    name: 'String Ops',
    description: '字符串操作',
    code: `// 字符串
let name = "MaìLang"
let greeting = "你好，{name}！"

println(greeting)
println("长度: {len(name)}")

// 字符串拼接
let a = "Hello"
let b = "World"
println("{a}, {b}!")
`,
  },
  {
    name: 'Comparison',
    description: '比较和逻辑运算',
    code: `// 比较运算
println("1 == 1: {1 == 1}")
println("1 != 2: {1 != 2}")
println("1 < 2: {1 < 2}")
println("2 > 1: {2 > 1}")

// 逻辑运算
println("true && false: {true && false}")
println("true || false: {true || false}")
println("!true: {!true}")

// 条件表达式
let x = 42
if x > 40 {
    println("x 大于 40")
} else {
    println("x 不大于 40")
}
`,
  },
  {
    name: 'IoT Sensor',
    description: 'IoT 传感器数据处理',
    code: `// IoT 传感器数据处理示例
fn read_sensor() {
    // 模拟传感器读数
    return 23.5
}

fn celsius_to_fahrenheit(c) {
    return c * 9.0 / 5.0 + 32.0
}

fn classify_temperature(c) {
    if c < 0 {
        return "极寒"
    } elif c < 10 {
        return "寒冷"
    } elif c < 25 {
        return "适宜"
    } elif c < 35 {
        return "炎热"
    } else {
        return "极热"
    }
}

// 读取并处理温度
let temp = read_sensor()
let fahrenheit = celsius_to_fahrenheit(temp)
let category = classify_temperature(temp)

println("=== 传感器数据 ===")
println("温度: {temp}°C / {fahrenheit}°F")
println("分类: {category}")
`,
  },
  {
    name: 'Config Parser',
    description: 'IoT 设备配置解析',
    code: `// IoT 设备配置
fn parse_config() {
    let config = "device_sensor_01"
    let interval = 5
    let threshold = 30

    println("=== 设备配置 ===")
    println("设备ID: {config}")
    println("采样间隔: {interval}秒")
    println("报警阈值: {threshold}°C")

    return interval
}

let interval = parse_config()
println("配置完成，间隔={interval}秒")
`,
  },
]
