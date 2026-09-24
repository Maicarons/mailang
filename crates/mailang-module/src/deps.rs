//! Minimal project dependency resolution via `mailang.toml`.
//!
//! Supported layout (see `docs/MODULE_SPEC.md`):
//!
//! ```toml
//! [package]
//! name = "my-app"
//! version = "0.1.0"
//!
//! [dependencies]
//! json = "libs/json"
//! utils = "./local/utils"
//! sensors = { path = "../shared/sensors" }
//! cool = { github = "org/repo" }
//! other = { git = "https://example.com/org/repo.git" }
//! ```
//!
//! Resolution also maintains `mailang.lock` (SEMVER-lite + content hash).
//! `github` / `git` deps are offline-safe: look in `vendor/<name>/` then
//! `~/.mailang/pkg/<name>/`. Network fetch is a host/CI operation (curl/git).

use std::collections::HashMap;
use std::path::{Path, PathBuf};

// ---------------------------------------------------------------------------
// SEMVER-lite
// ---------------------------------------------------------------------------

/// SEMVER-lite triple (`MAJOR.MINOR.PATCH`; missing parts default to 0).
pub type SemverLite = (u64, u64, u64);

/// Parse `1`, `1.2`, or `1.2.3` into a SEMVER-lite triple.
/// Extra dot-segments are ignored; non-numeric segments fail the whole parse
/// only when they occupy a present slot and are non-empty.
pub fn parse_semver_lite(s: &str) -> Option<SemverLite> {
    let s = s.trim();
    if s.is_empty() {
        return None;
    }
    let mut parts = s.split('.');
    let major = parts.next()?.trim().parse::<u64>().ok()?;
    let minor = match parts.next() {
        Some(p) if !p.trim().is_empty() => p.trim().parse::<u64>().ok()?,
        _ => 0,
    };
    let patch = match parts.next() {
        Some(p) if !p.trim().is_empty() => p.trim().parse::<u64>().ok()?,
        _ => 0,
    };
    Some((major, minor, patch))
}

/// SEMVER-lite equality (normalizes missing minor/patch to 0).
pub fn semver_lite_eq(a: &str, b: &str) -> bool {
    match (parse_semver_lite(a), parse_semver_lite(b)) {
        (Some(x), Some(y)) => x == y,
        _ => a.trim() == b.trim(),
    }
}

/// SEMVER-lite ordering: `-1` / `0` / `1` like `strcmp`. Unparseable sorts as (0,0,0).
pub fn semver_lite_cmp(a: &str, b: &str) -> i32 {
    let x = parse_semver_lite(a).unwrap_or((0, 0, 0));
    let y = parse_semver_lite(b).unwrap_or((0, 0, 0));
    x.cmp(&y) as i32
}

// ---------------------------------------------------------------------------
// Content hash (FNV-1a 64)
// ---------------------------------------------------------------------------

/// Simple non-crypto content hash of raw bytes (FNV-1a 64, 16 hex chars).
pub fn content_hash(bytes: &[u8]) -> String {
    let mut h: u64 = 0xcbf2_9ce4_8422_2325;
    for &b in bytes {
        h ^= b as u64;
        h = h.wrapping_mul(0x0000_0100_0000_01b3);
    }
    format!("{:016x}", h)
}

/// Hash the contents of a file; empty string if unreadable.
pub fn content_hash_file(path: &Path) -> String {
    match std::fs::read(path) {
        Ok(bytes) => content_hash(&bytes),
        Err(_) => String::new(),
    }
}

// ---------------------------------------------------------------------------
// Lockfile (`mailang.lock`)
// ---------------------------------------------------------------------------

/// One locked dependency record.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LockEntry {
    pub name: String,
    /// Version from `mailib.ini` (SEMVER-lite string; empty if unknown).
    pub version: String,
    /// Path (project-root-relative with `/` when possible).
    pub path: String,
    /// Content hash of the entry file (`lib.mai` / `mailib.ini` entry).
    pub hash: String,
}

/// Outcome of ensuring `mailang.lock` during resolve.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LockOutcome {
    /// No lock file existed; one was written.
    Created,
    /// Lock existed and matched the resolution; left unchanged (reused).
    Matched,
    /// Lock existed but differed; rewritten.
    Refreshed,
}

/// Per-entry lock verification status.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum EntryLockStatus {
    Ok,
    /// Remote dep recorded in lock but not fetched yet (empty hash, consistent).
    Planned,
    VersionMismatch,
    PathMismatch,
    HashMismatch,
    MissingInLock,
    MissingOnDisk,
    ExtraInLock,
}

/// One row of a lock verification report.
#[derive(Debug, Clone)]
pub struct LockEntryStatus {
    pub name: String,
    pub status: EntryLockStatus,
    pub version: String,
    pub path: String,
    pub hash: String,
}

/// Lock verification report (does not write).
#[derive(Debug, Clone)]
pub struct LockReport {
    pub lock_path: PathBuf,
    pub exists: bool,
    pub entries: Vec<LockEntryStatus>,
}

impl LockReport {
    /// True when every entry is `Ok` or `Planned` (and the lock file exists).
    pub fn is_healthy(&self) -> bool {
        self.exists
            && self
                .entries
                .iter()
                .all(|e| e.status == EntryLockStatus::Ok || e.status == EntryLockStatus::Planned)
    }
}

