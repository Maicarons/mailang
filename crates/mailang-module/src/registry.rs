//! Filesystem-first package registry (crates.io-style, offline, HTTP-friendly).
//!
//! Layout under `<registry-root>/`:
//!
//! ```text
//! index/<name-prefix>/<name>     # JSON lines of IndexEntry
//! pkgs/<name>/<vers>/            # published package directory copy
//! pkgs/<name>/<name>-<vers>.mpkg # length-prefixed bundle (HTTP single-file fetch)
//! ```
//!
//! Index entry fields: `{name, vers, deps, cksum, yanked, description}`.
//! Content hash (`cksum`) is FNV-1a 64 of the packed `.mpkg` bytes.

use crate::deps::{content_hash, parse_semver_lite, semver_lite_cmp};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

// ---------------------------------------------------------------------------
// Errors
// ---------------------------------------------------------------------------

#[derive(Debug, Clone)]
pub enum RegistryError {
    Io(String),
    NotFound(String),
    AlreadyPublished(String),
    Yanked(String),
    Invalid(String),
    Http(String),
}

impl std::fmt::Display for RegistryError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            RegistryError::Io(m) => write!(f, "IO error: {}", m),
            RegistryError::NotFound(m) => write!(f, "not found: {}", m),
            RegistryError::AlreadyPublished(m) => write!(f, "already published: {}", m),
            RegistryError::Yanked(m) => write!(f, "yanked: {}", m),
            RegistryError::Invalid(m) => write!(f, "invalid: {}", m),
            RegistryError::Http(m) => write!(f, "http error: {}", m),
        }
    }
}

impl std::error::Error for RegistryError {}

type Result<T> = std::result::Result<T, RegistryError>;

fn io_err(e: std::io::Error) -> RegistryError {
    RegistryError::Io(e.to_string())
}

// ---------------------------------------------------------------------------
// Index
// ---------------------------------------------------------------------------

/// One published version record (JSON line in the package index).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct IndexEntry {
    pub name: String,
    pub vers: String,
    #[serde(default)]
    pub deps: Vec<String>,
    pub cksum: String,
    #[serde(default)]
    pub yanked: bool,
    #[serde(default)]
    pub description: String,
}

/// Metadata used when publishing a package directory.
#[derive(Debug, Clone)]
pub struct PublishMeta {
    pub name: String,
    pub version: String,
    pub description: String,
    pub deps: Vec<String>,
}

/// Result of a successful publish.
#[derive(Debug, Clone)]
pub struct PublishResult {
    pub entry: IndexEntry,
    pub pkg_dir: PathBuf,
    pub mpkg_path: PathBuf,
}

/// Result of a successful install.
#[derive(Debug, Clone)]
pub struct InstallResult {
    pub entry: IndexEntry,
    pub dest: PathBuf,
    pub manifest_path: Option<PathBuf>,
}

/// crates.io-style name prefix under `index/`.
///
/// - 1 char → `1/<name>`
/// - 2 chars → `2/<name>`
/// - 3 chars → `3/<first>/<name>`
/// - 4+ → `<first two>/<name>`
pub fn name_prefix(name: &str) -> String {
    let chars: Vec<char> = name.chars().collect();
    match chars.len() {
        0 => String::new(),
        1 => format!("1/{}", name),
        2 => format!("2/{}", name),
        3 => format!("3/{}/{}", chars[0], name),
        _ => format!(
            "{}/{}",
            &name[..name
                .char_indices()
                .nth(2)
                .map(|(i, _)| i)
                .unwrap_or(name.len())],
            name
        ),
    }
}

/// Index file path for a package name (filesystem registry).
pub fn index_file_path(root: &Path, name: &str) -> PathBuf {
    root.join("index").join(name_prefix(name)).join(name)
}

/// Package directory for name/vers (filesystem registry).
pub fn pkg_dir_path(root: &Path, name: &str, vers: &str) -> PathBuf {
    root.join("pkgs").join(name).join(vers)
}

/// `.mpkg` bundle path for name/vers.
pub fn mpkg_path(root: &Path, name: &str, vers: &str) -> PathBuf {
    root.join("pkgs")
        .join(name)
        .join(format!("{}-{}.mpkg", name, vers))
}

/// Serialize one index entry as a JSON line (no trailing newline).
pub fn format_index_line(entry: &IndexEntry) -> String {
    serde_json::to_string(entry).unwrap_or_default()
}

/// Parse an index file body (JSON lines; blank lines skipped).
pub fn parse_index(content: &str) -> Vec<IndexEntry> {
    let mut out = Vec::new();
    for line in content.lines() {
        let line = line.trim();
        if line.is_empty() {
            continue;
        }
        if let Ok(e) = serde_json::from_str::<IndexEntry>(line) {
            out.push(e);
        }
    }
    out
}

/// Serialize a list of entries as JSON lines (one object per line).
pub fn format_index(entries: &[IndexEntry]) -> String {
    let mut out = String::new();
    for e in entries {
        out.push_str(&format_index_line(e));
        out.push('\n');
    }
    out
}

// ---------------------------------------------------------------------------
// Bundle format (.mpkg)
// ---------------------------------------------------------------------------

const MPKG_MAGIC: &[u8] = b"MAILANGPKG1";

