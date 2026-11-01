//! Reproducible binding dependencies (`docs/research/rust-interop.md`,
//! Build item 3): the bindings' committed `Cargo.lock` files decide
//! every crates.io version. [`seed`] writes the union the generated
//! crate starts from; [`verify`] holds cargo's resulting lock to it.
//!
//! Only the parts of a lockfile this needs are read: each `[[package]]`'s
//! `name`, `version`, `source`, `checksum` and `dependencies`.

use std::collections::{BTreeMap, BTreeSet};

/// One `[[package]]` of a `Cargo.lock`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Package {
    pub name: String,
    pub version: String,
    /// `None` for a path package (the binding itself, a local crate).
    pub source: Option<String>,
    pub checksum: Option<String>,
    /// As written: `name`, `name version` or `name version (source)`.
    pub deps: Vec<String>,
}

/// A parsed `Cargo.lock`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Lock {
    /// The `version = N` format line; 0 when absent (format 1).
    pub format: u32,
    pub packages: Vec<Package>,
}

/// A binding's name and its parsed lock.
pub type BindingLock<'a> = (&'a str, &'a Lock);

/// Parses the `Cargo.lock` `text`.
pub fn parse(text: &str) -> Result<Lock, String> {
    let mut lock = Lock {
        format: 0,
        packages: Vec::new(),
    };
    let mut current: Option<Package> = None;
    let mut in_deps = false;
    for (i, raw) in text.lines().enumerate() {
        let line = raw.trim();
        let bad = || format!("Cargo.lock line {}: cannot read `{line}`", i + 1);
        if in_deps {
            in_deps = deps_line(line, current.as_mut()).ok_or_else(bad)?;
            continue;
        }
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        if line.starts_with('[') {
            lock.packages.extend(current.take());
            if line == "[[package]]" {
                current = Some(empty_package());
            }
            continue;
        }
        let Some((key, value)) = line.split_once('=') else {
            return Err(bad());
        };
        let (key, value) = (key.trim(), value.trim());
        match (key, current.as_mut()) {
            ("version", None) => lock.format = value.parse().map_err(|_| bad())?,
            ("dependencies", Some(_)) => {
                in_deps = value != "[]";
                if value != "[" && value != "[]" {
                    return Err(bad());
                }
            }
            (_, Some(p)) => set_field(p, key, unquote(value).ok_or_else(bad)?),
            (_, None) => {}
        }
    }
    lock.packages.extend(current);
    Ok(lock)
}

fn empty_package() -> Package {
    Package {
        name: String::new(),
        version: String::new(),
        source: None,
        checksum: None,
        deps: Vec::new(),
    }
}

/// One line inside `dependencies = [`; whether the list goes on.
fn deps_line(line: &str, package: Option<&mut Package>) -> Option<bool> {
    if line == "]" {
        return Some(false);
    }
    let dep = unquote(line.strip_suffix(',').unwrap_or(line))?;
    package?.deps.push(dep);
    Some(true)
}

fn set_field(p: &mut Package, key: &str, value: String) {
    match key {
        "name" => p.name = value,
        "version" => p.version = value,
        "source" => p.source = Some(value),
        "checksum" => p.checksum = Some(value),
        _ => {}
    }
}

/// The text of a `"..."` TOML string with no escapes, as cargo writes
/// every lockfile value.
fn unquote(value: &str) -> Option<String> {
    let inner = value.strip_prefix('"')?.strip_suffix('"')?;
    (!inner.contains(['"', '\\'])).then(|| inner.to_string())
}

impl Package {
    /// Whether the lock pins this package: any source but a path (no
    /// source) or git, which [`refuse_git`] refuses. So an alternative
    /// registry (`sparse+...`) is pinned like crates.io.
    fn is_pinned(&self) -> bool {
        self.source.is_some() && !self.is_git()
    }

    fn is_git(&self) -> bool {
        self.source
            .as_deref()
            .is_some_and(|s| s.starts_with("git+"))
    }
}

impl Lock {
    /// `p`'s dependencies by name, each with the versions `self` resolves
    /// it to: the one in the entry, or the only package of that name. A
    /// crate may depend on two versions of one name, renamed.
    fn resolved(&self, p: &Package) -> BTreeMap<String, BTreeSet<String>> {
        let mut out: BTreeMap<String, BTreeSet<String>> = BTreeMap::new();
        for dep in &p.deps {
            let mut parts = dep.split(' ');
            let name = parts.next().unwrap_or_default();
            let version = match parts.next() {
                Some(v) => Some(v.to_string()),
                None => self.only_version(name),
            };
            if let Some(v) = version {
                out.entry(name.to_string()).or_default().insert(v);
            }
        }
        out
    }

    fn only_version(&self, name: &str) -> Option<String> {
        let mut found = self.packages.iter().filter(|p| p.name == name);
        let first = found.next()?;
        found.next().is_none().then(|| first.version.clone())
    }