/// Normalize a path for lock storage: relative to `root` with `/` separators when possible.
pub fn lock_path_string(root: &Path, path: &Path) -> String {
    // Lexical strip first (works for not-yet-created vendor paths).
    if let Ok(rel) = path.strip_prefix(root) {
        let s = rel.to_string_lossy().replace('\\', "/");
        if !s.is_empty() {
            return s;
        }
    }
    let abs_root = std::fs::canonicalize(root).unwrap_or_else(|_| root.to_path_buf());
    let abs_path = std::fs::canonicalize(path).unwrap_or_else(|_| path.to_path_buf());
    if let Ok(rel) = abs_path.strip_prefix(&abs_root) {
        rel.to_string_lossy().replace('\\', "/")
    } else {
        path.to_string_lossy().replace('\\', "/")
    }
}

fn format_lock_entry(e: &LockEntry) -> String {
    let version = if e.version.is_empty() {
        "-".to_string()
    } else {
        e.version.clone()
    };
    let hash = if e.hash.is_empty() {
        "-".to_string()
    } else {
        e.hash.clone()
    };
    format!("{} | {} | {} | {}", e.name, version, e.path, hash)
}

/// Serialize entries to `mailang.lock` text.
pub fn format_lockfile(entries: &[LockEntry]) -> String {
    let mut out = String::new();
    out.push_str("# mailang.lock -- generated by mailang; do not edit by hand.\n");
    out.push_str("# SEMVER-lite + content hash of the module entry file.\n");
    out.push_str("# fields: name | version | path | hash\n");
    let mut sorted: Vec<&LockEntry> = entries.iter().collect();
    sorted.sort_by(|a, b| a.name.cmp(&b.name));
    for e in sorted {
        out.push_str(&format_lock_entry(e));
        out.push('\n');
    }
    out
}

/// Parse `mailang.lock` text into entries (skips comments / blank lines).
pub fn parse_lockfile(content: &str) -> Vec<LockEntry> {
    let mut out = Vec::new();
    for raw in content.lines() {
        let line = raw.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        let parts: Vec<&str> = line.split('|').map(|p| p.trim()).collect();
        if parts.len() < 4 {
            continue;
        }
        let un = |s: &str| {
            if s == "-" {
                String::new()
            } else {
                s.to_string()
            }
        };
        out.push(LockEntry {
            name: parts[0].to_string(),
            version: un(parts[1]),
            path: un(parts[2]),
            hash: un(parts[3]),
        });
    }
    out
}

/// Read `mailang.lock` under `root` (empty vec if missing/unreadable).
pub fn read_lockfile(root: &Path) -> Vec<LockEntry> {
    let path = root.join("mailang.lock");
    match std::fs::read_to_string(&path) {
        Ok(s) => parse_lockfile(&s),
        Err(_) => Vec::new(),
    }
}

/// Write `mailang.lock` under `root`.
pub fn write_lockfile(root: &Path, entries: &[LockEntry]) -> std::io::Result<()> {
    std::fs::write(root.join("mailang.lock"), format_lockfile(entries))
}

fn lock_entries_match(existing: &[LockEntry], fresh: &[LockEntry]) -> bool {
    if existing.len() != fresh.len() {
        return false;
    }
    for f in fresh {
        let Some(e) = existing.iter().find(|e| e.name == f.name) else {
            return false;
        };
        if !semver_lite_eq(&e.version, &f.version) || e.path != f.path || e.hash != f.hash {
            return false;
        }
    }
    true
}

/// Build lock entries from a resolution (hashes entry files on disk).
pub fn lock_entries_from_resolved(root: &Path, resolved: &[ResolvedDependency]) -> Vec<LockEntry> {
    resolved
        .iter()
        .map(|d| LockEntry {
            name: d.name.clone(),
            version: d.version.clone(),
            path: lock_path_string(root, &d.path),
            hash: d.hash.clone(),
        })
        .collect()
}

/// Write/refresh `mailang.lock` for `resolved`. Reuses the file when it matches.
pub fn ensure_lock(root: &Path, resolved: &[ResolvedDependency]) -> LockOutcome {
    let fresh = lock_entries_from_resolved(root, resolved);
    let existing = read_lockfile(root);
    let had = root.join("mailang.lock").is_file();
    if had && lock_entries_match(&existing, &fresh) {
        return LockOutcome::Matched;
    }
    let _ = write_lockfile(root, &fresh);
    if had {
        LockOutcome::Refreshed
    } else {
        LockOutcome::Created
    }
}

/// Verify `mailang.lock` against a resolution without writing.
pub fn verify_lock(root: &Path, resolved: &[ResolvedDependency]) -> LockReport {
    let lock_path = root.join("mailang.lock");
    let exists = lock_path.is_file();
    let existing = read_lockfile(root);
    let fresh = lock_entries_from_resolved(root, resolved);
    let mut entries = Vec::new();

    for f in &fresh {
        if f.hash.is_empty() {
            // Not on disk: "planned" when the lock agrees (unfetched remote), else missing.
            let status = match existing.iter().find(|e| e.name == f.name) {
                Some(e)
                    if e.hash.is_empty()
                        && e.path == f.path
                        && semver_lite_eq(&e.version, &f.version) =>
                {
                    EntryLockStatus::Planned
                }
                Some(_) => EntryLockStatus::MissingOnDisk,
                None => EntryLockStatus::MissingInLock,
            };
            entries.push(LockEntryStatus {
                name: f.name.clone(),
                status,
                version: f.version.clone(),
                path: f.path.clone(),
                hash: f.hash.clone(),
            });
            continue;
        }
        match existing.iter().find(|e| e.name == f.name) {
            None => entries.push(LockEntryStatus {
                name: f.name.clone(),
                status: EntryLockStatus::MissingInLock,
                version: f.version.clone(),
                path: f.path.clone(),
                hash: f.hash.clone(),
            }),
            Some(e) => {
                let status = if !semver_lite_eq(&e.version, &f.version) {
                    EntryLockStatus::VersionMismatch
                } else if e.path != f.path {
                    EntryLockStatus::PathMismatch
                } else if e.hash != f.hash {
                    EntryLockStatus::HashMismatch
                } else {
                    EntryLockStatus::Ok
                };
                entries.push(LockEntryStatus {
                    name: f.name.clone(),
                    status,
                    version: f.version.clone(),
                    path: f.path.clone(),
                    hash: f.hash.clone(),
                });
            }
        }
    }
    for e in &existing {
        if !fresh.iter().any(|f| f.name == e.name) {
            entries.push(LockEntryStatus {
                name: e.name.clone(),
                status: EntryLockStatus::ExtraInLock,
                version: e.version.clone(),
                path: e.path.clone(),
                hash: e.hash.clone(),
            });
        }
    }

    LockReport {
        lock_path,
        exists,
        entries,
    }
}