/// Pack `files` (relative path → bytes) into a simple length-prefixed container.
///
/// Layout:
/// ```text
/// magic "MAILANGPKG1"
/// u32 LE entry_count
/// per entry:
///   u32 LE name_len | name bytes
///   u64 LE data_len | data bytes
/// ```
pub fn pack_mpkg(files: &BTreeMap<String, Vec<u8>>) -> Vec<u8> {
    let mut out = Vec::new();
    out.extend_from_slice(MPKG_MAGIC);
    out.extend_from_slice(&(files.len() as u32).to_le_bytes());
    for (name, data) in files {
        let nb = name.as_bytes();
        out.extend_from_slice(&(nb.len() as u32).to_le_bytes());
        out.extend_from_slice(nb);
        out.extend_from_slice(&(data.len() as u64).to_le_bytes());
        out.extend_from_slice(data);
    }
    out
}

/// Unpack a `.mpkg` container into relative path → bytes.
pub fn unpack_mpkg(bytes: &[u8]) -> Result<BTreeMap<String, Vec<u8>>> {
    if bytes.len() < MPKG_MAGIC.len() + 4 || &bytes[..MPKG_MAGIC.len()] != MPKG_MAGIC {
        return Err(RegistryError::Invalid("bad .mpkg magic".into()));
    }
    let mut pos = MPKG_MAGIC.len();
    let count = read_u32(bytes, &mut pos)? as usize;
    let mut out = BTreeMap::new();
    for _ in 0..count {
        let nlen = read_u32(bytes, &mut pos)? as usize;
        if pos + nlen > bytes.len() {
            return Err(RegistryError::Invalid("truncated entry name".into()));
        }
        let name = String::from_utf8(bytes[pos..pos + nlen].to_vec())
            .map_err(|_| RegistryError::Invalid("non-utf8 entry name".into()))?;
        pos += nlen;
        let dlen = read_u64(bytes, &mut pos)? as usize;
        if pos + dlen > bytes.len() {
            return Err(RegistryError::Invalid("truncated entry data".into()));
        }
        let data = bytes[pos..pos + dlen].to_vec();
        pos += dlen;
        out.insert(name, data);
    }
    Ok(out)
}

fn read_u32(bytes: &[u8], pos: &mut usize) -> Result<u32> {
    if *pos + 4 > bytes.len() {
        return Err(RegistryError::Invalid("truncated u32".into()));
    }
    let v = u32::from_le_bytes(bytes[*pos..*pos + 4].try_into().unwrap());
    *pos += 4;
    Ok(v)
}

fn read_u64(bytes: &[u8], pos: &mut usize) -> Result<u64> {
    if *pos + 8 > bytes.len() {
        return Err(RegistryError::Invalid("truncated u64".into()));
    }
    let v = u64::from_le_bytes(bytes[*pos..*pos + 8].try_into().unwrap());
    *pos += 8;
    Ok(v)
}

// ---------------------------------------------------------------------------
// Registry source (filesystem path or http(s) base URL)
// ---------------------------------------------------------------------------

/// A registry location: local directory or `http(s)://` base URL.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RegistrySource {
    Fs(PathBuf),
    Http(String),
}

impl RegistrySource {
    /// Parse a registry argument. Accepts URL or filesystem path.
    pub fn parse(s: &str) -> Self {
        let s = s.trim();
        if s.starts_with("http://") || s.starts_with("https://") {
            RegistrySource::Http(s.trim_end_matches('/').to_string())
        } else {
            RegistrySource::Fs(PathBuf::from(s))
        }
    }

    pub fn is_http(&self) -> bool {
        matches!(self, RegistrySource::Http(_))
    }

    /// Resolve default: `MAILANG_REGISTRY` → `./.mailang-registry` → `~/.mailang/registry`.
    pub fn default_source() -> Self {
        if let Ok(v) = std::env::var("MAILANG_REGISTRY") {
            let v = v.trim().to_string();
            if !v.is_empty() {
                return Self::parse(&v);
            }
        }
        let local = PathBuf::from(".mailang-registry");
        if local.is_dir() {
            return RegistrySource::Fs(local);
        }
        if let Some(home) = crate::deps::home_dir() {
            return RegistrySource::Fs(home.join(".mailang").join("registry"));
        }
        RegistrySource::Fs(PathBuf::from(".mailang-registry"))
    }

    /// Explicit path or default.
    pub fn resolve(explicit: Option<&str>) -> Self {
        match explicit {
            Some(s) if !s.trim().is_empty() => Self::parse(s),
            _ => Self::default_source(),
        }
    }

    /// Human-readable display.
    pub fn display(&self) -> String {
        match self {
            RegistrySource::Fs(p) => p.display().to_string(),
            RegistrySource::Http(u) => u.clone(),
        }
    }

    /// Create empty registry skeleton (filesystem only).
    pub fn init(&self) -> Result<PathBuf> {
        let RegistrySource::Fs(root) = self else {
            return Err(RegistryError::Invalid(
                "registry init requires a filesystem path".into(),
            ));
        };
        std::fs::create_dir_all(root.join("index")).map_err(io_err)?;
        std::fs::create_dir_all(root.join("pkgs")).map_err(io_err)?;
        let cfg = root.join("registry.toml");
        if !cfg.is_file() {
            std::fs::write(
                &cfg,
                "# mailang registry\nformat = 1\n# host this directory as static files for http(s)\n",
            )
            .map_err(io_err)?;
        }
        Ok(root.clone())
    }

    fn url_join(&self, rel: &str) -> String {
        match self {
            RegistrySource::Http(base) => format!("{}/{}", base.trim_end_matches('/'), rel),
            RegistrySource::Fs(_) => rel.to_string(),
        }
    }

