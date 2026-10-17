# tui - 终端 UI（TUI）工具库

> 面向**终端**（ANSI 转义序列 + TTY + 按键），不是图形窗口 GUI。
> 图形/窗口版见 `libs/gui`。

版本: 0.1.0

**Status: host-fn contract + pure-Mai simulation.** 默认不接触真实终端：所有 ANSI 序列以文本形式产出并记录在内存缓冲里，终端尺寸固定为 80x24，按键从注入队列读取。注册 host 函数后才会真正操作终端。

## 功能说明

ANSI 转义序列构造（颜色 / 样式 / 光标 / 屏幕）、文本布局（对齐 / 截断 / 填充）、常用组件（进度条 / 表格 / 边框 /键值行），以及终端尺寸与按键的 host-fn 契约。

## Host-function contract

真实终端能力由宿主注册，注册后**按名覆盖** `lib.mai` 中的桩实现：

| Host function | Signature | Returns |
|---|---|---|
| `ttui_term_size` | `() -> [cols, rows]` | `Ok(any)` \| `Err(str)` |
| `ttui_is_tty` | `() -> bool` | `bool` |
| `ttui_read_key` | `() -> str` | `Ok(str)` \| `Err(str)` |
| `ttui_write` | `(text: str) -> any` | `Ok(any)` \| `Err(str)` |

C FFI:

```c
mailang_register_host_fn(interp, "ttui_write", host_ttui_write, NULL);
```

Rust:

```rust
interp.register_host_fn("ttui_write", |args| { /* write to stdout */ });
```

未注册时使用 `lib.mai` 里的同名桩函数：写入内容进内存缓冲、尺寸返回 80x24、`ttui_read_key` 从队列取键。

## API 列表

### ANSI 序列

#### csi(code)

构造 `ESC [ <code>`。字符串 `code` 为纯数字或含`;` / `?` 的参数。

- 参数: `code` (str)
- 返回值: (str)控制序列前缀

#### sgr(params)

由参数数组构造 SGR 序列，如 `[1, 31]` → `ESC[1;31m`。

- 参数: `params` (array<int>)
- 返回值: (str)

#### color(text, name)

前景色包裹。颜色名：`black` `red` `green` `yellow` `blue` `magenta` `cyan` `white` `gray` `bright_red` `bright_green` `bright_yellow` `bright_blue` `bright_magenta` `bright_cyan` `bright_white`。未知名字原样返回。

- 参数: `text` (str), `name` (str)
- 返回值: (str)

#### bg(text, name)

背景色包裹，颜色名同上。未知名字原样返回。

- 参数: `text` (str), `name` (str)
- 返回值: (str)

#### style(text, name) / bold(text) / dim(text) / underline(text)

样式包裹。样式名：`bold` `dim` `italic` `underline` `blink` `reverse` `hidden` `strike`。未知名字原样返回。

- 参数: `text` (str), `name` (str)
- 返回值: (str)

#### attr(text, names)

多个属性合并为一个 SGR 序列，如 `attr("x", ["bold", "red"])`。

- 参数: `text` (str), `names` (array<str>)
- 返回值: (str)

#### reset_style()

返回 `ESC[0m`。

- 返回值: (str)

#### 屏幕与光标

`clear_screen()` `ESC[2J`；`clear_line()` `ESC[2K`；`home()` `ESC[H`；`cursor_up(n)` / `cursor_down(n)` / `cursor_forward(n)` / `cursor_back(n)`；`cursor_to(row, col)` → `ESC[row;colH`；`hide_cursor()` / `show_cursor()`；`enter_alt_screen()` / `exit_alt_screen()`（`ESC[?1049h/l`）；`line_up()` → `ESC[1A ESC[2K`（原地重绘用）。

- 参数: 各函数见名
- 返回值: (str)

### 输出

#### write(text)

经`ttui_write` 钩子写入，不追加换行。

- 参数: `text` (str)
- 返回值: `Ok("sim")` \| `Err(str)`

#### line(text)

写入并追加 `\n`。

- 参数: `text` (str)
- 返回值: `Ok("sim")` \| `Err(str)`

#### puts(text)

用内建 `print` 输出（不经钩子），无 host 时也能打到真实终端。

- 参数: `text` (str)
- 返回值: null

### 文本布局

#### width(s)

字符数（见Honest limits）。

- 参数: `s` (str)
- 返回值: (int)

#### pad_left(s, w) / pad_right(s, w) / pad(s, w, align)

对齐填充。`align` 取 `"left"` / `"right"` / `"center"`；已超宽时原样返回。

- 参数: `s` (str), `w` (int), `align` (str)
- 返回值: (str)

#### truncate(s, w)

截断到最多 `w` 个字符（无省略号）。

- 参数: `s` (str), `w` (int)
- 返回值: (str)

#### repeat(ch, n)

重复 `ch` 共 `n` 次。

- 参数: `ch` (str), `n` (int)
- 返回值: (str)

### 组件

#### percent(current, total)

整数百分比，钳制到 0..100；`total <= 0` 返回 0。

- 参数: `current` (int), `total` (int)
- 返回值: (int)

#### progress_bar(current, total, bar_width) / progress_labeled(...) / progress_task(label, ...)

进度条。`progress_labeled` 末尾加百分比；`progress_task` 前缀标签并补齐到 `total_width`。

- 参数: `current` (int), `total` (int), `bar_width` (int)
- 返回值: (str)

#### kv(label, value, label_width)