    /// The entry for the package `p` is: same name, version, and kind
    /// (path or not).
    fn find(&self, p: &Package) -> Option<&Package> {
        self.packages.iter().find(|q| {
            q.name == p.name && q.version == p.version && q.source.is_none() == p.source.is_none()
        })
    }
}

/// Why the bindings' locks cannot build together, or why a build's lock
/// drifted from them.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum LockError {
    /// A binding's lock has a git dependency, which the registry-only
    /// seed and check cannot hold.
    Git { binding: String, package: String },
    /// Two locks pin one crate at different, semver-compatible versions,
    /// which cargo would unify.
    Drift {
        package: String,
        pins: [(String, String); 2],
    },
    /// Two bindings take one crate at one version from different
    /// sources, which the lock checks cannot tell apart yet.
    Sources {
        package: String,
        bindings: [String; 2],
    },
    /// The build's lock differs from the bindings': an entry or an edge
    /// added or changed. `binding` is `None` for an entry none pinned.
    Changed {
        binding: Option<String>,
        detail: String,
    },
}

impl std::fmt::Display for LockError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            LockError::Git { binding, package } => write!(
                f,
                "binding `{binding}`: git dependencies are not supported yet; \
                 use a crates.io version (`{package}`)"
            ),
            LockError::Drift { package, pins } => {
                let [(a, va), (b, vb)] = pins;
                write!(
                    f,
                    "bindings `{a}` and `{b}` pin `{package}` at {va} and {vb}, which cargo \
                     would unify; pin one version in both Cargo.lock files"
                )
            }
            LockError::Sources { package, bindings } => {
                let [a, b] = bindings;
                write!(
                    f,
                    "bindings `{a}` and `{b}` take `{package}` from different sources, \
                     which is not supported yet; use one registry for it"
                )
            }
            LockError::Changed { binding, detail } => {
                let who = binding
                    .as_deref()
                    .map_or("a binding".to_string(), |b| format!("binding `{b}`"));
                write!(
                    f,
                    "{who}: dependencies differ from its Cargo.lock ({detail}); \
                     update and commit the binding's lockfile"
                )
            }
        }
    }
}

/// The text of the lock the generated crate starts from: every pinned
/// package of every binding's lock, and each binding's path packages
/// with their edges, each dependency written with its version so the
/// union stays unambiguous.
pub fn seed(bindings: &[BindingLock]) -> Result<String, LockError> {
    let union = union(bindings)?;
    let format = bindings.iter().map(|(_, l)| l.format).max().unwrap_or(0);
    let mut out = String::from("# Seeded by pythrust from the bindings' Cargo.lock files.\n");
    if format > 0 {
        out.push_str(&format!("version = {format}\n"));
    }
    for (p, lock) in union.values() {
        out.push_str(&format!(
            "\n[[package]]\nname = \"{}\"\nversion = \"{}\"\n",
            p.name, p.version
        ));
        if let Some(s) = &p.source {
            out.push_str(&format!("source = \"{s}\"\n"));
        }
        if let Some(c) = &p.checksum {
            out.push_str(&format!("checksum = \"{c}\"\n"));
        }
        let deps = lock.resolved(p);
        if !deps.is_empty() {
            out.push_str("dependencies = [\n");
            for (name, version) in deps
                .iter()
                .flat_map(|(n, vs)| vs.iter().map(move |v| (n, v)))
            {
                out.push_str(&format!(" \"{name} {version}\",\n"));
            }
            out.push_str("]\n");
        }
    }
    Ok(out)
}

/// Refuses a git package in `binding`'s lock: the registry-only seed
/// and check cannot hold its commit.
pub fn refuse_git(binding: &str, lock: &Lock) -> Result<(), LockError> {
    match lock.packages.iter().find(|p| p.is_git()) {
        Some(p) => Err(LockError::Git {
            binding: binding.to_string(),
            package: p.name.clone(),
        }),
        None => Ok(()),
    }
}

/// Seeded packages by name and version, each with the lock it came from.
type Union<'a> = BTreeMap<(String, String), (&'a Package, &'a Lock)>;