    /// Read raw bytes of a registry-relative path (file or URL path).
    pub fn read_bytes(&self, rel: &str) -> Result<Vec<u8>> {
        match self {
            RegistrySource::Fs(root) => {
                let p = root.join(rel.replace('/', std::path::MAIN_SEPARATOR_STR));
                std::fs::read(&p).map_err(|e| {
                    if e.kind() == std::io::ErrorKind::NotFound {
                        RegistryError::NotFound(p.display().to_string())
                    } else {
                        io_err(e)
                    }
                })
            }
            RegistrySource::Http(_) => curl_bytes(&self.url_join(rel)),
        }
    }

    /// Read text of a registry-relative path.
    pub fn read_string(&self, rel: &str) -> Result<String> {
        let b = self.read_bytes(rel)?;
        String::from_utf8(b).map_err(|_| RegistryError::Invalid(format!("non-utf8: {}", rel)))
    }

    /// Write bytes to a filesystem registry relative path. HTTP is read-only.
    pub fn write_bytes(&self, rel: &str, data: &[u8]) -> Result<()> {
        match self {
            RegistrySource::Fs(root) => {
                let p = root.join(rel.replace('/', std::path::MAIN_SEPARATOR_STR));
                if let Some(parent) = p.parent() {
                    std::fs::create_dir_all(parent).map_err(io_err)?;
                }
                std::fs::write(&p, data).map_err(io_err)
            }
            RegistrySource::Http(_) => Err(RegistryError::Invalid(
                "cannot write to http registry (host the directory as static files)".into(),
            )),
        }
    }

    /// Copy a local directory into a filesystem registry relative path.
    pub fn write_dir(&self, rel: &str, src: &Path) -> Result<PathBuf> {
        let RegistrySource::Fs(root) = self else {
            return Err(RegistryError::Invalid(
                "directory write requires a filesystem registry".into(),
            ));
        };
        let dest = root.join(rel.replace('/', std::path::MAIN_SEPARATOR_STR));
        copy_dir(src, &dest)?;
        Ok(dest)
    }
}

/// Fetch `url` via `curl -fsSL` (no HTTP crate). Returns body bytes.
fn curl_bytes(url: &str) -> Result<Vec<u8>> {
    let out = std::process::Command::new("curl")
        .args(["-fsSL", url])
        .output()
        .map_err(|e| RegistryError::Http(format!("failed to run curl: {}", e)))?;
    if !out.status.success() {
        let err = String::from_utf8_lossy(&out.stderr).trim().to_string();
        return Err(RegistryError::Http(format!(
            "curl {} failed: {}",
            url,
            if err.is_empty() {
                out.status.to_string()
            } else {
                err
            }
        )));
    }
    Ok(out.stdout)
}

// ---------------------------------------------------------------------------
// Index I/O
// ---------------------------------------------------------------------------

/// Relative index path for a package name.
pub fn index_rel(name: &str) -> String {
    format!("index/{}/{}", name_prefix(name), name)
}

/// Read all index entries for `name` (empty vec if package unknown).
pub fn read_index(src: &RegistrySource, name: &str) -> Result<Vec<IndexEntry>> {
    match src.read_string(&index_rel(name)) {
        Ok(text) => Ok(parse_index(&text)),
        Err(RegistryError::NotFound(_)) => Ok(Vec::new()),
        Err(e) => Err(e),
    }
}

/// Write the full index line set for `name` (filesystem only).
pub fn write_index(src: &RegistrySource, name: &str, entries: &[IndexEntry]) -> Result<()> {
    src.write_bytes(&index_rel(name), format_index(entries).as_bytes())
}

fn semver_order(a: &str, b: &str) -> std::cmp::Ordering {
    match semver_lite_cmp(a, b) {
        -1 => std::cmp::Ordering::Less,
        0 => std::cmp::Ordering::Equal,
        _ => std::cmp::Ordering::Greater,
    }
}

/// Append or replace the entry for `(name, vers)` in the index.
///
/// When `replace` is false and the version exists, returns
/// `RegistryError::AlreadyPublished`.
pub fn upsert_index_entry(
    src: &RegistrySource,
    entry: &IndexEntry,
    allow_republish: bool,
) -> Result<()> {
    let mut entries = read_index(src, &entry.name)?;
    if let Some(existing) = entries.iter().find(|e| e.vers == entry.vers) {
        if !allow_republish {
            return Err(RegistryError::AlreadyPublished(format!(
                "{}@{} (use --allow-republish to overwrite)",
                entry.name, entry.vers
            )));
        }
        let _ = existing;
        entries.retain(|e| e.vers != entry.vers);
    }
    entries.push(entry.clone());
    entries.sort_by(|a, b| semver_order(&a.vers, &b.vers));
    write_index(src, &entry.name, &entries)
}

/// Mark `(name, vers)` as yanked (or un-yank with `yanked = false`).
pub fn set_yanked(
    src: &RegistrySource,
    name: &str,
    vers: &str,
    yanked: bool,
) -> Result<IndexEntry> {
    let mut entries = read_index(src, name)?;
    if entries.is_empty() {
        return Err(RegistryError::NotFound(format!(
            "package '{}' in registry {}",
            name,
            src.display()
        )));
    }
    let mut found = None;
    for e in entries.iter_mut() {
        if semver_lite_eq_str(&e.vers, vers) {
            e.yanked = yanked;
            found = Some(e.clone());
        }
    }
    let Some(entry) = found else {
        return Err(RegistryError::NotFound(format!("{}@{}", name, vers)));
    };
    write_index(src, name, &entries)?;
    Ok(entry)
}

