# Package Registry

> **Project Links**: [GitHub](https://github.com/Maicarons/mailang)

MaìLang ships with a small **filesystem-first** package registry. Its behavior lines up with the core crates.io workflow (publish / install / search / yank / index), but it works fully offline. You can also host the registry directory as static files and serve it over HTTP.

> Canonical fields and layout are documented in [MODULE_SPEC.md](../../MODULE_SPEC.md#包注册表registry).

## Quick Start

```bash
# 1. Create an empty registry
mailang registry init ./.mailang-registry

# 2. Publish from a package project (requires mailang.toml + lib.mai)
mailang publish --registry ./.mailang-registry

# 3. Install in another project
mailang install mypkg --registry ./.mailang-registry

# 4. Search
mailang search json --registry ./.mailang-registry

# 5. Yank a bad version (install rejects it by default)
mailang yank mypkg 0.1.0 --registry ./.mailang-registry
```

## Directory Layout

```
<registry-root>/
├── registry.toml
├── index/<name-prefix>/<name>       # JSON Lines index
└── pkgs/<name>/
    ├── <vers>/                      # package directory copy
    └── <name>-<vers>.mpkg           # single-file package (for HTTP)
```

`name-prefix` rules (same as crates.io):

- 1 character → `1/<name>`
- 2 characters → `2/<name>`
- 3 characters → `3/<first-char>/<name>`
- ≥4 characters → `<first-two-chars>/<name>`

For example, the index path for `json` is `index/js/json`.

## Default Registry Location

Resolved in the following priority order (override with `--registry`):

1. Environment variable `MAILANG_REGISTRY`
2. `./.mailang-registry` in the current directory
3. User directory `~/.mailang/registry` (Windows: `%USERPROFILE%\.mailang\registry\`)

```bash
# Specify a registry globally
export MAILANG_REGISTRY=/var/mailang/registry   # or https://reg.example.com

mailang publish
mailang install mypkg
```

## Publish

Run in the package project root (the directory that contains `mailang.toml`):

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

Behavior notes:

| Rule | Description |
|------|-------------|
| Packaged contents | `mailang.toml`, `lib.mai`, `mailib.ini`, `README`/`README.md`, top-level `*.mai` |
| Content hash | FNV-1a 64 over the `.mpkg` bytes, written to the index as `cksum` |
| Version conflict | Rejected if the same `name@vers` already exists; `--allow-republish` allows overwrite |
| Dependency record | The index `deps` field lists names from `[dependencies]` |

## Install

```bash
mailang install greet                 # latest non-yanked version
mailang install greet@0.1.0           # exact version
mailang install greet --global        # install to ~/.mailang/pkg/greet
mailang install greet --allow-yanked  # allow installing a yanked version
mailang install greet --no-manifest   # do not rewrite mailang.toml
```

Install results:

1. Package files are copied to `vendor/<name>/` (or `~/.mailang/pkg/<name>/` with `--global`)
2. By default, `mailang.toml` `[dependencies]` is rewritten to a path dependency pointing at `vendor/<name>`
3. Afterwards, `mailang deps` / `mailang run` resolve fully offline

Example of a rewritten `mailang.toml`:

```toml
[package]
name = "app"
version = "0.0.1"

[dependencies]
# greet = registry install -> vendor/greet (do not edit by hand)
greet = "vendor/greet"
```

## Search

```bash
mailang search hello --registry ./.mailang-registry
# NAME                 VERSION      YANKED   DESCRIPTION
# greet                0.1.0        no       hello helpers
```

Matching rules: case-insensitive substring match on the package name and `description`. An empty query lists everything.
HTTP registries cannot walk `index/` — search on a local mirror instead.

## Yank

```bash
mailang yank greet 0.1.0 --registry ./.mailang-registry
mailang yank greet 0.1.0 --undo --registry ./.mailang-registry   # restore
```

- After a yank, `install` fails by default (the error message suggests `--allow-yanked`).
- Bare-name "latest" resolution skips yanked versions; if every version is yanked, it errors.

## HTTP Hosting

Put the entire `<registry-root>` directory on any static file server:

```
https://reg.example.com/index/js/json
https://reg.example.com/pkgs/greet/greet-0.1.0.mpkg
```

```bash
mailang install greet --registry https://reg.example.com
```

Implementation details:

- Index / package reads go through `curl -fsSL` (**no HTTP dependency crate is introduced**)
- Install uses the `.mpkg` single-file package and verifies `cksum`
- **Write operations** (`publish` / `yank` / `registry init`) require a filesystem path; HTTP is read-only

## Offline and Workflow Tips

1. **Local development**: `registry init ./.mailang-registry`, then publish/install packages and apps on the same machine.
2. **Team sharing**: put the registry directory on a shared drive or in Git, or rsync it to a static server.
3. **CI**: `publish` to an internal path in the pipeline; the artifact directory doubles as a registry snapshot.
4. **Coexisting with path/github dependencies**: `install` writes path dependencies, fully compatible with `mailang add` and `mailang.lock`.

## Troubleshooting

| Symptom | Fix |
|---------|-----|
| `already published` | Bump `version`, or use `--allow-republish` |
| `yanked: …` | Use `--allow-yanked`, or `mailang yank … --undo` |
| `checksum mismatch` | Index and `.mpkg` disagree — `publish` again or repair the mirror |
| `publish requires a filesystem registry` | Do not publish to `http://`; write locally first, then sync |
| `search requires a filesystem registry` | Search on a local `index/` mirror |

## API Entry Points

`crates/mailang-module/src/registry.rs`:

- `RegistrySource::default_source` / `parse` / `init`
- `publish` / `install` / `search` / `set_yanked` / `resolve_version`
- `read_index` / `write_index` / `upsert_index_entry`
- `pack_mpkg` / `unpack_mpkg` / `name_prefix`