// ---------------------------------------------------------------------------
// Manifest / dependency kinds
// ---------------------------------------------------------------------------

/// Where a dependency comes from.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DepSource {
    /// Local path (directory or `.mai` file).
    Path,
    /// `github = "org/repo"` — offline: `vendor/<name>/` then `~/.mailang/pkg/<name>/`.
    Github(String),
    /// `git = "<url>"` — same offline lookup as `Github`.
    Git(String),
}

/// One entry from `[dependencies]`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Dependency {
    pub name: String,
    /// Path as written in `mailang.toml` (relative to project root, or absolute).
    /// Empty for `github` / `git` sources.
    pub path: String,
    pub source: DepSource,
}

/// Parsed subset of `mailang.toml`.
#[derive(Debug, Clone, Default)]
pub struct ProjectManifest {
    pub name: Option<String>,
    pub version: Option<String>,
    pub description: Option<String>,
    pub dependencies: Vec<Dependency>,
}

/// A dependency resolved to a concrete module directory (or file).
#[derive(Debug, Clone)]
pub struct ResolvedDependency {
    pub name: String,
    /// Version from `mailib.ini` if present, else empty.
    pub version: String,
    /// Absolute or project-root-relative path that was resolved.
    pub path: PathBuf,
    /// Entry file name (`lib.mai` unless `mailib.ini` overrides).
    pub entry: String,
    /// Content hash of the entry file bytes (empty if missing).
    pub hash: String,
    pub source: DepSource,
}

fn parse_inline_table(value: &str) -> (String, DepSource) {
    let inner = value.trim();
    let inner = inner
        .strip_prefix('{')
        .and_then(|s| s.strip_suffix('}'))
        .unwrap_or(inner);
    let mut path = String::new();
    let mut github: Option<String> = None;
    let mut git: Option<String> = None;
    for part in inner.split(',') {
        let part = part.trim();
        if part.is_empty() {
            continue;
        }
        let Some((k, v)) = part.split_once('=') else {
            continue;
        };
        let k = k.trim();
        let v = v.trim().trim_matches('"').trim();
        match k {
            "path" => path = v.to_string(),
            "github" => github = Some(v.to_string()),
            "git" => git = Some(v.to_string()),
            _ => {}
        }
    }
    if let Some(gh) = github {
        (String::new(), DepSource::Github(gh))
    } else if let Some(g) = git {
        (String::new(), DepSource::Git(g))
    } else {
        (path, DepSource::Path)
    }
}

/// Parse a minimal `mailang.toml` (package name/version + `[dependencies]`).
/// Unknown sections and non-string dependency values are ignored.
///
/// Dependency values:
/// - string → path dep (`json = "libs/json"`)
/// - inline table `{ path = "..." }`
/// - inline table `{ github = "org/repo" }`
/// - inline table `{ git = "https://..." }`
pub fn parse_manifest(content: &str) -> ProjectManifest {
    let mut manifest = ProjectManifest::default();
    #[derive(Clone, Copy, PartialEq)]
    enum Section {
        None,
        Package,
        Dependencies,
    }
    let mut section = Section::None;

    for raw in content.lines() {
        let line = raw.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        if line.starts_with('[') && line.ends_with(']') {
            let name = line[1..line.len() - 1].trim();
            section = match name {
                "package" => Section::Package,
                "dependencies" => Section::Dependencies,
                _ => Section::None,
            };
            continue;
        }
        let Some((key, value)) = line.split_once('=') else {
            continue;
        };
        let key = key.trim();
        let value = value.trim();
        match section {
            Section::Package => {
                let value = value.trim_matches('"').trim();
                match key {
                    "name" => manifest.name = Some(value.to_string()),
                    "version" => manifest.version = Some(value.to_string()),
                    "description" => manifest.description = Some(value.to_string()),
                    _ => {}
                }
            }
            Section::Dependencies => {
                if key.is_empty() {
                    continue;
                }
                let (path, source) = if value.starts_with('{') {
                    parse_inline_table(value)
                } else {
                    (value.trim_matches('"').trim().to_string(), DepSource::Path)
                };
                manifest.dependencies.push(Dependency {
                    name: key.to_string(),
                    path,
                    source,
                });
            }
            Section::None => {}
        }
    }
    manifest
}

/// Walk up from `start` looking for `mailang.toml`. Returns the directory containing it.
pub fn find_project_root(start: &Path) -> Option<PathBuf> {
    let mut dir = if start.is_file() {
        start.parent()?.to_path_buf()
    } else {
        start.to_path_buf()
    };
    loop {
        if dir.join("mailang.toml").is_file() {
            return Some(dir);
        }
        if !dir.pop() {
            return None;
        }
    }
}

/// Load and parse `mailang.toml` under `root`.
pub fn load_manifest(root: &Path) -> Option<ProjectManifest> {
    let path = root.join("mailang.toml");
    let content = std::fs::read_to_string(&path).ok()?;
    Some(parse_manifest(&content))
}