fn semver_lite_eq_str(a: &str, b: &str) -> bool {
    match (parse_semver_lite(a), parse_semver_lite(b)) {
        (Some(x), Some(y)) => x == y,
        _ => a.trim() == b.trim(),
    }
}

/// Search the registry by scanning `index/**` for name/description matches.
///
/// Filesystem: walk `index/`. HTTP: not supported for full-text walk
/// (returns `Invalid`); use a local mirror or known names via `read_index`.
pub fn search(src: &RegistrySource, query: &str) -> Result<Vec<IndexEntry>> {
    let q = query.trim().to_lowercase();
    match src {
        RegistrySource::Fs(root) => {
            let index_root = root.join("index");
            let mut hits: Vec<IndexEntry> = Vec::new();
            let mut seen: std::collections::HashSet<String> = std::collections::HashSet::new();
            walk_index_files(&index_root, &mut |path| {
                let Ok(text) = std::fs::read_to_string(path) else {
                    return;
                };
                for e in parse_index(&text) {
                    let key = format!("{}@{}", e.name, e.vers);
                    if !seen.insert(key) {
                        continue;
                    }
                    if q.is_empty()
                        || e.name.to_lowercase().contains(&q)
                        || e.description.to_lowercase().contains(&q)
                    {
                        hits.push(e);
                    }
                }
            });
            hits.sort_by(|a, b| a.name.cmp(&b.name).then(semver_order(&a.vers, &b.vers)));
            Ok(hits)
        }
        RegistrySource::Http(_) => Err(RegistryError::Invalid(
            "search requires a filesystem registry (or mirror index/ locally)".into(),
        )),
    }
}

fn walk_index_files(dir: &Path, f: &mut dyn FnMut(&Path)) {
    let Ok(rd) = std::fs::read_dir(dir) else {
        return;
    };
    for entry in rd.flatten() {
        let path = entry.path();
        if path.is_dir() {
            walk_index_files(&path, f);
        } else if path.is_file() {
            f(&path);
        }
    }
}

// ---------------------------------------------------------------------------
// Resolve
// ---------------------------------------------------------------------------

/// Resolve `name` or `name@ver` against the index.
///
/// - bare name → latest non-yanked version
/// - `name@ver` → exact SEMVER-lite match
/// - `allow_yanked` permits selecting a yanked version
pub fn resolve_version(
    src: &RegistrySource,
    name: &str,
    version: Option<&str>,
    allow_yanked: bool,
) -> Result<IndexEntry> {
    let entries = read_index(src, name)?;
    if entries.is_empty() {
        return Err(RegistryError::NotFound(format!(
            "package '{}' in registry {}",
            name,
            src.display()
        )));
    }

    match version {
        Some(vers) => {
            let matches: Vec<&IndexEntry> = entries
                .iter()
                .filter(|e| semver_lite_eq_str(&e.vers, vers))
                .collect();
            let Some(entry) = matches.into_iter().next() else {
                return Err(RegistryError::NotFound(format!("{}@{}", name, vers)));
            };
            if entry.yanked && !allow_yanked {
                return Err(RegistryError::Yanked(format!(
                    "{}@{} (use --allow-yanked to install)",
                    name, entry.vers
                )));
            }
            Ok(entry.clone())
        }
        None => {
            let mut candidates: Vec<&IndexEntry> = entries
                .iter()
                .filter(|e| allow_yanked || !e.yanked)
                .collect();
            candidates.sort_by(|a, b| semver_order(&b.vers, &a.vers));
            if let Some(entry) = candidates.into_iter().next() {
                return Ok(entry.clone());
            }
            // Everything is yanked (or the list is empty).
            if entries.iter().any(|e| e.yanked) {
                return Err(RegistryError::Yanked(format!(
                    "all versions of '{}' are yanked (use --allow-yanked)",
                    name
                )));
            }
            Err(RegistryError::NotFound(format!(
                "no installable version of '{}'",
                name
            )))
        }
    }
}

/// Parse `name` or `name@ver` (or `name@1.2`).
pub fn parse_pkg_spec(spec: &str) -> Result<(String, Option<String>)> {
    let spec = spec.trim();
    if spec.is_empty() {
        return Err(RegistryError::Invalid("empty package spec".into()));
    }
    if let Some((name, ver)) = spec.split_once('@') {
        let name = name.trim();
        let ver = ver.trim();
        if name.is_empty() || ver.is_empty() {
            return Err(RegistryError::Invalid(format!(
                "invalid package spec '{}': expected name[@ver]",
                spec
            )));
        }
        if parse_semver_lite(ver).is_none() {
            return Err(RegistryError::Invalid(format!(
                "invalid version '{}' in '{}'",
                ver, spec
            )));
        }
        Ok((name.to_string(), Some(ver.to_string())))
    } else {
        Ok((spec.to_string(), None))
    }
}

// ---------------------------------------------------------------------------
// File collection / copy
// ---------------------------------------------------------------------------

/// Files included when publishing a package directory.
const PUBLISH_FILES: &[&str] = &[
    "mailang.toml",
    "lib.mai",
    "mailib.ini",
    "README",
    "README.md",
];

