# gui - 图形 UI 工具库

版本: 0.1.0

**Status: host-fn contract + pure-Mai simulation.** 默认不创建真实窗口：绘制调用记录在内存显示列表里，事件由代码注入，因此测试可以逐条断言绘制命令。

图形窗口版（窗口 / 画布 / 控件 / 事件）。终端（ANSI/TTY）版见 `libs/tui`。

## 功能说明

窗口生命周期、离屏画布绘制（矩形/文本/线/圆/像素）、颜色模型、即时模式控件、事件队列与几何命中测试。

## Host-function contract

真实后端（Win32 / X11 / WASM Canvas / 嵌入式屏）由宿主注册，注册后**按名覆盖** `lib.mai` 中的桩实现：

| Host function | Signature | Returns |
|---|---|---|
| `gui_create_window` | `(title, w, h) -> handle` | `Ok(int)` \| `Err(str)` |
| `gui_close_window` | `(handle) -> any` | `Ok(any)` \| `Err(str)` |
| `gui_poll_event` | `() -> event` | `Ok(event)` \| `Err("no event")` |
| `gui_present` | `(handle) -> any` | `Ok(any)` \| `Err(str)` |
| `gui_draw_rect` | `(handle, x, y, w, h, r, g, b, a) -> any` | `Ok(any)` \| `Err(str)` |
| `gui_draw_text` | `(handle, x, y, text, r, g, b, a) -> any` | `Ok(any)` \| `Err(str)` |
| `gui_draw_line` | `(handle, x1, y1, x2, y2, r, g, b, a) -> any` | `Ok(any)` \| `Err(str)` |
| `gui_draw_circle` | `(handle, cx, cy, r, filled, r, g, b, a) -> any` | `Ok(any)` \| `Err(str)` |
| `gui_load_font` | `(path) -> handle` | `Ok(str)` \| `Err(str)` |

C FFI：

```c
mailang_register_host_fn(interp, "gui_create_window", host_gui_create_window, NULL);
```

Rust：

```rust
interp.register_host_fn("gui_draw_rect", |args| { /* 画到真实窗口 */ });
```

未注册时使用同名桩函数：窗口记在内存里，绘制进显示列表，事件从注入队列取。

## API 列表

### 窗口

#### open_window(title, w, h)

创建窗口并直接返回句柄（整数，失败为 0）。想拿 `Result` 请调用 `gui_create_window`。

- 参数: `title` (str), `w` (int), `h` (int)
- 返回值: (int)

#### window_count() / is_open(h) / title(h) / width(h) / height(h)

窗口查询。未知句柄返回 0 / false / 空串。

#### resize(h, w, hgt) / close_window(h) / present(h) / frame_count()

调整尺寸、关闭、把后缓冲提交到屏幕并计数。对未知或已关闭窗口返回 `Err`。

#### mode() / set_mode(m) / reset()

模式查询（默认 `"sim"`）与整体重置。

### 画布绘制

#### begin(handle)

开始新一帧：清空显示列表并把后续绘制归属到该窗口。

- 参数: `handle` (int)
- 返回值: `Ok("sim")` \| `Err(str)`

#### draw_rect(handle, x, y, w, h, color) / draw_pixel(handle, x, y, color)

填充矩形 / 单像素。

#### draw_text(handle, x, y, text, color) / draw_line(...) / draw_circle(handle, cx, cy, r, filled, color)

文本、直线、圆（`filled` 为 true 实心）。

颜色可用名称（`"red"` `"cyan"` `"orange"` …）或 `[r, g, b]` / `[r, g, b, a]` 数组。

### 显示列表

#### cmd_count() / cmd_op(i) / cmd_args(i) / cmd_rgba(i) / cmd_text(i) / cmd_window(i)

读回本帧绘制命令，越界返回空值。`cmd_op` 是 `"rect"` / `"text"` / `"line"` / `"circle"`。

#### count_op(kind) / find_op(kind)

统计某类命令数量 / 返回首个匹配索引（无匹配为 -1）。

### 颜色

#### named_color(name) / to_rgba(color) / lookup_color(name) / is_known_name(v)

`"black" "white" "red" "green" "blue" "yellow" "cyan" "magenta" "gray" "orange" "transparent"`；未知名称返回 magenta 便于发现。`to_rgba` 接受名称或数组（3 元素补 alpha=255）。

### 控件

#### panel(handle, rect, fill) / button(handle, rect, text, color) / label(handle, x, y, text, color)

即时模式绘制助手。`rect` 为 `[x, y, w, h]`。`button` 画填充背景 + 左上角标题。

### 事件

#### post_key(key, mods) / post_mouse_down(x, y, button) / post_mouse_up(x, y, button) / post_mouse_move(x, y) / post_resize(w, h) / post_close()

向队列注入事件（模拟后端用；真实后端由 `gui_poll_event` 提供）。

#### peek_event_kind() / peek_event_x() / peek_event_y() / peek_event_data()

非消费读取队首事件字段。

#### next_event_kind() / next_event_x() / next_event_y() / next_event_data()

读取并消费队首事件；队列空时返回 `""` / 0。**一次只取一个字段**，想读多个请先用 `peek_*`。