/// Parse `mailib.ini` fields used for listing (version + entry).
fn module_meta(dir: &Path) -> (String, String) {
    let ini_path = dir.join("mailib.ini");
    let mut version = String::new();
    let mut entry = "lib.mai".to_string();
    if let Ok(content) = std::fs::read_to_string(&ini_path) {
        for line in content.lines() {
            let line = line.trim();
            if line.is_empty() || line.starts_with('#') || line.starts_with('[') {
                continue;
            }
            if let Some((key, value)) = line.split_once('=') {
                match key.trim() {
                    "version" => version = value.trim().trim_matches('"').to_string(),
                    "entry" => entry = value.trim().trim_matches('"').to_string(),
                    _ => {}
                }
            }
        }
    }
    (version, entry)
}

// ---------------------------------------------------------------------------
// Offline package dirs (vendor / ~/.mailang/pkg)
// ---------------------------------------------------------------------------

/// User home directory via `USERPROFILE` / `HOME` (no extra crates).
pub fn home_dir() -> Option<PathBuf> {
    for key in ["USERPROFILE", "HOME"] {
        if let Ok(h) = std::env::var(key) {
            if !h.is_empty() {
                return Some(PathBuf::from(h));
            }
        }
    }
    None
}

/// Global package cache: `~/.mailang/pkg/<name>`.
pub fn global_pkg_dir(name: &str) -> Option<PathBuf> {
    Some(home_dir()?.join(".mailang").join("pkg").join(name))
}

/// Project-local vendor dir: `vendor/<name>`.
pub fn vendor_dir(root: &Path, name: &str) -> PathBuf {
    root.join("vendor").join(name)
}

/// Offline lookup for `github` / `git` deps: `vendor/<name>/` then `~/.mailang/pkg/<name>/`.
/// Returns the package directory if present.
pub fn find_offline_package(root: &Path, name: &str) -> Option<PathBuf> {
    let v = vendor_dir(root, name);
    if v.is_dir() {
        return Some(v);
    }
    if let Some(g) = global_pkg_dir(name) {
        if g.is_dir() {
            return Some(g);
        }
    }
    None
}

/// Resolve a dependency path (relative to `root`) to a concrete module file.
fn resolve_dep_target(root: &Path, dep_path: &str) -> Option<(PathBuf, String)> {
    let joined = if Path::new(dep_path).is_absolute() {
        PathBuf::from(dep_path)
    } else {
        root.join(dep_path)
    };
    resolve_module_file(&joined)
}

fn resolve_module_file(joined: &Path) -> Option<(PathBuf, String)> {
    if joined.is_file() {
        let entry = joined
            .file_name()
            .map(|s| s.to_string_lossy().to_string())
            .unwrap_or_else(|| "lib.mai".to_string());
        return Some((joined.to_path_buf(), entry));
    }

    let mai = joined.with_extension("mai");
    if mai.is_file() {
        let entry = mai
            .file_name()
            .map(|s| s.to_string_lossy().to_string())
            .unwrap_or_else(|| "lib.mai".to_string());
        return Some((mai, entry));
    }

    if joined.is_dir() {
        let (_version_ignored, entry) = module_meta(joined);
        let lib_path = joined.join(&entry);
        if lib_path.is_file() {
            return Some((lib_path, entry));
        }
    }
    None
}

/// Resolve every `[dependencies]` entry under `root` and refresh `mailang.lock`.
/// Missing targets are omitted (present only if the path exists).
pub fn resolve_dependencies(root: &Path) -> Vec<ResolvedDependency> {
    resolve_dependencies_locked(root).0
}

/// Like [`resolve_dependencies`], but also reports the lock outcome.
pub fn resolve_dependencies_locked(root: &Path) -> (Vec<ResolvedDependency>, LockOutcome) {
    let resolved = resolve_dependencies_raw(root);
    let outcome = ensure_lock(root, &resolved);
    (resolved, outcome)
}

/// Resolve every `[dependencies]` entry without touching `mailang.lock`.
pub fn resolve_dependencies_raw(root: &Path) -> Vec<ResolvedDependency> {
    let Some(manifest) = load_manifest(root) else {
        return Vec::new();
    };
    let mut out = Vec::new();
    for dep in manifest.dependencies {
        let target = match &dep.source {
            DepSource::Path => {
                if dep.path.is_empty() {
                    None
                } else {
                    resolve_dep_target(root, &dep.path)
                }
            }
            DepSource::Github(_) | DepSource::Git(_) => {
                // Offline-safe: vendor/<name>/ then ~/.mailang/pkg/<name>/.
                // Network fetch is a host/CI operation (curl / git).
                find_offline_package(root, &dep.name).and_then(|dir| resolve_module_file(&dir))
            }
        };

        if let Some((file_path, entry)) = target {
            let module_dir = file_path.parent().map(|p| p.to_path_buf());
            let (version, entry_from_ini) = module_dir.map(|d| module_meta(&d)).unwrap_or_default();
            let entry = if entry_from_ini != "lib.mai" {
                entry_from_ini
            } else {
                entry
            };
            let hash = content_hash_file(&file_path);
            out.push(ResolvedDependency {
                name: dep.name,
                version,
                path: file_path,
                entry,
                hash,
                source: dep.source,
            });
        } else {
            // Keep unresolved entries visible with empty version and the raw path.
            let fallback = match &dep.source {
                DepSource::Path => root.join(&dep.path),
                _ => vendor_dir(root, &dep.name),
            };
            out.push(ResolvedDependency {
                name: dep.name.clone(),
                version: String::new(),
                path: fallback,
                entry: "lib.mai".to_string(),
                hash: String::new(),
                source: dep.source,
            });
        }
    }
    out
}