对齐的`标签   值` 行。

- 参数: `label` (str), `value` (any), `label_width` (int)
- 返回值: (str)

#### table(rows, widths)

按 `widths` 对齐的二维表。缺格补空。

- 参数: `rows` (array<array>), `widths` (array<int>)
- 返回值: (array<str>)

#### box(lines, style_name)

给多行文本加边框。`style_name` 取 `"single"` / `"double"` / `"round"`。

- 参数: `lines` (array<str>), `style_name` (str)
- 返回值: (array<str>)

#### join_lines(lines)

用 `\n` 连接各行。

- 参数: `lines` (array<str>)
- 返回值: (str)

### 终端信息

#### term_size() / term_cols() / term_rows() / is_tty()

经钩子查询；尺寸出错时回退 80x24。

- 返回值: `[cols, rows]` / (int) / (int) / (bool)

### 按键

#### read_key()

取下一个按键；队列为空返回 `Err`（模拟不阻塞）。

- 返回值: `Ok(str)` \| `Err(str)`

#### queue_keys(keys) / has_key()

注入按键队列并查询是否还有键。

- 参数: `keys` (array<str>)
- 返回值: null / (bool)

#### key_name(key)

把原始按键转成友好名：`up` `down` `left` `right` `enter` `space` `esc` `backspace`，其余原样返回。

- 参数: `key` (str)
- 返回值: (str)

### 模拟状态

#### reset() / clear() / out() / out_text() / written() / mode() / set_mode(m) / set_size(cols, rows) / set_tty(flag)

重置、清空缓冲、查看写入内容与计数、查询/设置模式、尺寸与 tty 标志。

- 返回值: 各函数见名

## Pure-Mai simulation (default)

| Function | Returns | Default sim |
|---|---|---|
| `ttui_write` | `Ok("sim")` | 追加到内存缓冲，`written` 累加字符数 |
| `ttui_term_size` | `Ok([80, 24])` | 固定 80x24，可 `set_size`改 |
| `ttui_is_tty` | `true` | 可 `set_tty` 改 |
| `ttui_read_key` | `Ok(key)` / `Err(...)` | 从 `queue_keys` 注入的队列取，耗尽即 `Err` |

## Layout

```
ui/
├── mailib.ini          # [module] name=ui / version=0.1.0 / entry=lib.mai
├── lib.mai             # 全部实现（纯 Mai，无 extern）
├── README.md
├── example/basic.mai   # 基础用法演示
└── test/test_tui.mai    # 78 个断言
```

## Usage

```mai
import "tui"

tui.puts(tui.bold(tui.color("状态", "green")))
println(tui.progress_labeled(3, 4, 20))
println(tui.kv("温度", "23.5C", 8))
```

## Tests

```
cargo run -p mailang-cli --release -- run libs/tui/test/test_tui.mai
```

全部通过时退出码 `0`，任一失败为 `1`。

## 实机测试（Windows）

CLI 内置 Win32 后端，`--host=tui` 会把上面的桩函数替换成真实调用：

| Hook | 实机实现 |
|------|----------|
| `tui_write` | 直接写进程 stdout（`std::io::stdout`，UTF-8） |
| `tui_term_size` | `GetConsoleScreenBufferInfo` 的窗口尺寸，失败回退 80x24 |
| `tui_is_tty` | 恒为 `true`（Windows 控制台） |
| `tui_read_key` | `ReadConsoleInputW` + `GetNumberOfConsoleInputEvents`；**非阻塞**，无按键立即返回 |

```
cargo run -p mailang-cli --release -- run --host=tui libs/tui/test/host_tui.mai
```

该脚本断言宿主确实接管（`write()` 返回 `"tty"` 而非桩的 `"sim"`）、帧缓冲仍可读、终端尺寸为正、无按键时 `read_key()` 返回 `Err` 值。**Windows 实测 7/7 通过。**

注意：宿主接管后 `read_key()` 读的是真实控制台，而 `has_key()` 仍读包内队列。写交互循环时请以 `read_key()` 的返回值为准，不要用 `has_key()` 当循环条件，否则会死循环（`example/basic.mai` 里有正确写法）。

## Honest limits

- 未经host 注册前不接触真实终端；ANSI 序列只是文本，重定向 stdout 时会看到原始转义字节。
- `width()` 按字符数计，不按East Asian Display Width：一个汉字算1 而非2，所以中英混排时 `pad` / `box` / `table` 的列会漂移。需要对齐时请自行按显示宽度补空格，或只用 ASCII 内容。
- 模拟永不阻塞：`read_key()` 在队列空时返回 `Err`，而不是等待输入。
- 没有光标位置查询、没有滚动区（scroll region）、没有鼠标上报，也没有真宽字符（wcwidth）表。
- `Ok("sim")` 只是"已写入模拟缓冲"，**不是**真实终端已渲染的确认。
- **寄存器后端（`--vm=register`）尚未跑通本库全部功能。** 栈式后端 78/78 通过；寄存器后端在`pad(text, width, align)` 这类"字符串 + 整数 + 字符串"混合参数调用上报 `Type error: Cannot apply binary op`，而`kv(label, value, label_width)`（字符串 + 字符串 + 整数）正常。这是项目既有的寄存器后端缺陷（参数寄存器分配与参数类型顺序相关，见 `ROADMAP.md` Phase M），不是本库的逻辑问题。库内所有涉及算术的参数都已先复制到局部变量以规避该缺陷，剩余问题需等寄存器后端修复。 