/// Each package of `bindings` the seed holds: every pinned one, and every
/// path one (the bindings themselves), so cargo keeps their dependency
/// edges too. Refuses git packages and compatible pins that differ.
fn union<'a>(bindings: &[BindingLock<'a>]) -> Result<Union<'a>, LockError> {
    let mut union: Union = BTreeMap::new();
    let mut owner: BTreeMap<(String, String), &str> = BTreeMap::new();
    for &(binding, lock) in bindings {
        refuse_git(binding, lock)?;
        for p in lock.packages.iter().filter(|p| p.is_pinned()) {
            let clash = owner.iter().find(|((name, version), _)| {
                *name == p.name && *version != p.version && compatible(version, &p.version)
            });
            if let Some(((name, version), other)) = clash {
                return Err(LockError::Drift {
                    package: name.clone(),
                    pins: [
                        (other.to_string(), version.clone()),
                        (binding.to_string(), p.version.clone()),
                    ],
                });
            }
            owner
                .entry((p.name.clone(), p.version.clone()))
                .or_insert(binding);
        }
        for p in lock
            .packages
            .iter()
            .filter(|p| p.source.is_none() || p.is_pinned())
        {
            let key = (p.name.clone(), p.version.clone());
            if let Some((q, _)) = union.get(&key).filter(|(q, _)| q.source != p.source) {
                let other = owner.get(&key).copied().unwrap_or("another binding");
                return Err(LockError::Sources {
                    package: format!("{} {}", q.name, q.version),
                    bindings: [other.to_string(), binding.to_string()],
                });
            }
            union
                .entry((p.name.clone(), p.version.clone()))
                .or_insert((p, lock));
        }
    }
    Ok(union)
}

/// A binding holding a package of a built lock: its name, its entry, and
/// its lock.
type Holder<'a> = (&'a str, &'a Package, &'a Lock);

/// Checks the lock cargo wrote, `built`, one way against the bindings':
/// every pinned entry is pinned by some binding with the same checksum,
/// and every dependency edge an entry (pinned, or a binding's path
/// package) shares with a binding's entry resolves to the same version.
/// Entries the build did not need may be missing.
pub fn verify(built: &Lock, bindings: &[BindingLock]) -> Result<(), LockError> {
    for p in &built.packages {
        let holders: Vec<Holder> = bindings
            .iter()
            .filter_map(|&(b, l)| l.find(p).map(|q| (b, q, l)))
            .collect();
        if p.is_pinned() {
            check_entry(p, &holders, bindings)?;
        }
        check_edges(built, p, &holders)?;
    }
    Ok(())
}

/// `p` is pinned by the first of `holders`, with the same checksum.
fn check_entry(p: &Package, holders: &[Holder], bindings: &[BindingLock]) -> Result<(), LockError> {
    let Some(&(first, q, _)) = holders.first() else {
        let detail = format!("`{} {}` is not pinned", p.name, p.version);
        return Err(changed(bindings, &p.name, detail));
    };
    if q.checksum != p.checksum {
        let detail = format!("`{} {}` has another checksum", p.name, p.version);
        return Err(changed_in(first, detail));
    }
    Ok(())
}

/// Each edge of `p` in `built` that a holder's entry also has resolves to
/// the same versions.
fn check_edges(built: &Lock, p: &Package, holders: &[Holder]) -> Result<(), LockError> {
    let built_deps = built.resolved(p);
    for &(binding, q, lock) in holders {
        for (dep, versions) in lock.resolved(q) {
            let Some(got) = built_deps.get(&dep).filter(|got| **got != versions) else {
                continue;
            };
            let detail = format!(
                "`{} {}` depends on `{dep} {}`, not `{dep} {}`",
                p.name,
                p.version,
                join(got),
                join(&versions)
            );
            return Err(changed_in(binding, detail));
        }
    }
    Ok(())
}

fn join(versions: &BTreeSet<String>) -> String {
    versions.iter().cloned().collect::<Vec<_>>().join(" and ")
}

/// A change to `name`: blamed on the binding that pins another version
/// of it, if one does.
fn changed(bindings: &[BindingLock], name: &str, detail: String) -> LockError {
    let binding = bindings
        .iter()
        .find(|(_, l)| l.packages.iter().any(|p| p.name == name))
        .map(|(b, _)| b.to_string());
    LockError::Changed { binding, detail }
}

fn changed_in(binding: &str, detail: String) -> LockError {
    LockError::Changed {
        binding: Some(binding.to_string()),
        detail,
    }
}

/// Whether cargo treats versions `a` and `b` as compatible, so one
/// resolution holds only one of them: the same major from 1.0, the
/// same minor in 0.x with x > 0, the same patch in 0.0.x.
pub fn compatible(a: &str, b: &str) -> bool {
    match (numbers(a), numbers(b)) {
        (Some([0, 0, pa]), Some([0, 0, pb])) => pa == pb,
        (Some([0, ma, _]), Some([0, mb, _])) => ma == mb,
        (Some([ma, ..]), Some([mb, ..])) => ma == mb,
        _ => false,
    }
}

/// `major.minor.patch`, ignoring a pre-release or build suffix.
fn numbers(v: &str) -> Option<[u64; 3]> {
    let core = v.split(['-', '+']).next()?;
    let mut parts = core.split('.').map(|n| n.parse::<u64>().ok());
    Some([parts.next()??, parts.next()??, parts.next()??])
}

#[cfg(test)]
mod tests;
