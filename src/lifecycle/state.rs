//! `state.yaml` schema, event log read/write, and state computation (F-010).
//!
//! The event log is append-only (LLR-015): writes read the existing log, push
//! exactly one new event, and serialize the whole log back with all prior
//! events preserved in order. Only explicit human command events are ever
//! written — inferred states are never persisted (LLR-016).

use std::fs;
use std::path::PathBuf;

use serde::{Deserialize, Serialize};

use crate::fsutil::FeaturePaths;

use super::LifecycleError;

/// The progress layer of lifecycle state, ordered
/// `draft < contract < patch < review < done` (HLR-002).
///
/// The derived `Ord` follows declaration order, which is the lifecycle
/// ordering used by the base-state precedence rule (LLR-004).
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum BaseState {
    Draft,
    Contract,
    Patch,
    Review,
    Done,
}

impl BaseState {
    pub fn as_str(self) -> &'static str {
        match self {
            BaseState::Draft => "draft",
            BaseState::Contract => "contract",
            BaseState::Patch => "patch",
            BaseState::Review => "review",
            BaseState::Done => "done",
        }
    }

    pub fn parse(s: &str) -> Option<BaseState> {
        match s {
            "draft" => Some(BaseState::Draft),
            "contract" => Some(BaseState::Contract),
            "patch" => Some(BaseState::Patch),
            "review" => Some(BaseState::Review),
            "done" => Some(BaseState::Done),
            _ => None,
        }
    }
}

/// The orthogonal overlay layer that suspends progress (HLR-002).
///
/// Overlay states are never inferred — they exist only as explicit events
/// (LLR-007).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OverlayKind {
    Blocked,
    Deferred,
}

impl OverlayKind {
    pub fn as_str(self) -> &'static str {
        match self {
            OverlayKind::Blocked => "blocked",
            OverlayKind::Deferred => "deferred",
        }
    }

    pub fn parse(s: &str) -> Option<OverlayKind> {
        match s {
            "blocked" => Some(OverlayKind::Blocked),
            "deferred" => Some(OverlayKind::Deferred),
            _ => None,
        }
    }
}

/// Whether a computed displayed state was set by an explicit event or inferred
/// from artifact presence.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Source {
    Inferred,
    Recorded,
}

impl Source {
    pub fn as_str(self) -> &'static str {
        match self {
            Source::Inferred => "inferred",
            Source::Recorded => "recorded",
        }
    }
}

/// A single append-only lifecycle event as stored in `state.yaml`.
///
/// `status` is the resulting status name (a base state, or `blocked`/`deferred`
/// for overlay events). `reason` and `previous_status` are recorded by
/// block/defer; `via` (`unblock` | `resume`) is recorded by overlay
/// resolutions, whose `status` is the revealed base state.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Event {
    pub status: String,
    pub at: String,
    pub by: String,
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub reason: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub previous_status: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub via: Option<String>,
}

/// The full `state.yaml` document: a feature id and an ordered event log.
///
/// The schema deliberately does not assume a single contract per feature,
/// leaving room for future parent/sub-contract lifecycle tracking (LLR-024).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct StateLog {
    pub feature_id: String,
    #[serde(default)]
    pub events: Vec<Event>,
}

/// The displayed lifecycle state: an active overlay if present, otherwise the
/// base state (LLR-005).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DisplayState {
    Base(BaseState),
    Overlay(OverlayKind),
}

impl DisplayState {
    pub fn as_str(self) -> &'static str {
        match self {
            DisplayState::Base(b) => b.as_str(),
            DisplayState::Overlay(o) => o.as_str(),
        }
    }
}

/// The fully computed lifecycle state for a feature, combining recorded events
/// with the current inferred base state.
#[derive(Debug, Clone)]
pub struct ComputedState {
    pub base: BaseState,
    pub overlay: Option<OverlayKind>,
    pub source: Source,
    /// The most recent event in the log, if any — supplies the displayed
    /// timestamp/actor and the overlay reason.
    pub last_event: Option<Event>,
}

impl ComputedState {
    /// The state to display: overlay if unresolved, otherwise base (LLR-005).
    pub fn displayed(&self) -> DisplayState {
        match self.overlay {
            Some(o) => DisplayState::Overlay(o),
            None => DisplayState::Base(self.base),
        }
    }
}

impl StateLog {
    fn empty(feature_id: &str) -> StateLog {
        StateLog {
            feature_id: feature_id.to_string(),
            events: Vec::new(),
        }
    }

    /// Computes the lifecycle state from the recorded events and the current
    /// inferred base state (HLR-002, LLR-004, LLR-005).
    ///
    /// - base = max(last recorded non-overlay event status, inferred).
    /// - overlay = last blocked/deferred event not since resolved by an
    ///   unblock/resume event.
    /// - Both layers are computed independently; the displayed state is the
    ///   overlay if one is active, otherwise the base.
    pub fn compute(&self, inferred: BaseState) -> ComputedState {
        let mut last_recorded_base: Option<BaseState> = None;
        let mut overlay: Option<OverlayKind> = None;

        for ev in &self.events {
            if let Some(bs) = BaseState::parse(&ev.status) {
                // A base-state event. Its status is the last recorded base.
                last_recorded_base = Some(bs);
                // unblock/resume events carry `via` and resolve the overlay.
                if ev.via.is_some() {
                    overlay = None;
                }
            } else if let Some(ok) = OverlayKind::parse(&ev.status) {
                overlay = Some(ok);
            }
            // Unknown statuses are ignored for state computation.
        }

        let base = match last_recorded_base {
            Some(recorded) => recorded.max(inferred),
            None => inferred,
        };

        // Source reflects the displayed state: an active overlay is always
        // recorded; otherwise the base is recorded when an explicit event is at
        // least as advanced as inference, else inferred.
        let source = if overlay.is_some() {
            Source::Recorded
        } else {
            match last_recorded_base {
                Some(recorded) if recorded >= inferred => Source::Recorded,
                _ => Source::Inferred,
            }
        };

        ComputedState {
            base,
            overlay,
            source,
            last_event: self.events.last().cloned(),
        }
    }
}

