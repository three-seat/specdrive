use std::path::PathBuf;

use crate::Result;
use crate::config;
use crate::feature_spec;
use crate::fsutil;

/// Scaffolds a new feature under `docs/features/<FEATURE_ID>/`.
///
/// Per ADR-002 / F-007, creates the canonical feature-local artifacts:
/// - `docs/features/<FEATURE_ID>/spec.md` (from template)
/// - `docs/features/<FEATURE_ID>/contract.yaml` (from minimal or critical template)
/// - `docs/features/<FEATURE_ID>/patches/` (empty directory)
pub fn new_feature(feature_id: &str, critical: bool) -> Result<()> {
    // Per F-005 contract: validate FEATURE_ID before any filesystem work
    config::validate_feature_id(feature_id).map_err(|e| {
        let err: Box<dyn std::error::Error + Send + Sync> = Box::new(e);
        err
    })?;

    let paths = fsutil::FeaturePaths::new(feature_id);
    std::fs::create_dir_all(&paths.dir)?;

    // 1) Create spec file from template (idempotent)
    let spec_path = feature_spec::ensure_feature_spec(feature_id)?;

    // 2) Copy contract template -> docs/features/<id>/contract.yaml
    let template = if critical {
        PathBuf::from("docs/templates/feature.contract.critical.yaml")
    } else {
        PathBuf::from("docs/templates/feature.contract.minimal.yaml")
    };

    fsutil::copy_template_with_replacements(
        &template,
        &paths.contract,
        &[
            ("F-XXX", feature_id),
            ("<TITLE>", &default_title(feature_id)),
        ],
    )?;

    // 3) Create patches/ directory (LLR-004)
    std::fs::create_dir_all(&paths.patches)?;

    println!("Scaffolded feature: {feature_id}");
    println!("  Spec:     {}", spec_path.display());
    println!("  Contract: {}", paths.contract.display());
    println!("  Patches:  {}", paths.patches.display());
    Ok(())
}

fn default_title(feature_id: &str) -> String {
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