#### has_events() / event_count() / pop_event() / poll_event()

队列状态与 `Result` 形式的取事件。

### 几何

#### hit_test(x, y, rx, ry, rw, rh) / hit_test_rect(x, y, rect) / rect_contains(rect, x, y)

命中测试（左上闭、右下开）。

#### center_of(rect) / clamp_int(v, lo, hi) / lerp(a, b, t) / slider_ratio(x, press_x, w)

矩形中心、钳制、线性插值、滑块比例（已钳制 0..1，零宽返回 0）。

### 字体

#### load_font(path) / font_count() / set_font_cell(w, h) / font_cell()

字体句柄登记与单元格尺寸（默认 8x16）。字体是**不透明句柄**，没有字形度量。

## Layout

```
gui/
├── mailib.ini          # [module] name=gui / version=0.1.0 / entry=lib.mai
├── lib.mai             # 全部实现（纯 Mai，无 extern）
├── README.md
├── example/dashboard.mai   # 窗口绘制 + 显示列表 + 事件演示
└── test/test_gui.mai       # 98 个断言
```

## Usage

```mai
import "gui"

var win = gui.open_window("Demo", 480, 320)
gui.begin(win)
gui.panel(win, [0, 0, 480, 40], [30, 30, 40])
gui.button(win, [10, 10, 120, 24], "Refresh", [70, 130, 180])
gui.draw_text(win, 10, 60, "Hello", [230, 230, 230])
gui.present(win)

var n = gui.cmd_count()
```

## Tests

```
cargo run -p mailang-cli --release -- run libs/gui/test/test_gui.mai
```

全部通过时退出码 `0`，任一失败为 `1`。

## 实机测试（Windows）

CLI 内置 Win32 后端，`--host=gui` 会把桩函数替换成真实 Win32 调用：

| Hook | 实机实现 |
|------|----------|
| `gui_create_window` | `CreateWindowExW` 真实顶层窗口（标题来自参数），返回 `HWND` 句柄 |
| `gui_close_window` | `PostMessageW(WM_CLOSE)` |
| `gui_poll_event` | `PeekMessageW` 轮询真实窗口消息（close/resize/key/mouse），**非阻塞** |
| `gui_present` | GDI 三步：内存 DC 选入 DIB → `BitBlt` 到位图 DC → `UpdateWindow` |
| `gui_draw_rect` | 在 32 位 top-down DIB 上 `FillRect` |
| `gui_draw_text` | `SetTextColorW` + `TextOutW`（UTF-16，默认GUI 字体） |
| `gui_draw_line` | `MoveToEx` + `LineTo` |
| `gui_draw_circle` | `Ellipse`（`filled=false` 时当前实现仍填充，见限制） |
| `gui_load_font` | `AddFontResourceExW`（FR_PRIVATE），失败返回错误 |

渲染模型：每个窗口持有一张 32 位 top-down DIB，`gui_present` 时 blit 到屏幕。

```
cargo run -p mailang-cli --release -- run --host=gui libs/gui/test/host_gui.mai
```

该脚本断言真实窗口创建成功、绘制钩子返回 `"win32"`、显示列表仍可读、`present` 生效、窗口可关闭。**Windows 实测 7/7 通过**（窗口创建 → GDI 绘制 → present → 销毁全链路）。

限制：单色文本、无抗锯齿；`draw_circle` 的空心模式尚未实现（当前总是实心）；窗口不响应 `WM_PAINT` 重绘，只在 `present` 时刷新。

## Honest limits

- 未经 host 注册前不创建真实窗口；绘制只是内存里的命令列表。
- **测试状态**：模拟套件 `test_gui.mai` 在两个后端都 **98/98** 通过（CI 矩阵）；**宿主套件 `host_gui.mai` 在两个后端都 7/7 通过**（Phase N 之后修好寄存器后端 `TailCall` 绕过宿主覆盖的问题，见 `ROADMAP.md` 13.8）。
- **没有 `draw_rect_outline` / `progress_bar` 助手**：历史上是因为寄存器后端在 `if … return` 汇合点的栈高错误（Phase N N3 已修）。栈式后端在"把参数继续下传"的形状（如 `to_rgba(color)`）下仍会报错，所以需要边框时请直接发四条 `draw_rect`（颜色用字面量），进度条按已算好的像素宽度发 `draw_rect`。
- 颜色只接受**名称或数组**两种形式：语言没有 `type_of`，而`len()` / 索引 / 算术在类型不符时都是**抛错**而非返回 false，无法在无 `try/catch` 的前提下安全区分标量颜色（如 `0xFF0000`）与容器，故不支持标量。
- 控件都是即时模式绘制助手，不做布局、不处理交互状态、不做主题。命中判断请用 `hit_test`。
- 字体只是不透明句柄 + 固定单元格尺寸，没有字形度量、没有换行、没有富文本。
- 坐标是虚拟像素整数，没有 DPI/HiDPI缩放，没有裁剪矩形（画到窗口外不会被裁掉）。
- 事件模型是「队列 + 取走」一次性消费，没有键盘焦点、没有事件合并、没有持续按键重复。
- `open_window` 返回 0 表示创建失败；因为语言没有 `try/catch`，库内不做更复杂的错误分支。