fn collect_pkg_files(src: &Path) -> Result<BTreeMap<String, Vec<u8>>> {
    let mut files = BTreeMap::new();
    for name in PUBLISH_FILES {
        let p = src.join(name);
        if p.is_file() {
            files.insert((*name).to_string(), std::fs::read(&p).map_err(io_err)?);
        }
    }
    // Also include extra top-level *.mai files (small, debuggable).
    if let Ok(rd) = std::fs::read_dir(src) {
        for ent in rd.flatten() {
            let path = ent.path();
            if path.is_file() {
                let fname = path.file_name().map(|s| s.to_string_lossy().to_string());
                if let Some(fname) = fname {
                    if fname.ends_with(".mai") && !files.contains_key(&fname) {
                        files.insert(fname, std::fs::read(&path).map_err(io_err)?);
                    }
                }
            }
        }
    }
    if !files.contains_key("lib.mai") && !files.keys().any(|k| k.ends_with(".mai")) {
        return Err(RegistryError::Invalid(format!(
            "package directory {} has no .mai entry file",
            src.display()
        )));
    }
    if !files.contains_key("mailang.toml") {
        return Err(RegistryError::Invalid(format!(
            "package directory {} is missing mailang.toml",
            src.display()
        )));
    }
    Ok(files)
}

fn copy_dir(src: &Path, dest: &Path) -> Result<()> {
    std::fs::create_dir_all(dest).map_err(io_err)?;
    let rd = std::fs::read_dir(src).map_err(io_err)?;
    for ent in rd.flatten() {
        let path = ent.path();
        let name = ent.file_name();
        let to = dest.join(&name);
        if path.is_dir() {
            // Skip bulky / local dirs.
            let n = name.to_string_lossy().to_string();
            if n == "vendor" || n == "target" || n == ".git" {
                continue;
            }
            copy_dir(&path, &to)?;
        } else if path.is_file() {
            let n = name.to_string_lossy().to_string();
            if n == "mailang.lock" {
                continue;
            }
            std::fs::copy(&path, &to).map_err(io_err)?;
        }
    }
    Ok(())
}

fn write_dir_from_map(dest: &Path, files: &BTreeMap<String, Vec<u8>>) -> Result<()> {
    std::fs::create_dir_all(dest).map_err(io_err)?;
    for (name, data) in files {
        let p = dest.join(name.replace('/', std::path::MAIN_SEPARATOR_STR));
        if let Some(parent) = p.parent() {
            std::fs::create_dir_all(parent).map_err(io_err)?;
        }
        std::fs::write(&p, data).map_err(io_err)?;
    }
    Ok(())
}

// ---------------------------------------------------------------------------
// Publish / install
// ---------------------------------------------------------------------------

/// Publish the package directory at `project` into `src`.
///
/// Copies files into `pkgs/<name>/<vers>/`, writes `pkgs/<name>/<name>-<vers>.mpkg`,
/// and appends an index entry. Refuses when the version exists unless
/// `allow_republish`.
pub fn publish(
    src: &RegistrySource,
    project: &Path,
    allow_republish: bool,
) -> Result<PublishResult> {
    if src.is_http() {
        return Err(RegistryError::Invalid(
            "publish requires a filesystem registry path".into(),
        ));
    }
    let manifest_path = project.join("mailang.toml");
    if !manifest_path.is_file() {
        return Err(RegistryError::Invalid(format!(
            "missing {} (required for publish)",
            manifest_path.display()
        )));
    }
    let meta = load_publish_meta(project)?;
    if meta.name.is_empty() {
        return Err(RegistryError::Invalid(
            "mailang.toml [package] name is required".into(),
        ));
    }
    if meta.version.is_empty() {
        return Err(RegistryError::Invalid(
            "mailang.toml [package] version is required".into(),
        ));
    }
    if parse_semver_lite(&meta.version).is_none() {
        return Err(RegistryError::Invalid(format!(
            "invalid version '{}'",
            meta.version
        )));
    }

    // Refuse duplicate before writing anything.
    if !allow_republish {
        let existing = read_index(src, &meta.name)?;
        if existing
            .iter()
            .any(|e| semver_lite_eq_str(&e.vers, &meta.version))
        {
            return Err(RegistryError::AlreadyPublished(format!(
                "{}@{} (use --allow-republish to overwrite)",
                meta.name, meta.version
            )));
        }
    }

    let files = collect_pkg_files(project)?;
    let bundle = pack_mpkg(&files);
    let cksum = content_hash(&bundle);

    let entry = IndexEntry {
        name: meta.name.clone(),
        vers: meta.version.clone(),
        deps: meta.deps.clone(),
        cksum: cksum.clone(),
        yanked: false,
        description: meta.description.clone(),
    };

    // Write package artifacts.
    let rel_dir = format!("pkgs/{}/{}", entry.name, entry.vers);
    let dest_dir = src.write_dir(&rel_dir, project)?;
    let rel_mpkg = format!("pkgs/{}/{}-{}.mpkg", entry.name, entry.name, entry.vers);
    src.write_bytes(&rel_mpkg, &bundle)?;

    upsert_index_entry(src, &entry, allow_republish)?;

    Ok(PublishResult {
        entry,
        pkg_dir: dest_dir,
        mpkg_path: match src {
            RegistrySource::Fs(root) => {
                root.join(rel_mpkg.replace('/', std::path::MAIN_SEPARATOR_STR))
            }
            RegistrySource::Http(_) => PathBuf::from(rel_mpkg),
        },
    })
}