/// Path to a feature's `state.yaml` sidecar.
pub fn state_path(paths: &FeaturePaths) -> PathBuf {
    paths.dir.join("state.yaml")
}

/// Loads the feature's event log, or an empty log when no `state.yaml` exists
/// (LLR-023). Never creates or modifies any file.
///
/// A `state.yaml` that exists but cannot be parsed as the expected schema is an
/// IO-level failure (exit 2, E-006) — the caller must not proceed to write.
pub fn load_or_empty(
    feature_id: &str,
    paths: &FeaturePaths,
) -> Result<StateLog, LifecycleError> {
    let path = state_path(paths);
    if !path.exists() {
        return Ok(StateLog::empty(feature_id));
    }

    let contents = fs::read_to_string(&path)
        .map_err(|e| LifecycleError::io(format!("failed to read {}: {}", path.display(), e)))?;

    if contents.trim().is_empty() {
        return Ok(StateLog::empty(feature_id));
    }

    serde_yaml::from_str(&contents).map_err(|e| {
        LifecycleError::io(format!(
            "failed to parse lifecycle event log {}: {}",
            path.display(),
            e
        ))
    })
}

/// Appends exactly one event to the log and persists it (LLR-015, LLR-018).
///
/// The full log — all prior events unchanged and in order, plus the new one —
/// is serialized back to `state.yaml`, creating the file on first write. No
/// existing event is ever modified, reordered, or deleted.
pub fn append_event(
    paths: &FeaturePaths,
    log: &mut StateLog,
    event: Event,
) -> Result<(), LifecycleError> {
    log.events.push(event);

    let yaml = serde_yaml::to_string(log)
        .map_err(|e| LifecycleError::io(format!("failed to serialize lifecycle event log: {}", e)))?;

    let path = state_path(paths);
    fs::write(&path, yaml)
        .map_err(|e| LifecycleError::io(format!("failed to write {}: {}", path.display(), e)))?;

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn ev(status: &str) -> Event {
        Event {
            status: status.to_string(),
            at: "2026-06-07T14:00:00Z".to_string(),
            by: "three-seat".to_string(),
            reason: None,
            previous_status: None,
            via: None,
        }
    }

    #[test]
    fn base_state_ordering() {
        assert!(BaseState::Draft < BaseState::Contract);
        assert!(BaseState::Contract < BaseState::Patch);
        assert!(BaseState::Patch < BaseState::Review);
        assert!(BaseState::Review < BaseState::Done);
    }

    #[test]
    fn empty_log_uses_inferred() {
        let log = StateLog::empty("F-1");
        let c = log.compute(BaseState::Patch);
        assert_eq!(c.base, BaseState::Patch);
        assert_eq!(c.overlay, None);
        assert_eq!(c.source, Source::Inferred);
        assert!(c.last_event.is_none());
        assert_eq!(c.displayed(), DisplayState::Base(BaseState::Patch));
    }

    #[test]
    fn recorded_review_beats_lower_inference() {
        // TC-004: recorded review with only a contract artifact yields review.
        let mut log = StateLog::empty("F-1");
        log.events.push(ev("review"));
        let c = log.compute(BaseState::Contract);
        assert_eq!(c.base, BaseState::Review);
        assert_eq!(c.source, Source::Recorded);
    }

    #[test]
    fn overlay_computed_independently_of_base() {
        // TC-005: blocked over inferred patch -> displayed blocked, base patch.
        let mut log = StateLog::empty("F-1");
        let mut blocked = ev("blocked");
        blocked.reason = Some("waiting".to_string());
        blocked.previous_status = Some("patch".to_string());
        log.events.push(blocked);

        let c = log.compute(BaseState::Patch);
        assert_eq!(c.base, BaseState::Patch);
        assert_eq!(c.overlay, Some(OverlayKind::Blocked));
        assert_eq!(c.displayed(), DisplayState::Overlay(OverlayKind::Blocked));
        assert_eq!(c.source, Source::Recorded);
    }

    #[test]
    fn unblock_resolves_overlay() {
        let mut log = StateLog::empty("F-1");
        let mut blocked = ev("blocked");
        blocked.previous_status = Some("contract".to_string());
        log.events.push(blocked);
        let mut unblock = ev("contract");
        unblock.via = Some("unblock".to_string());
        log.events.push(unblock);

        let c = log.compute(BaseState::Contract);
        assert_eq!(c.overlay, None);
        assert_eq!(c.base, BaseState::Contract);
        assert_eq!(c.displayed(), DisplayState::Base(BaseState::Contract));
    }

    #[test]
    fn inference_advances_past_stale_recorded_base() {
        // Unblocked to contract, then a patch artifact appears: base=patch,
        // source=inferred because inference overtook the recorded base.
        let mut log = StateLog::empty("F-1");
        let mut unblock = ev("contract");
        unblock.via = Some("unblock".to_string());
        log.events.push(unblock);

        let c = log.compute(BaseState::Patch);
        assert_eq!(c.base, BaseState::Patch);
        assert_eq!(c.overlay, None);
        assert_eq!(c.source, Source::Inferred);
    }
}
