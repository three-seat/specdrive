use std::path::{Path, PathBuf};

use crate::{fsutil, utils, Result};

pub fn ensure_feature_spec(feature_id: &str) -> Result<PathBuf> {
    let template = Path::new(".specify").join("templates").join("feature.spec.md");
    let dest = Path::new(".specify")
        .join("specs")
        .join(format!("{feature_id}.spec.md"));

    if dest.exists() {
        return Ok(dest);
    }

    let title = human_title_from_feature_id(feature_id);

    fsutil::copy_template_with_replacements(
    &template,
    &dest,
    &[
        ("F-XXX", feature_id),
        ("<title>", &title),
        ("docs/features/F-XXX/contract.yaml", &format!("docs/features/{feature_id}/contract.yaml")),
        ("<YYYY-MM-DD>", &utils::today_yyyy_mm_dd()),
        ("<you>", "three_seat"),
    ],
)?;


    Ok(dest)
}

fn human_title_from_feature_id(feature_id: &str) -> String {
    // Example: "F-001-bootstrap" -> "Bootstrap"
    let s = feature_id
        .splitn(2, '-')
        .nth(1)
        .unwrap_or(feature_id);

    s.replace('-', " ")
        .split_whitespace()
        .map(|w| {
            let mut chars = w.chars();
            match chars.next() {
                None => String::new(),
                Some(first) => first.to_uppercase().collect::<String>() + chars.as_str(),
            }
        })
        .collect::<Vec<_>>()
        .join(" ")
}