/// Load publish metadata from `mailang.toml` (and `mailib.ini` fallbacks).
pub fn load_publish_meta(project: &Path) -> Result<PublishMeta> {
    let manifest_text = std::fs::read_to_string(project.join("mailang.toml")).map_err(io_err)?;
    let m = crate::deps::parse_manifest(&manifest_text);
    let mut description = m.description.clone().unwrap_or_default();
    let deps: Vec<String> = m.dependencies.iter().map(|d| d.name.clone()).collect();

    let mut name = m.name.clone().unwrap_or_default();
    let mut version = m.version.clone().unwrap_or_default();

    // mailib.ini fills missing fields.
    if let Ok(ini) = std::fs::read_to_string(project.join("mailib.ini")) {
        for line in ini.lines() {
            let line = line.trim();
            if line.is_empty() || line.starts_with('#') || line.starts_with('[') {
                continue;
            }
            if let Some((k, v)) = line.split_once('=') {
                let v = v.trim().trim_matches('"');
                match k.trim() {
                    "name" if name.is_empty() => name = v.to_string(),
                    "version" if version.is_empty() => version = v.to_string(),
                    "description" if description.is_empty() => description = v.to_string(),
                    _ => {}
                }
            }
        }
    }

    Ok(PublishMeta {
        name,
        version,
        description,
        deps,
    })
}

/// Install `name[@ver]` from `src` into `vendor/<name>` (or `~/.mailang/pkg/<name>`
/// when `global` is true). Optionally rewrites `project/mailang.toml` deps.
pub fn install(
    src: &RegistrySource,
    spec: &str,
    project: Option<&Path>,
    global: bool,
    allow_yanked: bool,
    rewrite_manifest: bool,
) -> Result<InstallResult> {
    let (name, version) = parse_pkg_spec(spec)?;
    let entry = resolve_version(src, &name, version.as_deref(), allow_yanked)?;

    // Materialize package files.
    let files = fetch_pkg_files(src, &entry)?;
    if !files.keys().any(|k| k.ends_with(".mai")) {
        return Err(RegistryError::Invalid(format!(
            "package {}@{} has no .mai files",
            entry.name, entry.vers
        )));
    }

    let dest = if global {
        crate::deps::global_pkg_dir(&entry.name).ok_or_else(|| {
            RegistryError::Io("cannot resolve home directory for global install".into())
        })?
    } else {
        let root = project.ok_or_else(|| {
            RegistryError::Invalid("install to vendor/ needs a project root".into())
        })?;
        crate::deps::vendor_dir(root, &entry.name)
    };

    if dest.exists() {
        std::fs::remove_dir_all(&dest).map_err(io_err)?;
    }
    write_dir_from_map(&dest, &files)?;

    // Write a local lock-friendly marker.
    let meta = format!(
        "[module]\nname = {}\nversion = {}\ndescription = {}\n",
        entry.name,
        entry.vers,
        if entry.description.is_empty() {
            "-"
        } else {
            entry.description.as_str()
        }
    );
    // Do not clobber a real mailib.ini from the package.
    if !files.contains_key("mailib.ini") {
        let _ = std::fs::write(dest.join("mailib.ini"), meta);
    }

    let mut manifest_path = None;
    if rewrite_manifest && !global {
        if let Some(root) = project {
            manifest_path = Some(rewrite_manifest_dep(root, &entry.name, &dest)?);
        }
    }

    Ok(InstallResult {
        entry,
        dest,
        manifest_path,
    })
}

fn fetch_pkg_files(src: &RegistrySource, entry: &IndexEntry) -> Result<BTreeMap<String, Vec<u8>>> {
    // Prefer directory copy on filesystem.
    if let RegistrySource::Fs(root) = src {
        let dir = pkg_dir_path(root, &entry.name, &entry.vers);
        if dir.is_dir() {
            return collect_pkg_files(&dir);
        }
    }
    // Fall back to .mpkg (HTTP or filesystem).
    let rel = format!("pkgs/{}/{}-{}.mpkg", entry.name, entry.name, entry.vers);
    let bytes = src.read_bytes(&rel)?;
    let cksum = content_hash(&bytes);
    if cksum != entry.cksum {
        return Err(RegistryError::Invalid(format!(
            "checksum mismatch for {}@{}: index {} vs package {}",
            entry.name, entry.vers, entry.cksum, cksum
        )));
    }
    unpack_mpkg(&bytes)
}

/// Point `mailang.toml` `[dependencies]` at the installed vendor path.
fn rewrite_manifest_dep(project: &Path, name: &str, dest: &Path) -> Result<PathBuf> {
    let manifest_path = project.join("mailang.toml");
    let content = std::fs::read_to_string(&manifest_path).unwrap_or_default();
    let rel = crate::deps::lock_path_string(project, dest);
    let spec = crate::deps::AddSpec {
        name: name.to_string(),
        source: crate::deps::DepSource::Path,
        path: rel.clone(),
    };
    let comment = format!(
        "# {} = registry install -> {} (do not edit by hand)",
        name, rel
    );
    // Use upsert then ensure our install comment sits above the entry.
    let mut updated =
        crate::deps::upsert_dependency(&content, &spec).map_err(RegistryError::Invalid)?;
    if !updated.contains(&comment) {
        // Insert comment line just before the dep entry if possible.
        let needle = format!("{} = \"{}\"", name, rel.replace('\\', "/"));
        let needle_win = format!("{} = \"{}\"", name, rel);
        let pos = updated.find(&needle).or_else(|| updated.find(&needle_win));
        if let Some(pos) = pos {
            updated.insert_str(pos, &format!("{}\n", comment));
        }
    }
    std::fs::write(&manifest_path, &updated).map_err(io_err)?;
    Ok(manifest_path)
}

