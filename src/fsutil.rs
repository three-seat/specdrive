use std::path::Path;

use crate::Result;

pub fn copy_template_once(from: &Path, to: &Path) -> Result<()> {
    if !from.exists() {
        return Err(format!("template missing: {}", from.display()).into());
    }
    if to.exists() {
        return Err(format!("refusing to overwrite: {}", to.display()).into());
    }
    if let Some(parent) = to.parent() {
        std::fs::create_dir_all(parent)?;
    }
    std::fs::copy(from, to)?;
    Ok(())
}

/// Copy a UTF-8 text template to `to`, doing simple string replacements.
/// Refuses to overwrite existing destination.
pub fn copy_template_with_replacements(
    from: &Path,
    to: &Path,
    replacements: &[(&str, &str)],
) -> Result<()> {
    if !from.exists() {
        return Err(format!("template missing: {}", from.display()).into());
    }
    if to.exists() {
        return Err(format!("refusing to overwrite: {}", to.display()).into());
    }
    if let Some(parent) = to.parent() {
        std::fs::create_dir_all(parent)?;
    }

    let mut contents = std::fs::read_to_string(from)
        .map_err(|e| format!("failed to read template {}: {e}", from.display()))?;

    for (needle, value) in replacements {
        contents = contents.replace(needle, value);
    }

    std::fs::write(to, contents).map_err(|e| format!("failed to write {}: {e}", to.display()))?;

    Ok(())
}
