use anyhow::{bail, Context, Result};
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
    let path = absolute_normalized(path)?;
    for root in roots {
        if root.trim().is_empty() {
            continue;
        }
        let root = absolute_normalized(Path::new(root))?;
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
