use anyhow::{bail, Context, Result};
use std::fs;
use std::io::ErrorKind;
use std::path::{Component, Path, PathBuf};

use super::model::PermissionEnvelope;

pub fn ensure_read_path(permissions: &PermissionEnvelope, path: impl AsRef<Path>) -> Result<()> {
    if path_within_roots(path.as_ref(), &permissions.read_roots)? {
        Ok(())
    } else {
        bail!(
            "read path {} is outside declared read roots",
            path.as_ref().display()
        )
    }
}

pub fn ensure_write_path(permissions: &PermissionEnvelope, path: impl AsRef<Path>) -> Result<()> {
    if path_within_roots(path.as_ref(), &permissions.write_roots)? {
        Ok(())
    } else {
        bail!(
            "write path {} is outside declared write roots",
            path.as_ref().display()
        )
    }
}

pub fn path_within_roots(path: &Path, roots: &[String]) -> Result<bool> {
    if roots.is_empty() {
        return Ok(false);
    }
    let path = resolved_normalized(path)?;
    for root in roots {
        if root.trim().is_empty() {
            continue;
        }
        let root = resolved_normalized(Path::new(root))?;
        if path.starts_with(&root) {
            return Ok(true);
        }
    }
    Ok(false)
}

pub fn absolute_normalized(path: &Path) -> Result<PathBuf> {
    let absolute = if path.is_absolute() {
        path.to_path_buf()
    } else {
        std::env::current_dir()
            .context("resolve current directory for work permission check")?
            .join(path)
    };
    Ok(normalize_lexically(&absolute))
}

/// Resolve every existing path component, including symlinks, while preserving
/// a lexically normalized suffix that does not exist yet. This lets permission
/// checks safely cover both existing read targets and not-yet-created outputs.
pub fn resolved_normalized(path: &Path) -> Result<PathBuf> {
    let absolute = if path.is_absolute() {
        path.to_path_buf()
    } else {
        std::env::current_dir()
            .context("resolve current directory for work permission check")?
            .join(path)
    };
    let mut resolved = PathBuf::new();
    let mut missing = Vec::new();

    for component in absolute.components() {
        match component {
            Component::Prefix(_) | Component::RootDir => {
                resolved.push(component.as_os_str());
            }
            Component::CurDir => {}
            Component::ParentDir => {
                if missing.pop().is_none() {
                    resolved.pop();
                }
            }
            Component::Normal(name) if !missing.is_empty() => {
                missing.push(name.to_os_string());
            }
            Component::Normal(name) => {
                let candidate = resolved.join(name);
                match fs::canonicalize(&candidate) {
                    Ok(canonical) => resolved = canonical,
                    Err(err) if err.kind() == ErrorKind::NotFound => {
                        match fs::symlink_metadata(&candidate) {
                            Ok(metadata) if metadata.file_type().is_symlink() => {
                                bail!("cannot resolve dangling symlink {}", candidate.display());
                            }
                            Ok(_) => {
                                return Err(err).with_context(|| {
                                    format!("resolve existing path {}", candidate.display())
                                });
                            }
                            Err(metadata_err) if metadata_err.kind() != ErrorKind::NotFound => {
                                return Err(metadata_err).with_context(|| {
                                    format!("inspect path component {}", candidate.display())
                                });
                            }
                            Err(_) => {}
                        }
                        missing.push(name.to_os_string());
                    }
                    Err(err) => {
                        return Err(err).with_context(|| {
                            format!("resolve path component {}", candidate.display())
                        });
                    }
                }
            }
        }
    }

    for component in missing {
        resolved.push(component);
    }
    Ok(resolved)
}

fn normalize_lexically(path: &Path) -> PathBuf {
    let mut normalized = PathBuf::new();
    for component in path.components() {
        match component {
            Component::CurDir => {}
            Component::ParentDir => {
                normalized.pop();
            }
            Component::RootDir | Component::Prefix(_) | Component::Normal(_) => {
                normalized.push(component.as_os_str());
            }
        }
    }
    normalized
}