/// Map of name → resolved entry file path, used by the module loader.
pub fn dependency_map(root: &Path) -> HashMap<String, PathBuf> {
    resolve_dependencies(root)
        .into_iter()
        .filter(|d| !d.hash.is_empty() || d.path.exists())
        .map(|d| (d.name, d.path))
        .collect()
}

// ---------------------------------------------------------------------------
// `mailang add` planning (offline-safe; fetch is host-op)
// ---------------------------------------------------------------------------

/// A planned dependency add (does not fetch).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AddSpec {
    pub name: String,
    pub source: DepSource,
    /// Path string for path deps (empty otherwise).
    pub path: String,
}

impl AddSpec {
    /// TOML line for `[dependencies]`.
    pub fn toml_line(&self) -> String {
        match &self.source {
            DepSource::Path => format!("{} = \"{}\"", self.name, self.path),
            DepSource::Github(repo) => {
                format!("{} = {{ github = \"{}\" }}", self.name, repo)
            }
            DepSource::Git(url) => format!("{} = {{ git = \"{}\" }}", self.name, url),
        }
    }

    /// Comment noting that network fetch is a host/CI operation.
    pub fn plan_comment(&self) -> Option<String> {
        match &self.source {
            DepSource::Path => None,
            DepSource::Github(repo) => Some(format!(
                "# {} = github:{} -- fetch is host-op (curl/git); mailang only plans. \
                 Resolve offline via vendor/{}/ or ~/.mailang/pkg/{}/",
                self.name, repo, self.name, self.name
            )),
            DepSource::Git(url) => Some(format!(
                "# {} = git:{} -- fetch is host-op (curl/git); mailang only plans. \
                 Resolve offline via vendor/{}/ or ~/.mailang/pkg/{}/",
                self.name, url, self.name, self.name
            )),
        }
    }
}

fn name_from_repo(repo: &str) -> String {
    repo.rsplit('/')
        .next()
        .unwrap_or(repo)
        .trim_end_matches(".git")
        .to_string()
}

/// True when a bare spec is clearly a filesystem path (not `org/repo`).
/// Bare `org/repo` is mip-style GitHub and is *not* a path.
fn looks_like_path(s: &str) -> bool {
    s.starts_with("./")
        || s.starts_with("../")
        || s.starts_with('/')
        || s.starts_with('\\')
        || s.ends_with(".mai")
        || s.contains('\\')
}

/// Parse a `mailang add <spec>` argument (mip-style, offline plan only).
///
/// Forms:
/// - `name=path` / `./rel` / `../rel` / `abs/path` / `file.mai` → path dep
/// - `github:org/repo` / `name=github:org/repo` / `org/repo` → github dep
/// - `git:https://...` / `name=git:https://...` → git dep
pub fn parse_add_spec(spec: &str) -> Result<AddSpec, String> {
    let spec = spec.trim();
    if spec.is_empty() {
        return Err("empty dependency spec".to_string());
    }

    let (name_opt, body) = if let Some((n, b)) = spec.split_once('=') {
        let n = n.trim();
        if n.is_empty() {
            return Err(format!("invalid spec '{}': empty name before '='", spec));
        }
        (Some(n.to_string()), b.trim())
    } else {
        (None, spec)
    };

    // Explicit prefixes
    if let Some(rest) = body.strip_prefix("github:") {
        let repo = rest
            .trim()
            .trim_start_matches("https://github.com/")
            .trim_end_matches(".git")
            .to_string();
        if repo.is_empty() || !repo.contains('/') {
            return Err(format!("invalid github spec '{}': expected org/repo", spec));
        }
        let name = name_opt.unwrap_or_else(|| name_from_repo(&repo));
        return Ok(AddSpec {
            name,
            source: DepSource::Github(repo),
            path: String::new(),
        });
    }
    if let Some(rest) = body.strip_prefix("git:") {
        let url = rest.trim().to_string();
        if url.is_empty() {
            return Err(format!("invalid git spec '{}': expected URL", spec));
        }
        let name = name_opt.unwrap_or_else(|| name_from_repo(&url));
        return Ok(AddSpec {
            name,
            source: DepSource::Git(url),
            path: String::new(),
        });
    }

    // Named path
    if let Some(name) = name_opt {
        if body.starts_with("http://") || body.starts_with("https://") {
            return Ok(AddSpec {
                name,
                source: DepSource::Git(body.to_string()),
                path: String::new(),
            });
        }
        return Ok(AddSpec {
            name,
            source: DepSource::Path,
            path: body.to_string(),
        });
    }

    // Bare org/repo → github (mip-style), unless it clearly looks like a path.
    if !looks_like_path(body)
        && body.contains('/')
        && !body.contains("://")
        && body.split('/').count() == 2
        && body.split('/').all(|p| !p.is_empty())
    {
        let repo = body.trim_end_matches(".git").to_string();
        return Ok(AddSpec {
            name: name_from_repo(&repo),
            source: DepSource::Github(repo),
            path: String::new(),
        });
    }

    // Bare path
    let name = Path::new(body)
        .file_stem()
        .map(|s| s.to_string_lossy().to_string())
        .filter(|s| !s.is_empty())
        .unwrap_or_else(|| body.to_string());
    Ok(AddSpec {
        name,
        source: DepSource::Path,
        path: body.to_string(),
    })
}

/// Result of planning `mailang add` (manifest was written).
#[derive(Debug, Clone)]
pub struct AddPlan {
    pub spec: AddSpec,
    pub manifest_path: PathBuf,
    pub created_manifest: bool,
    pub updated: bool,
}

