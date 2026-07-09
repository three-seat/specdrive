//! Explicit, human-gated lifecycle commands (F-010, HLR-004).
//!
//! Each command validates its preconditions, and only when they hold appends a
//! single event to the feature's own `state.yaml` (LLR-015). On any
//! precondition failure the command rejects with a clear message and a non-zero
//! exit code, writing nothing.

use crate::Result;

use super::infer::{infer_base_state, patch_artifact_exists};
use super::state::{append_event, BaseState, Event, OverlayKind};
use super::{load_feature, now_timestamp, resolve_actor, FeatureContext, LifecycleError};

/// Wraps a lifecycle command result into the generic CLI `Result`, preserving
/// the `LifecycleError` for exit-code mapping in `main`.
fn wrap(result: std::result::Result<(), LifecycleError>) -> Result<()> {
    result.map_err(|e| {
        let err: Box<dyn std::error::Error + Send + Sync> = Box::new(e);
        err
    })
}

/// Builds a bare event (status/at/by) with optional fields left unset.
fn base_event(status: &str) -> Event {
    Event {
        status: status.to_string(),
        at: now_timestamp(),
        by: resolve_actor(),
        reason: None,
        previous_status: None,
        via: None,
    }
}

/// `specdrive review <FEATURE_ID>` (LLR-009, AC-4, AC-27).
///
/// Appends a review event and advances the base state to review, only when the
/// base state is patch, a patch artifact exists, and no overlay is unresolved.
pub fn review(feature_id: &str) -> Result<()> {
    wrap(review_inner(feature_id))
}

fn review_inner(feature_id: &str) -> std::result::Result<(), LifecycleError> {
    let FeatureContext { paths, mut log } = load_feature(feature_id)?;
    let computed = log.compute(infer_base_state(&paths));

    if computed.overlay.is_some() {
        return Err(overlay_active_error("review", &computed.overlay));
    }
    if computed.base != BaseState::Patch {
        return Err(LifecycleError::usage(format!(
            "review requires base state 'patch', but {} is '{}'. \
             Nothing was written.",
            feature_id,
            computed.base.as_str()
        )));
    }
    if !patch_artifact_exists(&paths) {
        return Err(LifecycleError::usage(format!(
            "review requires at least one patch artifact under {}, but none exists. \
             Nothing was written.",
            paths.patches.display()
        )));
    }

    let event = base_event("review");
    let at = event.at.clone();
    let by = event.by.clone();
    append_event(&paths, &mut log, event)?;

    println!();
    println!("Feature:  {}", feature_id);
    println!("Base:     patch");
    println!();
    println!("Review recorded:");
    println!("  by:  {}", by);
    println!("  at:  {}", at);
    println!();
    println!("State advanced to: review");
    Ok(())
}

/// `specdrive done <FEATURE_ID>` (LLR-010, AC-5, AC-27).
///
/// Appends a done event and advances the base state to done, only when the base
/// state is review, a review event exists in `state.yaml`, a patch artifact
/// exists, and no overlay is unresolved.
pub fn done(feature_id: &str) -> Result<()> {
    wrap(done_inner(feature_id))
}

fn done_inner(feature_id: &str) -> std::result::Result<(), LifecycleError> {
    let FeatureContext { paths, mut log } = load_feature(feature_id)?;
    let computed = log.compute(infer_base_state(&paths));

    if computed.overlay.is_some() {
        return Err(overlay_active_error("done", &computed.overlay));
    }
    if computed.base != BaseState::Review {
        return Err(LifecycleError::usage(format!(
            "done requires base state 'review', but {} is '{}'. Nothing was written.",
            feature_id,
            computed.base.as_str()
        )));
    }
    // A review event must exist in the recorded log (Q4, LLR-010).
    let has_review_event = log.events.iter().any(|e| e.status == "review");
    if !has_review_event {
        return Err(LifecycleError::usage(format!(
            "done requires a recorded review event in state.yaml for {}, but none exists. \
             Run 'specdrive review {}' first. Nothing was written.",
            feature_id, feature_id
        )));
    }
    if !patch_artifact_exists(&paths) {
        return Err(LifecycleError::usage(format!(
            "done requires at least one patch artifact under {}, but none exists. \
             Nothing was written.",
            paths.patches.display()
        )));
    }

    let event = base_event("done");
    let at = event.at.clone();
    let by = event.by.clone();
    append_event(&paths, &mut log, event)?;

    println!();
    println!("Feature:  {}", feature_id);
    println!("Base:     review");
    println!();
    println!("Done recorded:");
    println!("  by:  {}", by);
    println!("  at:  {}", at);
    println!();
    println!("State advanced to: done");
    Ok(())
}

/// `specdrive block <FEATURE_ID> --reason "<reason>"` (LLR-011, AC-6, AC-10).
pub fn block(feature_id: &str, reason: Option<&str>) -> Result<()> {
    wrap(overlay_command(
        feature_id,
        reason,
        OverlayKind::Blocked,
        "block",
    ))
}

/// `specdrive defer <FEATURE_ID> --reason "<reason>"` (LLR-012, AC-7, AC-10).
pub fn defer(feature_id: &str, reason: Option<&str>) -> Result<()> {
    wrap(overlay_command(
        feature_id,
        reason,
        OverlayKind::Deferred,
        "defer",
    ))
}

