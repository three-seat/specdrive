//! `specdrive status` — strictly read-only lifecycle reporting (F-010, HLR-001).
//!
//! Status never writes to `state.yaml` or any other file under any
//! circumstances (LLR-003). It infers the base state from artifact presence,
//! combines it with any recorded events, and prints the result for a single
//! feature or, with `--all`, for every feature under `docs/features/`.

use std::fs;
use std::path::PathBuf;

use serde::Deserialize;

use crate::Result;
use crate::fsutil::FeaturePaths;
use crate::lifecycle::infer::infer_base_state;
use crate::lifecycle::state::{self, ComputedState};
use crate::lifecycle::{LifecycleError, ensure_git_repo, validate_feature_id};

/// Wraps a status result into the generic CLI `Result`, preserving the
/// `LifecycleError` for exit-code mapping in `main`.
fn wrap(result: std::result::Result<(), LifecycleError>) -> Result<()> {
    result.map_err(|e| {
        let err: Box<dyn std::error::Error + Send + Sync> = Box::new(e);
        err
    })
}

/// Minimal view over a contract used only to recover a feature's title.
#[derive(Debug, Deserialize)]
struct ContractHead {
    metadata: Option<ContractMeta>,
}

#[derive(Debug, Deserialize)]
struct ContractMeta {
    title: Option<String>,
}

/// `specdrive status <FEATURE_ID>` (LLR-001).
pub fn run_one(feature_id: &str) -> Result<()> {
    wrap(run_one_inner(feature_id))
}

fn run_one_inner(feature_id: &str) -> std::result::Result<(), LifecycleError> {
    validate_feature_id(feature_id)?;
    ensure_git_repo()?;

    let paths = FeaturePaths::new(feature_id);
    if !paths.dir.is_dir() {
        return Err(LifecycleError::usage(format!(
            "feature '{}' not found: {} does not exist",
            feature_id,
            paths.dir.display()
        )));
    }

    // Read-only load — never creates state.yaml (LLR-003, LLR-023).
    let log = state::load_or_empty(feature_id, &paths)?;
    let computed = log.compute(infer_base_state(&paths));

    let title = read_title(&paths);
    print_single(feature_id, title.as_deref(), &computed);
    Ok(())
}

/// `specdrive status --all` (LLR-002).
pub fn run_all() -> Result<()> {
    wrap(run_all_inner())
}

fn run_all_inner() -> std::result::Result<(), LifecycleError> {
    ensure_git_repo()?;

    let features = discover_features()?;

    println!();
    for feature_id in features {
        let paths = FeaturePaths::new(&feature_id);
        let log = state::load_or_empty(&feature_id, &paths)?;
        let computed = log.compute(infer_base_state(&paths));
        print_row(&feature_id, &computed);
    }
    Ok(())
}

/// Returns the sorted list of feature directory names under `docs/features/`.
fn discover_features() -> std::result::Result<Vec<String>, LifecycleError> {
    let features_dir = PathBuf::from("docs/features");
    let entries = match fs::read_dir(&features_dir) {
        Ok(e) => e,
        // No features directory means no features to report — not an error.
        Err(_) => return Ok(Vec::new()),
    };

    let mut names = Vec::new();
    for entry in entries.flatten() {
        let path = entry.path();
        if !path.is_dir() {
            continue;
        }
        if let Some(name) = path.file_name().and_then(|n| n.to_str()) {
            names.push(name.to_string());
        }
    }
    names.sort();
    Ok(names)
}

/// Reads a feature's title from `contract.yaml` metadata, falling back to the
/// spec front matter `title:` field. Never fails — returns `None` when no title
/// can be recovered.
fn read_title(paths: &FeaturePaths) -> Option<String> {
    if let Ok(contents) = fs::read_to_string(&paths.contract)
        && let Ok(head) = serde_yaml::from_str::<ContractHead>(&contents)
        && let Some(title) = head.metadata.and_then(|m| m.title)
    {
        let title = title.trim();
        if !title.is_empty() {
            return Some(title.to_string());
        }
    }

    title_from_spec_front_matter(paths)
}