/// Plan an add: write the dep into `mailang.toml` only (no fetch).
/// Creates `mailang.toml` when missing. Replacing an existing name is allowed.
pub fn plan_add(root: &Path, spec: &str) -> Result<AddPlan, String> {
    let add = parse_add_spec(spec)?;
    let manifest_path = root.join("mailang.toml");
    let created_manifest = !manifest_path.is_file();
    let existing = std::fs::read_to_string(&manifest_path).unwrap_or_default();
    let new_content = upsert_dependency(&existing, &add)?;
    let updated = new_content != existing;
    if updated {
        std::fs::write(&manifest_path, &new_content)
            .map_err(|e| format!("failed to write {}: {}", manifest_path.display(), e))?;
    }
    Ok(AddPlan {
        spec: add,
        manifest_path,
        created_manifest,
        updated,
    })
}

/// Insert or replace a `[dependencies]` entry (pure string edit).
pub fn upsert_dependency(content: &str, add: &AddSpec) -> Result<String, String> {
    let mut lines: Vec<String> = content.lines().map(|l| l.to_string()).collect();
    // Drop trailing empty lines for stable append, re-add at end.
    while lines.last().map(|l| l.trim().is_empty()).unwrap_or(false) {
        lines.pop();
    }

    // Find `[dependencies]` span: start of section header → start of next section.
    let mut dep_start: Option<usize> = None;
    let mut dep_end: Option<usize> = None;
    for (i, line) in lines.iter().enumerate() {
        let t = line.trim();
        if t.starts_with('[') && t.ends_with(']') {
            let name = t[1..t.len() - 1].trim();
            if name == "dependencies" {
                dep_start = Some(i);
                dep_end = Some(i + 1);
            } else if dep_start.is_some() {
                dep_end = Some(i);
                break;
            }
        } else if dep_start.is_some() {
            dep_end = Some(i + 1);
        }
    }

    let mut new_entry: Vec<String> = Vec::new();
    if let Some(c) = add.plan_comment() {
        new_entry.push(c);
    }
    new_entry.push(add.toml_line());

    if let Some(start) = dep_start {
        let end = dep_end.unwrap_or(start + 1);
        // Remove existing entry with the same name (and our plan comment).
        let mut kept: Vec<String> = Vec::with_capacity(lines.len());
        kept.extend_from_slice(&lines[..=start]);
        let mut skip_next_plan_comment = false;
        for line in &lines[start + 1..end.min(lines.len())] {
            let t = line.trim();
            let is_target = t.starts_with(&format!("{} =", add.name))
                || t.starts_with(&format!("{}=", add.name));
            let is_plan_comment =
                t.starts_with('#') && t.contains(&add.name) && t.contains("host-op");
            if is_target {
                // Drop a plan comment we already kept for this name.
                if skip_next_plan_comment {
                    kept.pop();
                    skip_next_plan_comment = false;
                }
                continue;
            }
            if is_plan_comment {
                skip_next_plan_comment = true;
                // Keep for now; dropped if the following line is the target entry.
                kept.push(line.clone());
                continue;
            }
            skip_next_plan_comment = false;
            kept.push(line.clone());
        }
        // Trailing plan comment with no entry still removed if name matches.
        if skip_next_plan_comment {
            kept.pop();
        }
        let insert_at = kept.len();
        kept.extend_from_slice(&lines[end.min(lines.len())..]);
        let mut out: Vec<String> = Vec::with_capacity(kept.len() + new_entry.len() + 1);
        out.extend_from_slice(&kept[..insert_at]);
        out.extend(new_entry);
        out.extend_from_slice(&kept[insert_at..]);
        out.push(String::new());
        return Ok(out.join("\n"));
    }

    // No [dependencies] section: create one.
    if !lines.is_empty() {
        lines.push(String::new());
    }
    lines.push("[dependencies]".to_string());
    lines.extend(new_entry);
    lines.push(String::new());
    Ok(lines.join("\n"))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    fn tmp_root(tag: &str) -> PathBuf {
        let p = std::env::temp_dir().join(format!("mailang_deps_{}_{}", tag, std::process::id()));
        let _ = fs::remove_dir_all(&p);
        fs::create_dir_all(&p).unwrap();
        p
    }

    #[test]
    fn parse_basic_manifest() {
        let src = r#"
# comment
[package]
name = "demo"
version = "0.1.0"

[dependencies]
json = "libs/json"
utils = "./local/utils"
"#;
        let m = parse_manifest(src);
        assert_eq!(m.name.as_deref(), Some("demo"));
        assert_eq!(m.version.as_deref(), Some("0.1.0"));
        assert_eq!(m.dependencies.len(), 2);
        assert_eq!(m.dependencies[0].name, "json");
        assert_eq!(m.dependencies[0].path, "libs/json");
        assert_eq!(m.dependencies[0].source, DepSource::Path);
        assert_eq!(m.dependencies[1].name, "utils");
        assert_eq!(m.dependencies[1].path, "./local/utils");
    }

    #[test]
    fn parse_table_deps() {
        let src = r#"
[dependencies]
utils = { path = "./local/utils" }
cool = { github = "org/repo" }
other = { git = "https://example.com/x.git" }
"#;
        let m = parse_manifest(src);
        assert_eq!(m.dependencies.len(), 3);
        assert_eq!(m.dependencies[0].source, DepSource::Path);
        assert_eq!(m.dependencies[0].path, "./local/utils");
        assert_eq!(
            m.dependencies[1].source,
            DepSource::Github("org/repo".into())
        );
        assert!(m.dependencies[1].path.is_empty());
        assert_eq!(
            m.dependencies[2].source,
            DepSource::Git("https://example.com/x.git".into())
        );
    }

    #[test]
    fn semver_lite_parse_and_cmp() {
        assert_eq!(parse_semver_lite("0.2.0"), Some((0, 2, 0)));
        assert_eq!(parse_semver_lite("1.2"), Some((1, 2, 0)));
        assert_eq!(parse_semver_lite("3"), Some((3, 0, 0)));
        assert!(semver_lite_eq("1.2.0", "1.2"));
        assert!(!semver_lite_eq("1.2.0", "1.3.0"));
        assert_eq!(semver_lite_cmp("0.2.0", "0.10.0"), -1);
        assert_eq!(semver_lite_cmp("1.0.0", "1.0.0"), 0);
        assert_eq!(semver_lite_cmp("2.0.0", "1.9.9"), 1);
    }

    #[test]
    fn content_hash_stable() {
        assert_eq!(content_hash(b"abc"), content_hash(b"abc"));
        assert_ne!(content_hash(b"abc"), content_hash(b"abd"));
        assert_eq!(content_hash(b"").len(), 16);
    }

    #[test]
    fn lockfile_roundtrip() {
        let entries = vec![
            LockEntry {
                name: "json".into(),
                version: "0.2.0".into(),
                path: "libs/json".into(),
                hash: "aabbccdd00112233".into(),
            },
            LockEntry {
                name: "utils".into(),
                version: String::new(),
                path: "vendor/utils".into(),
                hash: String::new(),
            },
        ];
        let text = format_lockfile(&entries);
        let parsed = parse_lockfile(&text);
        assert_eq!(parsed.len(), 2);
        assert_eq!(parsed[0].name, "json");
        assert_eq!(parsed[0].version, "0.2.0");
        assert_eq!(parsed[1].version, "");
        assert_eq!(parsed[1].hash, "");
    }

    #[test]
    fn find_root_walks_up() {
        let tmp = tmp_root("root");
        let nested = tmp.join("a").join("b");
        fs::create_dir_all(&nested).unwrap();
        fs::write(tmp.join("mailang.toml"), "[package]\nname = \"t\"\n").unwrap();
        assert_eq!(find_project_root(&nested), Some(tmp.clone()));
        let _ = fs::remove_dir_all(&tmp);
    }

    #[test]
    fn resolve_named_dep_dir() {
        let tmp = tmp_root("res");
        let dep_dir = tmp.join("libs").join("json");
        fs::create_dir_all(&dep_dir).unwrap();
        fs::write(
            tmp.join("mailang.toml"),
            "[dependencies]\njson = \"libs/json\"\n",
        )
        .unwrap();
        fs::write(
            dep_dir.join("mailib.ini"),
            "[module]\nname = json\nversion = 0.2.0\nentry = lib.mai\n",
        )
        .unwrap();
        fs::write(dep_dir.join("lib.mai"), "fn id(x) { return x }\n").unwrap();

        let resolved = resolve_dependencies(&tmp);
        assert_eq!(resolved.len(), 1);
        assert_eq!(resolved[0].name, "json");
        assert_eq!(resolved[0].version, "0.2.0");
        assert!(
            resolved[0].path.ends_with("json/lib.mai")
                || resolved[0].path.ends_with("json\\lib.mai")
        );
        assert!(!resolved[0].hash.is_empty());

        let map = dependency_map(&tmp);
        assert!(map.contains_key("json"));
        let _ = fs::remove_dir_all(&tmp);
    }

    #[test]
    fn missing_dep_listed_with_raw_path() {
        let tmp = tmp_root("miss");
        fs::create_dir_all(&tmp).unwrap();
        fs::write(
            tmp.join("mailang.toml"),
            "[dependencies]\nghost = \"nope/ghost\"\n",
        )
        .unwrap();
        let resolved = resolve_dependencies(&tmp);
        assert_eq!(resolved.len(), 1);
        assert_eq!(resolved[0].name, "ghost");
        assert!(resolved[0].version.is_empty());
        let _ = fs::remove_dir_all(&tmp);
    }

    #[test]
    fn path_dep_writes_lock_and_reuses() {
        let tmp = tmp_root("lock1");
        let dep_dir = tmp.join("libs").join("json");
        fs::create_dir_all(&dep_dir).unwrap();
        fs::write(
            tmp.join("mailang.toml"),
            "[dependencies]\njson = \"libs/json\"\n",
        )
        .unwrap();
        fs::write(
            dep_dir.join("mailib.ini"),
            "[module]\nname = json\nversion = 0.2.0\n",
        )
        .unwrap();
        fs::write(dep_dir.join("lib.mai"), "fn id(x) { return x }\n").unwrap();

        let (_, outcome) = resolve_dependencies_locked(&tmp);
        assert_eq!(outcome, LockOutcome::Created);
        assert!(tmp.join("mailang.lock").is_file());

        let (_, outcome2) = resolve_dependencies_locked(&tmp);
        assert_eq!(outcome2, LockOutcome::Matched);

        // Change lib.mai → hash mismatch → refresh
        fs::write(dep_dir.join("lib.mai"), "fn id(x) { return 0 }\n").unwrap();
        let (_, outcome3) = resolve_dependencies_locked(&tmp);
        assert_eq!(outcome3, LockOutcome::Refreshed);

        let report = verify_lock(&tmp, &resolve_dependencies_raw(&tmp));
        assert!(report.is_healthy(), "{:?}", report.entries);
        let _ = fs::remove_dir_all(&tmp);
    }

    #[test]
    fn lock_verify_detects_mismatch() {
        let tmp = tmp_root("lock2");
        let dep_dir = tmp.join("libs").join("json");
        fs::create_dir_all(&dep_dir).unwrap();
        fs::write(
            tmp.join("mailang.toml"),
            "[dependencies]\njson = \"libs/json\"\n",
        )
        .unwrap();
        fs::write(dep_dir.join("mailib.ini"), "[module]\nversion = 0.2.0\n").unwrap();
        fs::write(dep_dir.join("lib.mai"), "fn id(x) { return x }\n").unwrap();
        let resolved = resolve_dependencies_raw(&tmp);
        let _ = ensure_lock(&tmp, &resolved);

        // Corrupt lock hash
        let lock_text = fs::read_to_string(tmp.join("mailang.lock")).unwrap();
        let bad = lock_text.replace(&resolved[0].hash, "0000000000000000");
        fs::write(tmp.join("mailang.lock"), bad).unwrap();

        let report = verify_lock(&tmp, &resolve_dependencies_raw(&tmp));
        assert!(!report.is_healthy());
        assert_eq!(report.entries[0].status, EntryLockStatus::HashMismatch);
        let _ = fs::remove_dir_all(&tmp);
    }

    #[test]
    fn github_dep_resolves_from_vendor() {
        let tmp = tmp_root("vendor");
        let pkg = tmp.join("vendor").join("cool");
        fs::create_dir_all(&pkg).unwrap();
        fs::write(
            tmp.join("mailang.toml"),
            "[dependencies]\ncool = { github = \"org/cool\" }\n",
        )
        .unwrap();
        fs::write(pkg.join("mailib.ini"), "[module]\nversion = 1.0.0\n").unwrap();
        fs::write(pkg.join("lib.mai"), "fn hi() { return 1 }\n").unwrap();

        let resolved = resolve_dependencies_raw(&tmp);
        assert_eq!(resolved.len(), 1);
        assert_eq!(resolved[0].name, "cool");
        assert_eq!(resolved[0].version, "1.0.0");
        assert!(!resolved[0].hash.is_empty());
        let _ = fs::remove_dir_all(&tmp);
    }

    #[test]
    fn git_dep_unresolved_lists_vendor_path() {
        let tmp = tmp_root("gitmiss");
        fs::create_dir_all(&tmp).unwrap();
        fs::write(
            tmp.join("mailang.toml"),
            "[dependencies]\nremote = { git = \"https://example.com/a/b.git\" }\n",
        )
        .unwrap();
        let resolved = resolve_dependencies_raw(&tmp);
        assert_eq!(resolved.len(), 1);
        assert!(resolved[0].hash.is_empty());
        assert!(
            resolved[0].path.ends_with("vendor/remote")
                || resolved[0].path.ends_with("vendor\\remote")
        );
        let _ = fs::remove_dir_all(&tmp);
    }

    #[test]
    fn parse_add_spec_forms() {
        let a = parse_add_spec("json=libs/json").unwrap();
        assert_eq!(a.name, "json");
        assert_eq!(a.source, DepSource::Path);
        assert_eq!(a.toml_line(), "json = \"libs/json\"");

        let b = parse_add_spec("cool=github:org/repo").unwrap();
        assert_eq!(b.name, "cool");
        assert_eq!(b.source, DepSource::Github("org/repo".into()));
        assert!(b.plan_comment().unwrap().contains("host-op"));

        let c = parse_add_spec("github:acme/tools").unwrap();
        assert_eq!(c.name, "tools");
        assert_eq!(c.source, DepSource::Github("acme/tools".into()));

        let d = parse_add_spec("acme/widget").unwrap();
        assert_eq!(d.name, "widget");
        assert_eq!(d.source, DepSource::Github("acme/widget".into()));

        let e = parse_add_spec("foo=git:https://example.com/x.git").unwrap();
        assert_eq!(e.name, "foo");
        assert_eq!(e.source, DepSource::Git("https://example.com/x.git".into()));

        let f = parse_add_spec("./local/utils").unwrap();
        assert_eq!(f.name, "utils");
        assert_eq!(f.source, DepSource::Path);
    }

    #[test]
    fn plan_add_writes_manifest() {
        let tmp = tmp_root("add");
        let plan = plan_add(&tmp, "cool=github:org/repo").unwrap();
        assert!(plan.created_manifest);
        assert!(plan.updated);
        let text = fs::read_to_string(tmp.join("mailang.toml")).unwrap();
        assert!(text.contains("[dependencies]"));
        assert!(text.contains("cool = { github = \"org/repo\" }"));
        assert!(text.contains("host-op"));

        // Second add keeps first and adds path dep
        let plan2 = plan_add(&tmp, "utils=./local/utils").unwrap();
        assert!(!plan2.created_manifest);
        let text2 = fs::read_to_string(tmp.join("mailang.toml")).unwrap();
        assert!(text2.contains("cool = { github = \"org/repo\" }"));
        assert!(text2.contains("utils = \"./local/utils\""));

        // Replace same name
        let _ = plan_add(&tmp, "cool=./other/cool").unwrap();
        let text3 = fs::read_to_string(tmp.join("mailang.toml")).unwrap();
        assert!(text3.contains("cool = \"./other/cool\""));
        assert!(!text3.contains("cool = { github"));
        let _ = fs::remove_dir_all(&tmp);
    }

    #[test]
    fn table_path_dep_still_works() {
        let tmp = tmp_root("tablepath");
        let dep_dir = tmp.join("local").join("utils");
        fs::create_dir_all(&dep_dir).unwrap();
        fs::write(
            tmp.join("mailang.toml"),
            "[dependencies]\nutils = { path = \"./local/utils\" }\n",
        )
        .unwrap();
        fs::write(dep_dir.join("mailib.ini"), "[module]\nversion = 1.2.3\n").unwrap();
        fs::write(dep_dir.join("lib.mai"), "fn id(x) { return x }\n").unwrap();
        let resolved = resolve_dependencies(&tmp);
        assert_eq!(resolved.len(), 1);
        assert_eq!(resolved[0].version, "1.2.3");
        let _ = fs::remove_dir_all(&tmp);
    }
}
