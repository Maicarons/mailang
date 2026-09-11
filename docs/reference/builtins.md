# 内置函数

> **项目链接**：[GitHub](https://github.com/Maicarons/mailang) · [stdlib 源码](https://github.com/Maicarons/mailang/tree/master/crates/mailang-stdlib)

## 概述

MaìLang 提供丰富的内置函数，无需导入即可使用。

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

## 数学函数

### 数值运算

```
abs(x)           // 绝对值
sqrt(x)          // 平方根
cbrt(x)          // 立方根
pow(base, exp)   // 幂运算
exp(x)           // e^x
log(x)           // 自然对数
log2(x)          // 以2为底的对数
log10(x)         // 以10为底的对数
```

### 三角函数

```
sin(x)           // 正弦
cos(x)           // 余弦
tan(x)           // 正切
asin(x)          // 反正弦
acos(x)          // 反余弦
atan(x)          // 反正切
atan2(y, x)      // 反正切（双参数）
```

### 取整函数

```
floor(x)         // 向下取整
ceil(x)          // 向上取整
round(x)         // 四舍五入
trunc(x)         // 截断小数
```

### 比较函数

```
min(a, b, ...)   // 最小值
max(a, b, ...)   // 最大值
clamp(x, min, max) // 限制在范围内
```

### 随机数

```
random()         // 0.0 ~ 1.0 的随机浮点数
random_int(min, max) // 指定范围的随机整数
random_choice(arr)   // 从数组中随机选择
```

### 常量

```
PI               // 3.141592653589793
E                // 2.718281828459045
INFINITY         // 无穷大
NAN              // 非数字
```

## 字符串函数

### 基本操作

```
len(s)               // 字符串长度
contains(s, sub)     // 是否包含子串
starts_with(s, prefix) // 是否以指定前缀开始
ends_with(s, suffix)   // 是否以指定后缀结束
find(s, sub)         // 查找子串位置
rfind(s, sub)        // 从后向前查找
```

### 转换

```
upper(s)             // 转大写
lower(s)             // 转小写
trim(s)              // 去除首尾空白
trim_start(s)        // 去除开头空白
trim_end(s)          // 去除结尾空白
```

### 截取与替换

```
slice(s, start, end) // 截取子串
replace(s, old, new) // 替换第一个匹配
replace_all(s, old, new) // 替换所有匹配
split(s, delimiter)  // 分割字符串
join(arr, delimiter) // 连接数组为字符串
```

### 填充

```
pad_start(s, len, char) // 左填充
pad_end(s, len, char)   // 右填充
repeat(s, n)           // 重复字符串
```

## 数组函数

### 基本操作

```
len(arr)             // 数组长度
push(arr, x)         // 添加到末尾
pop(arr)             // 删除并返回末尾
insert(arr, i, x)    // 在指定位置插入
remove(arr, i)       // 删除指定位置
```

### 查找

```
contains(arr, x)     // 是否包含
index_of(arr, x)     // 查找位置
last_index_of(arr, x) // 从后向前查找
find(arr, fn)        // 查找满足条件的元素
find_index(arr, fn)  // 查找满足条件的索引
```

### 排序

```
sort(arr)            // 排序
sort_by(arr, fn)     // 按函数排序
reverse(arr)         // 反转
shuffle(arr)         // 随机打乱
```

### 高阶函数

```
map(arr, fn)         // 映射
filter(arr, fn)      // 过滤
reduce(arr, init, fn) // 归约
for_each(arr, fn)    // 遍历
every(arr, fn)       // 是否所有元素满足条件
some(arr, fn)        // 是否有元素满足条件
```

### 切片

```
slice(arr, start, end) // 截取子数组
concat(a, b)         // 连接两个数组
flat(arr)            // 展平嵌套数组
```

### 统计

```
sum(arr)             // 求和
min(arr)             // 最小值
max(arr)             // 最大值
average(arr)         // 平均值
```

## 字典函数

```
len(map)             // 键值对数量
has_key(map, key)    // 是否包含键
has_value(map, val)  // 是否包含值
get(map, key)        // 获取值
get_or(map, key, default) // 获取值或默认值
set(map, key, val)   // 设置键值对
remove(map, key)     // 删除键值对
keys(map)            // 所有键
values(map)          // 所有值
entries(map)         // 所有键值对
merge(a, b)          // 合并两个字典
```

## 类型转换

```
to_string(x)         // 转为字符串
to_int(x)            // 转为整数
to_float(x)          // 转为浮点数
parse_int(s)         // 字符串解析为整数
parse_float(s)       // 字符串解析为浮点数
```

## 类型检查

```
is_int(x)            // 是否为整数
is_float(x)          // 是否为浮点数
is_str(x)            // 是否为字符串
is_bool(x)           // 是否为布尔
is_null(x)           // 是否为null
is_array(x)          // 是否为数组
is_map(x)            // 是否为字典
type_of(x)           // 返回类型名称字符串
```

## 系统函数

```
time()               // 当前时间戳（秒）
env(name)            // 获取环境变量
args()               // 命令行参数
exit(code)           // 退出程序
os()                 // 操作系统名称
arch()               // CPU 架构
```

## 下一步

- [错误码](/reference/errors) - 错误代码说明
- [类型系统](/reference/types) - 类型详解
