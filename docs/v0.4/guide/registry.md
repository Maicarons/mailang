# 包注册表（Registry）

MaìLang 内置 **文件系统优先** 的小型包注册表，行为对齐 crates.io 的核心工作流（发布 / 安装 / 搜索 / 撤回 / 索引），但完全可离线使用，也可把注册表目录当静态文件托管后走 HTTP。

> 规范字段与布局详见 [MODULE_SPEC.md](../MODULE_SPEC.md#包注册表registry)。

## 快速开始

```bash
# 1. 创建一个空注册表
mailang registry init ./.mailang-registry

# 2. 在包项目里发布（需要 mailang.toml + lib.mai）
mailang publish --registry ./.mailang-registry

# 3. 在另一个项目里安装
mailang install mypkg --registry ./.mailang-registry

# 4. 搜索
mailang search json --registry ./.mailang-registry

# 5. 撤回坏版本（install 默认拒绝）
mailang yank mypkg 0.1.0 --registry ./.mailang-registry
```

## 目录布局

```
<registry-root>/
├── registry.toml
├── index/<name-prefix>/<name>       # JSON Lines 索引
└── pkgs/<name>/
    ├── <vers>/                      # 包目录拷贝
    └── <name>-<vers>.mpkg           # 单文件包（HTTP 用）
```

`name-prefix` 规则（与 crates.io 相同）：

- 1 字符 → `1/<name>`
- 2 字符 → `2/<name>`
- 3 字符 → `3/<首字符>/<name>`
- ≥4 字符 → `<前两字符>/<name>`

例如 `json` 的索引路径为 `index/js/json`。

## 默认注册表位置

按以下优先级解析（可用 `--registry` 覆盖）：

1. 环境变量 `MAILANG_REGISTRY`
2. 当前目录下 `./.mailang-registry`
3. 用户目录 `~/.mailang/registry`（Windows: `%USERPROFILE%\.mailang\registry\`）

```bash
# 全局指定注册表
export MAILANG_REGISTRY=/var/mailang/registry   # 或 https://reg.example.com

mailang publish
mailang install mypkg
```

## 发布（publish）

在包项目根目录（含 `mailang.toml`）执行：

```toml
# mailang.toml
[package]
name = "greet"
version = "0.1.0"
description = "hello helpers"
```

```mai
// lib.mai
fn ping() {
  return "greet"
}
```

```bash
mailang publish --registry ./.mailang-registry
# published greet@0.1.0 -> ./.mailang-registry
#   cksum: a1b2c3d4e5f60718
#   dir:   .\.mailang-registry\pkgs\greet\0.1.0
#   mpkg:  .\.mailang-registry\pkgs\greet\greet-0.1.0.mpkg
```

行为说明：

| 规则 | 说明 |
|------|------|
| 打包内容 | `mailang.toml`、`lib.mai`、`mailib.ini`、`README`/`README.md`、顶层 `*.mai` |
| 内容哈希 | 对 `.mpkg` 字节做 FNV-1a 64，写入索引 `cksum` |
| 版本冲突 | 同 `name@vers` 已存在则拒绝；`--allow-republish` 允许覆盖 |
| 依赖记录 | 索引 `deps` 列出 `[dependencies]` 中的名字 |

## 安装（install）

```bash
mailang install greet                 # 最新非 yanked
mailang install greet@0.1.0           # 精确版本
mailang install greet --global        # 装到 ~/.mailang/pkg/greet
mailang install greet --allow-yanked  # 允许安装已撤回版本
mailang install greet --no-manifest   # 不改写 mailang.toml
```

安装结果：

1. 包文件拷贝到 `vendor/<name>/`（或 `--global` 时 `~/.mailang/pkg/<name>/`）
2. 默认改写 `mailang.toml` `[dependencies]`，指向 `vendor/<name>` 的 path 依赖
3. 随后 `mailang deps` / `mailang run` 可直接离线解析

示例改写后的 `mailang.toml`：

```toml
[package]
name = "app"
version = "0.0.1"

[dependencies]
# greet = registry install -> vendor/greet (do not edit by hand)
greet = "vendor/greet"
```

## 搜索（search）

```bash
mailang search hello --registry ./.mailang-registry
# NAME                 VERSION      YANKED   DESCRIPTION
# greet                0.1.0        no       hello helpers
```

匹配规则：对包名与 `description` 做**不区分大小写**的子串匹配。空查询列出全部。
HTTP 注册表无法遍历 `index/`，请在本地镜像上搜索。

## 撤回（yank）

```bash
mailang yank greet 0.1.0 --registry ./.mailang-registry
mailang yank greet 0.1.0 --undo --registry ./.mailang-registry   # 恢复
```

- 撤回后 `install` 默认失败（错误提示可用 `--allow-yanked`）。
- 裸名解析「最新」时会跳过 yanked 版本；若全部 yanked 则报错。

## HTTP 托管

把 `<registry-root>` 整个目录放到任意静态服务器即可：

```
https://reg.example.com/index/js/json
https://reg.example.com/pkgs/greet/greet-0.1.0.mpkg
```

```bash
mailang install greet --registry https://reg.example.com
```

实现细节：

- 读取索引 / 包通过 `curl -fsSL`（**不引入 HTTP 依赖 crate**）
- 安装走 `.mpkg` 单文件并校验 `cksum`
- **写操作**（`publish` / `yank` / `registry init`）需要文件系统路径，HTTP 只读

## 离线与工作流建议

1. **本地开发**：`registry init ./.mailang-registry`，包与应用同机 publish/install。
2. **团队共享**：把注册表目录放进共享盘或 Git，或 rsync 到静态服务器。
3. **CI**：在流水线里 `publish` 到内网路径，产物目录即注册表快照。
4. **与 path/github 依赖并存**：`install` 写入的是 path 依赖，与 `mailang add`、`mailang.lock` 完全兼容。

## 故障排查

| 现象 | 处理 |
|------|------|
| `already published` |  bump `version`，或 `--allow-republish` |
| `yanked: …` | `--allow-yanked`，或 `mailang yank … --undo` |
| `checksum mismatch` | 索引与 `.mpkg` 不一致，重新 `publish` 或修复镜像 |
| `publish requires a filesystem registry` | 不要对 `http://` 执行 publish，先写本地再同步 |
| `search requires a filesystem registry` | 在本地 `index/` 镜像上搜索 |

## API 入口

`crates/mailang-module/src/registry.rs`：

- `RegistrySource::default_source` / `parse` / `init`
- `publish` / `install` / `search` / `set_yanked` / `resolve_version`
- `read_index` / `write_index` / `upsert_index_entry`
- `pack_mpkg` / `unpack_mpkg` / `name_prefix`
