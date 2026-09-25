# 体积仪表盘（Footprint Dashboard）

> **相关**：[IoT 部署](/v0.3/guide/iot) · [真实硬件](/v0.3/guide/hardware) · 脚本 [`benchmark/measure_size.py`](https://github.com/Maicarons/mailang/blob/master/benchmark/measure_size.py)

本页说明如何用 `benchmark/measure_size.py` 测量 MaìLang 产物体积，以及如何解读输出。**本页不编造任何未实测数字**：MaìLang 体积以本机测量为准；第三方运行时（MicroPython 等）只给对比注意点与占位符。

## 如何运行

```bash
python benchmark/measure_size.py
```

脚本会：

1. `cargo build --release -p mailang-cli`，打印 **host CLI** 体积
2. 对已安装的嵌入式 target 构建 `mailang-bytecode`（`--no-default-features`），打印 **rlib** 体积
3. 输出一段可直接粘贴的 **Markdown 表格**（本机日期 / 平台写在表头上方）

未安装的 target 会标成 `—` 并注明原因，而不是填 0。

## 输出怎么读

| 列 | 含义 |
|----|------|
| Artifact | 产物：`mailang-cli` 或 `mailang-bytecode` rlib |
| Target | host 或嵌入式 triple 的用途标签 |
| Size (bytes) | **本机实测**字节数 |
| Size | 人类可读（KiB / MiB） |
| Notes | 产物范围说明 |

### 关键区分

- **host CLI**：完整 `std` 解释器 + REPL + 模块系统，**不是** MCU 固件。
- **mailang-bytecode rlib**：`no_std` 可移植核心（指令、`Value`、`.mailangbc` 格式）。它是**中间产物**，不是可烧录 `.bin`。
- **最终固件** = rlib/VM 代码 + 你的 HAL + 链接脚本 + 启动代码。请用链接器 map / `size` 工具看最终镜像。

## 示例表（占位 — 请以本机运行为准）

下面数字列是**占位符**。请运行脚本后把真实输出贴进发布说明或 PR：

| Artifact | Target | Size (bytes) | Size | Notes |
|----------|--------|-------------:|------|-------|
| mailang-cli (release) | host | *(run script)* | *(run script)* | full std CLI |
| mailang-bytecode rlib | embedded (Cortex-M4F) | *(run script / target installed)* | *(…)* | no_std + alloc core |
| mailang-bytecode rlib | embedded (ESP32-C3 class) | *(run script / target installed)* | *(…)* | no_std + alloc core |

## MicroPython / JerryScript 对比注意点

**不要**把网上的 “MicroPython ~300KB” 之类数字直接和 rlib 对比：

| 因素 | 说明 |
|------|------|
| 产物形态不同 | rlib ≠ `.bin`；MicroPython 数字通常是完整固件镜像 |
| 功能范围不同 | MicroPython 含 REPL、文件系统、大量内置模块；MaìLang 嵌入式路径可只链 bytecode + VM |
| 板级差异 | 同一运行时在不同 flash/链接脚本下体积差很多 |
| 构建选项 | LTO、`opt-level = "z"`、panic 策略、是否 strip 会改变结果 |
| 版本漂移 | 上游版本更新后数字会变 |

对比时请：

1. 在**同一块板**、**同一工具链**、**相近功能集**下两边各测一次镜像；
2. 在报告里写明 commit、profile、target、是否含 FS/REPL；
3. 脚本输出的对比行只作 scale 备注，**保持占位或写“vendor-dependent”**，除非你自己测过。

## 嵌入式目标怎么补测

```bash
rustup target add thumbv7em-none-eabihf
rustup target add riscv32imc-unknown-none-elf
python benchmark/measure_size.py
```

最终 MCU 镜像请另测：

```bash
# 示例：读取链接产物
arm-none-eabi-size firmware.elf
# 或解析 .map 看 .text/.rodata/.bss
```

## 范围声明（诚实边界）

- 本工具只测 **host CLI** 与 **mailang-bytecode rlib**。
- 它**不**测量完整 ESP32 / Cortex-M4 固件。
- 它**不**内置任何第三方运行时基准数字；对比行故意留空或标注 “measure locally / vendor-dependent”。

运行后请把 Markdown 表格连同日期与 triple 一并归档，避免过期数字被再次引用。
