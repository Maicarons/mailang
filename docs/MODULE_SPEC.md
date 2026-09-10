# MaìLang 模块规范

## 目录结构

每个模块必须遵循以下目录结构：

```
libs/<module_name>/
├── README          # 模块说明文档（无扩展名）
├── mailib.ini      # 模块配置文件
├── lib.mai         # 模块入口文件
├── example/        # 示例代码目录
│   └── basic.mai   # 基础使用示例
└── test/           # 测试代码目录
    └── test_<module>.mai  # 模块测试
```

## mailib.ini 格式

```ini
[module]
name = <模块名>
version = <版本号，语义化版本>
description = <简短描述>
author = <作者>
entry = lib.mai
```

示例：
```ini
[module]
name = time
version = 0.2.0
description = Time and date functions
author = MaìLang Team
entry = lib.mai
```

## README 格式

README 文件无扩展名，内容格式如下：

```
# <模块名> - <简短描述>

版本: <版本号>

## 功能说明

<模块功能的详细说明>

## API 列表

### <函数名>(<参数>)

<函数描述>

- 参数: <参数说明>
- 返回值: <返回值说明>

## 使用示例

​```mai
import "<模块名>"
// 示例代码
​```
```

## 模块导入方式

### 方式一：命名导入（推荐，仅 PC）

```mai
import "time"
let ts = time.now()
```

模块文件需放在 `mailang-cli.exe` 同目录的 `libs/` 下。

### 方式二：路径导入（全平台）

```mai
import "./libs/time"
let ts = time.now()
```

支持相对路径和绝对路径。

## 函数命名规范

- 使用 snake_case 命名函数
- 函数名应简洁明了
- 避免使用缩写（除非是通用缩写如 `str`, `num`）

## 模块间依赖

- 模块可以相互引用，但不能交叉引用（A 引用 B，B 引用 A）
- 依赖关系应为有向无环图（DAG）

## 内置函数

模块可以调用以下内置函数：

- `time_now()` - 获取当前时间戳（毫秒）
- `time_now_secs()` - 获取当前时间戳（秒）
- `time_date()` - 获取当前日期字符串
- `time_datetime()` - 获取当前日期时间字符串
- `time_year()` - 获取当前年份
- `time_month()` - 获取当前月份
- `time_day()` - 获取当前日期
- `time_hour()` - 获取当前小时
- `time_minute()` - 获取当前分钟
- `time_second()` - 获取当前秒数
- `time_sleep(ms)` - 休眠指定毫秒
- `time_elapsed(start)` - 计算经过的时间