/// Extracts `title:` from a spec's leading `---` front-matter block.
fn title_from_spec_front_matter(paths: &FeaturePaths) -> Option<String> {
    let contents = fs::read_to_string(&paths.spec).ok()?;
    let mut lines = contents.lines();
    if lines.next()?.trim() != "---" {
        return None;
    }
    for line in lines {
        let trimmed = line.trim();
        if trimmed == "---" {
            break;
        }
        if let Some(rest) = trimmed.strip_prefix("title:") {
            let value = rest.trim().trim_matches('"').trim_matches('\'').trim();
            if !value.is_empty() {
                return Some(value.to_string());
            }
        }
    }
    None
}

/// The timestamp and actor to display for the "Since"/"Actor" fields: the last
/// recorded event's metadata, or "--" when there is no event (LLR-001).
fn since_actor(computed: &ComputedState) -> (String, String) {
    match &computed.last_event {
        Some(ev) => (ev.at.clone(), ev.by.clone()),
        None => ("--".to_string(), "--".to_string()),
    }
}

/// The date portion (YYYY-MM-DD) of the last event, or "--" when none.
fn since_date(computed: &ComputedState) -> String {
    match &computed.last_event {
        Some(ev) if ev.at.len() >= 10 => ev.at[..10].to_string(),
        Some(ev) => ev.at.clone(),
        None => "--".to_string(),
    }
}

/// Prints the multi-line single-feature status view (LLR-001).
fn print_single(feature_id: &str, title: Option<&str>, computed: &ComputedState) {
    let header = match title {
        Some(t) => format!("{} ({})", feature_id, t),
        None => feature_id.to_string(),
    };
    let (since, actor) = since_actor(computed);

    println!();
    println!("Feature:  {}", header);
    println!("Status:   {}", computed.displayed().as_str());
    // The base line is shown only when an overlay is masking the base state.
    if computed.overlay.is_some() {
        println!("Base:     {}", computed.base.as_str());
    }
    println!("Source:   {}", computed.source.as_str());
    println!("Since:    {}", since);
    println!("Actor:    {}", actor);
    if computed.overlay.is_some()
        && let Some(reason) = computed.last_event.as_ref().and_then(|e| e.reason.as_ref())
    {
        println!("Reason:   {}", reason);
    }
}

/// Prints a single `--all` row (LLR-002).
fn print_row(feature_id: &str, computed: &ComputedState) {
    let (_, actor) = since_actor(computed);
    println!(
        "{:<24} {:<9} {:<9} {:<12} {}",
        feature_id,
        computed.displayed().as_str(),
        computed.source.as_str(),
        since_date(computed),
        actor,
    );
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::lifecycle::state::{BaseState, Event, StateLog};

    fn computed_with(events: Vec<Event>, inferred: BaseState) -> ComputedState {
        let log = StateLog {
            feature_id: "F-1".to_string(),
            events,
        };
        log.compute(inferred)
    }

    fn event(status: &str, reason: Option<&str>, previous: Option<&str>) -> Event {
        Event {
            status: status.to_string(),
            at: "2026-06-07T14:00:00Z".to_string(),
            by: "three-seat".to_string(),
            reason: reason.map(|s| s.to_string()),
            previous_status: previous.map(|s| s.to_string()),
            via: None,
        }
    }

    #[test]
    fn since_actor_uses_last_event() {
        let c = computed_with(vec![event("review", None, None)], BaseState::Patch);
        let (since, actor) = since_actor(&c);
        assert_eq!(since, "2026-06-07T14:00:00Z");
        assert_eq!(actor, "three-seat");
    }

    #[test]
    fn since_actor_is_dashes_without_events() {
        let c = computed_with(vec![], BaseState::Patch);
        let (since, actor) = since_actor(&c);
        assert_eq!(since, "--");
        assert_eq!(actor, "--");
    }

    #[test]
    fn since_date_truncates_to_day() {
        let c = computed_with(vec![event("blocked", Some("r"), Some("contract"))], BaseState::Contract);
        assert_eq!(since_date(&c), "2026-06-07");
    }
}
