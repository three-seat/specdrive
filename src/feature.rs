use std::path::{Path, PathBuf};

use crate::fsutil;
use crate::specify;
use crate::Result;

pub fn new_feature(feature_id: &str, critical: bool) -> Result<()> {
    // 1) Create spec file from template (idempotent)
    let spec_path = specify::ensure_feature_spec(feature_id)?;


    // 2) Create docs/features/<id>/
    let feature_dir = Path::new("docs").join("features").join(feature_id);
    std::fs::create_dir_all(&feature_dir)?;

    // 3) Copy contract template -> docs/features/<id>/contract.yaml
    let template = if critical {
        PathBuf::from("docs/templates/feature.contract.critical.yaml")
    } else {
        PathBuf::from("docs/templates/feature.contract.minimal.yaml")
    };

    let dest = feature_dir.join("contract.yaml");

    // Replace placeholders in your contract templates too.
    // Recommended placeholders in templates:
    //   id: "F-XXX"
    //   title: "TODO"
    fsutil::copy_template_with_replacements(
        &template,
        &dest,
        &[
            ("F-XXX", feature_id),
            ("<TITLE>", &default_title(feature_id)),
        ],
    )?;

    println!("Scaffolded feature: {feature_id}");
    println!("  Spec: {}", spec_path.display());
    println!("  Contract: {}", dest.display());
    Ok(())
}

fn default_title(feature_id: &str) -> String {
    // keep it consistent with specify.rs helper
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