/// Shared implementation for block and defer: both apply an overlay recording
/// the reason and the current base state as `previous_status`.
fn overlay_command(
    feature_id: &str,
    reason: Option<&str>,
    kind: OverlayKind,
    command: &str,
) -> std::result::Result<(), LifecycleError> {
    // --reason is required and must be non-empty (AC-10, E-004). Checked before
    // any other precondition so the usage error is unambiguous.
    let reason = match reason {
        Some(r) if !r.trim().is_empty() => r.trim().to_string(),
        _ => {
            return Err(LifecycleError::usage(format!(
                "{} requires a non-empty --reason \"<reason>\". Nothing was written.",
                command
            )));
        }
    };

    let FeatureContext { paths, mut log } = load_feature(feature_id)?;
    let computed = log.compute(infer_base_state(&paths));

    if let Some(active) = computed.overlay {
        return Err(LifecycleError::usage(format!(
            "{} rejected: {} already has an active '{}' overlay. \
             Run 'specdrive {}' first. Nothing was written.",
            command,
            feature_id,
            active.as_str(),
            resolution_command(active),
        )));
    }
    if computed.base == BaseState::Done {
        return Err(LifecycleError::usage(format!(
            "{} rejected: {} is 'done' and cannot be {}. Nothing was written.",
            command,
            feature_id,
            kind.as_str()
        )));
    }

    let previous = computed.base;
    let event = Event {
        reason: Some(reason.clone()),
        previous_status: Some(previous.as_str().to_string()),
        ..base_event(kind.as_str())
    };
    let at = event.at.clone();
    let by = event.by.clone();
    append_event(&paths, &mut log, event)?;

    let label = match kind {
        OverlayKind::Blocked => "Blocked",
        OverlayKind::Deferred => "Deferred",
    };
    println!();
    println!("Feature:  {}", feature_id);
    println!("Base:     {}", previous.as_str());
    println!();
    println!("{}:", label);
    println!("  by:      {}", by);
    println!("  at:      {}", at);
    println!("  reason:  {}", reason);
    println!("  returns: {}", previous.as_str());
    println!();
    println!("State overlaid as: {}", kind.as_str());
    Ok(())
}

/// `specdrive unblock <FEATURE_ID>` (LLR-013, AC-8, AC-26).
pub fn unblock(feature_id: &str) -> Result<()> {
    wrap(resolve_overlay(feature_id, OverlayKind::Blocked, "unblock"))
}

/// `specdrive resume <FEATURE_ID>` (LLR-014, AC-9, AC-26).
pub fn resume(feature_id: &str) -> Result<()> {
    wrap(resolve_overlay(feature_id, OverlayKind::Deferred, "resume"))
}

/// Shared implementation for unblock and resume: both resolve the matching
/// active overlay, appending an event with `via` set whose status is the
/// revealed previous base state.
fn resolve_overlay(
    feature_id: &str,
    expected: OverlayKind,
    command: &str,
) -> std::result::Result<(), LifecycleError> {
    let FeatureContext { paths, mut log } = load_feature(feature_id)?;
    let computed = log.compute(infer_base_state(&paths));

    match computed.overlay {
        Some(active) if active == expected => {}
        Some(active) => {
            return Err(LifecycleError::usage(format!(
                "{} rejected: {} is '{}', not '{}'. Run 'specdrive {}' instead. \
                 Nothing was written.",
                command,
                feature_id,
                active.as_str(),
                expected.as_str(),
                resolution_command(active),
            )));
        }
        None => {
            return Err(LifecycleError::usage(format!(
                "{} rejected: {} has no active '{}' overlay. Nothing was written.",
                command,
                feature_id,
                expected.as_str()
            )));
        }
    }

    // The overlay event is the most recent event while an overlay is active; it
    // carries the previous_status to return to (LLR-013, LLR-014).
    let previous_status = computed
        .last_event
        .as_ref()
        .and_then(|e| e.previous_status.clone())
        .filter(|s| !s.trim().is_empty());

    let previous_status = match previous_status {
        Some(s) => s,
        None => {
            return Err(LifecycleError::usage(format!(
                "{} rejected: no previous_status was recorded for {}, cannot resolve the \
                 overlay. Nothing was written.",
                command, feature_id
            )));
        }
    };

    let was_reason = computed
        .last_event
        .as_ref()
        .and_then(|e| e.reason.clone())
        .unwrap_or_default();

    let event = Event {
        via: Some(command.to_string()),
        ..base_event(&previous_status)
    };
    let at = event.at.clone();
    let by = event.by.clone();
    append_event(&paths, &mut log, event)?;

    let past = match expected {
        OverlayKind::Blocked => "blocked",
        OverlayKind::Deferred => "deferred",
    };
    let resolved_label = match expected {
        OverlayKind::Blocked => "Unblocked",
        OverlayKind::Deferred => "Resumed",
    };
    println!();
    println!("Feature:  {}", feature_id);
    if was_reason.is_empty() {
        println!("Was:      {}", past);
    } else {
        println!("Was:      {} ({})", past, was_reason);
    }
    println!();
    println!("{}:", resolved_label);
    println!("  by:  {}", by);
    println!("  at:  {}", at);
    println!();
    println!("Overlay resolved. Base state returned to: {}", previous_status);
    Ok(())
}

/// The command that resolves a given overlay, for use in guidance messages.
fn resolution_command(kind: OverlayKind) -> &'static str {
    match kind {
        OverlayKind::Blocked => "unblock",
        OverlayKind::Deferred => "resume",
    }
}

/// Rejection message for a command invoked while an overlay is active.
fn overlay_active_error(command: &str, overlay: &Option<OverlayKind>) -> LifecycleError {
    let (state, resolver) = match overlay {
        Some(OverlayKind::Blocked) => ("blocked", "unblock"),
        Some(OverlayKind::Deferred) => ("deferred", "resume"),
        None => ("overlaid", "unblock or resume"),
    };
    LifecycleError::usage(format!(
        "{} rejected: feature has an active '{}' overlay. Run 'specdrive {}' first. \
         Nothing was written.",
        command, state, resolver
    ))
}