/// Convenience: init + return source (CLI `registry init`).
pub fn registry_init(path: &str) -> Result<PathBuf> {
    RegistrySource::parse(path).init()
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    fn tmp(tag: &str) -> PathBuf {
        let p =
            std::env::temp_dir().join(format!("mailang_registry_{}_{}", tag, std::process::id()));
        let _ = std::fs::remove_dir_all(&p);
        std::fs::create_dir_all(&p).unwrap();
        p
    }

    fn make_pkg(dir: &Path, name: &str, vers: &str, desc: &str) {
        std::fs::create_dir_all(dir).unwrap();
        std::fs::write(
            dir.join("mailang.toml"),
            format!(
                "[package]\nname = \"{}\"\nversion = \"{}\"\ndescription = \"{}\"\n",
                name, vers, desc
            ),
        )
        .unwrap();
        std::fs::write(
            dir.join("lib.mai"),
            format!("fn ping() {{ return \"{}\" }}\n", name),
        )
        .unwrap();
        std::fs::write(
            dir.join("mailib.ini"),
            format!(
                "[module]\nname = {}\nversion = {}\nentry = lib.mai\n",
                name, vers
            ),
        )
        .unwrap();
        std::fs::write(dir.join("README"), format!("{} {}\n", name, desc)).unwrap();
    }

    #[test]
    fn name_prefix_layout() {
        assert_eq!(name_prefix("a"), "1/a");
        assert_eq!(name_prefix("ab"), "2/ab");
        assert_eq!(name_prefix("abc"), "3/a/abc");
        assert_eq!(name_prefix("json"), "js/json");
        assert_eq!(name_prefix("serde_json"), "se/serde_json");
    }

    #[test]
    fn index_line_roundtrip() {
        let e = IndexEntry {
            name: "json".into(),
            vers: "0.2.0".into(),
            deps: vec!["utils".into()],
            cksum: "aabbccdd00112233".into(),
            yanked: false,
            description: "JSON helpers".into(),
        };
        let line = format_index_line(&e);
        let parsed = parse_index(&line);
        assert_eq!(parsed, vec![e]);
    }

    #[test]
    fn mpkg_pack_unpack_roundtrip() {
        let mut files = BTreeMap::new();
        files.insert("lib.mai".to_string(), b"fn x() { return 1 }\n".to_vec());
        files.insert(
            "mailang.toml".to_string(),
            b"[package]\nname=\"t\"\n".to_vec(),
        );
        let packed = pack_mpkg(&files);
        let unpacked = unpack_mpkg(&packed).unwrap();
        assert_eq!(unpacked, files);
    }

    #[test]
    fn registry_init_and_index_write_read() {
        let root = tmp("init");
        let src = RegistrySource::Fs(root.clone());
        src.init().unwrap();
        assert!(root.join("index").is_dir());
        assert!(root.join("pkgs").is_dir());

        let e = IndexEntry {
            name: "demo".into(),
            vers: "1.0.0".into(),
            deps: vec![],
            cksum: "0000000000000000".into(),
            yanked: false,
            description: "demo pkg".into(),
        };
        upsert_index_entry(&src, &e, false).unwrap();
        let got = read_index(&src, "demo").unwrap();
        assert_eq!(got.len(), 1);
        assert_eq!(got[0].name, "demo");
        let idx = index_file_path(&root, "demo");
        let idx_s = idx.to_string_lossy().replace('\\', "/");
        assert!(idx_s.ends_with("index/de/demo/demo"), "got {}", idx_s);

        // Duplicate rejected without allow_republish
        let err = upsert_index_entry(&src, &e, false).unwrap_err();
        assert!(matches!(err, RegistryError::AlreadyPublished(_)));

        // allow_republish overwrites
        let mut e2 = e.clone();
        e2.description = "updated".into();
        upsert_index_entry(&src, &e2, true).unwrap();
        let got2 = read_index(&src, "demo").unwrap();
        assert_eq!(got2.len(), 1);
        assert_eq!(got2[0].description, "updated");
        let _ = std::fs::remove_dir_all(&root);
    }

    #[test]
    fn publish_install_roundtrip() {
        let work = tmp("roundtrip");
        let reg = work.join("reg");
        let src = RegistrySource::Fs(reg.clone());
        src.init().unwrap();

        let pkg = work.join("pkg");
        make_pkg(&pkg, "greet", "0.1.0", "hello helpers");

        let pub_res = publish(&src, &pkg, false).unwrap();
        assert_eq!(pub_res.entry.name, "greet");
        assert_eq!(pub_res.entry.vers, "0.1.0");
        assert!(!pub_res.entry.cksum.is_empty());
        assert!(pub_res.pkg_dir.join("lib.mai").is_file());
        assert!(pub_res.mpkg_path.is_file());

        // Second publish same version refused
        assert!(matches!(
            publish(&src, &pkg, false).unwrap_err(),
            RegistryError::AlreadyPublished(_)
        ));

        // Install into a consumer project
        let app = work.join("app");
        std::fs::create_dir_all(&app).unwrap();
        std::fs::write(
            app.join("mailang.toml"),
            "[package]\nname = \"app\"\nversion = \"0.0.1\"\n",
        )
        .unwrap();

        let inst = install(&src, "greet", Some(&app), false, false, true).unwrap();
        assert_eq!(inst.entry.vers, "0.1.0");
        assert!(inst.dest.join("lib.mai").is_file());
        let manifest = std::fs::read_to_string(app.join("mailang.toml")).unwrap();
        assert!(manifest.contains("greet"));
        assert!(manifest.contains("vendor"));

        // Exact version install
        let inst2 = install(&src, "greet@0.1.0", Some(&app), false, false, false).unwrap();
        assert_eq!(inst2.entry.vers, "0.1.0");

        // Missing version
        assert!(matches!(
            install(&src, "greet@9.9.9", Some(&app), false, false, false).unwrap_err(),
            RegistryError::NotFound(_)
        ));

        let _ = std::fs::remove_dir_all(&work);
    }

    #[test]
    fn yank_blocks_install() {
        let work = tmp("yank");
        let reg = work.join("reg");
        let src = RegistrySource::Fs(reg.clone());
        src.init().unwrap();
        let pkg = work.join("pkg");
        make_pkg(&pkg, "tools", "1.2.3", "tooling");
        publish(&src, &pkg, false).unwrap();

        let app = work.join("app");
        std::fs::create_dir_all(&app).unwrap();
        std::fs::write(
            app.join("mailang.toml"),
            "[package]\nname = \"app\"\nversion = \"0.0.1\"\n",
        )
        .unwrap();

        let yanked = set_yanked(&src, "tools", "1.2.3", true).unwrap();
        assert!(yanked.yanked);

        let err = install(&src, "tools", Some(&app), false, false, false).unwrap_err();
        assert!(matches!(err, RegistryError::Yanked(_)));

        // Exact yanked also blocked
        let err2 = install(&src, "tools@1.2.3", Some(&app), false, false, false).unwrap_err();
        assert!(matches!(err2, RegistryError::Yanked(_)));

        // --allow-yanked works
        let ok = install(&src, "tools@1.2.3", Some(&app), false, true, false).unwrap();
        assert_eq!(ok.entry.vers, "1.2.3");

        // Un-yank restores latest resolution
        set_yanked(&src, "tools", "1.2.3", false).unwrap();
        let ok2 = install(&src, "tools", Some(&app), false, false, false).unwrap();
        assert_eq!(ok2.entry.vers, "1.2.3");

        let _ = std::fs::remove_dir_all(&work);
    }

    #[test]
    fn search_by_name_and_description() {
        let work = tmp("search");
        let reg = work.join("reg");
        let src = RegistrySource::Fs(reg.clone());
        src.init().unwrap();

        let a = work.join("a");
        make_pkg(&a, "jsonkit", "0.1.0", "JSON encode and decode");
        publish(&src, &a, false).unwrap();
        let b = work.join("b");
        make_pkg(&b, "strutil", "0.2.0", "string helpers");
        publish(&src, &b, false).unwrap();

        let hits = search(&src, "json").unwrap();
        assert_eq!(hits.len(), 1);
        assert_eq!(hits[0].name, "jsonkit");

        let hits2 = search(&src, "helpers").unwrap();
        assert_eq!(hits2.len(), 1);
        assert_eq!(hits2[0].name, "strutil");

        let hits3 = search(&src, "").unwrap();
        assert_eq!(hits3.len(), 2);

        let hits4 = search(&src, "nope-not-here").unwrap();
        assert!(hits4.is_empty());

        let _ = std::fs::remove_dir_all(&work);
    }

    #[test]
    fn parse_pkg_spec_forms() {
        assert_eq!(parse_pkg_spec("json").unwrap(), ("json".into(), None));
        assert_eq!(
            parse_pkg_spec("json@0.2.0").unwrap(),
            ("json".into(), Some("0.2.0".into()))
        );
        assert!(parse_pkg_spec("").is_err());
        assert!(parse_pkg_spec("json@").is_err());
        assert!(parse_pkg_spec("@1.0.0").is_err());
    }

    #[test]
    fn http_registry_rejects_publish() {
        let src = RegistrySource::Http("https://example.com/reg".into());
        assert!(src.is_http());
        let work = tmp("http_pub");
        make_pkg(&work.join("p"), "x", "1.0.0", "d");
        assert!(matches!(
            publish(&src, &work.join("p"), false).unwrap_err(),
            RegistryError::Invalid(_)
        ));
        let _ = std::fs::remove_dir_all(&work);
    }

    #[test]
    fn checksum_verifies_mpkg() {
        let work = tmp("cksum");
        let reg = work.join("reg");
        let src = RegistrySource::Fs(reg.clone());
        src.init().unwrap();
        let pkg = work.join("pkg");
        make_pkg(&pkg, "hashy", "1.0.0", "hash");
        let res = publish(&src, &pkg, false).unwrap();

        // Tamper with the bundle → install fails checksum (when dir copy is absent).
        // Remove the directory so install uses .mpkg path.
        let _ = std::fs::remove_dir_all(&res.pkg_dir);
        let mut tampered = std::fs::read(&res.mpkg_path).unwrap();
        let last = tampered.len() - 1;
        tampered[last] ^= 0xff;
        std::fs::write(&res.mpkg_path, &tampered).unwrap();

        let app = work.join("app");
        std::fs::create_dir_all(&app).unwrap();
        std::fs::write(
            app.join("mailang.toml"),
            "[package]\nname = \"app\"\nversion = \"0.0.1\"\n",
        )
        .unwrap();
        let err = install(&src, "hashy", Some(&app), false, false, false).unwrap_err();
        assert!(matches!(err, RegistryError::Invalid(_)));
        let _ = std::fs::remove_dir_all(&work);
    }
}
