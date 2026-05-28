use std::path::PathBuf;

use crate::{Result, fsutil, utils};

/// Creates a feature spec under `docs/features/<FEATURE_ID>/spec.md` from the
/// template at `docs/templates/feature.spec.md`. Idempotent: if the spec
/// already exists, the existing path is returned without modification.
///
/// Per ADR-002 / F-007 the canonical feature spec lives in the feature-local
/// directory; `.specify/` is no longer involved.
pub fn ensure_feature_spec(feature_id: &str) -> Result<PathBuf> {
    let template = PathBuf::from("docs/templates/feature.spec.md");
    let dest = PathBuf::from("docs")
        .join("features")
        .join(feature_id)
        .join("spec.md");

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
            (
                "docs/features/F-XXX/contract.yaml",
                &format!("docs/features/{feature_id}/contract.yaml"),
            ),
            ("<YYYY-MM-DD>", &utils::today_yyyy_mm_dd()),
            ("<you>", "three_seat"),
        ],
    )?;

    Ok(dest)
}

fn human_title_from_feature_id(feature_id: &str) -> String {
    // Example: "F-001-bootstrap" -> "Bootstrap"
    let s = feature_id.splitn(2, '-').nth(1).unwrap_or(feature_id);

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
