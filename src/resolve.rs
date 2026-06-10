//! Shared file resolution for spec-aware commands.
//!
//! Per F-009, the set of files that make up the context for the `draft` and
//! `implement` workflows is determined by these resolver functions and nothing
//! else. They are the single source of truth shared by the existing `draft`
//! and `implement` commands and the new `chat export` command. Export must not
//! perform any recursive or additional file discovery beyond what these
//! functions return (LLR-001, AC-16, AC-22).

use std::path::PathBuf;

use crate::fsutil;

/// The role a resolved file plays in a workflow context bundle.
///
/// Roles let consumers (the `draft`/`implement` prompt builders and the
/// `chat export` bundle assembler) format or inline each file appropriately
/// without re-deriving what each path is.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FileRole {
    Spec,
    Contract,
    Constitution,
    SystemOverview,
    Adr,
    MinimalTemplate,
    CriticalTemplate,
}

/// A single file in a resolved workflow context.
#[derive(Debug, Clone)]
pub struct ResolvedFile {
    /// Repository-root-relative path to the file.
    pub path: PathBuf,
    /// The role this file plays in the workflow context.
    pub role: FileRole,
    /// Whether the file is required. Spec and contract are required; all
    /// supporting context (constitution, system overview, ADRs, templates) is
    /// optional and may be missing without failing the workflow.
    pub required: bool,
}

impl ResolvedFile {
    fn new(path: PathBuf, role: FileRole, required: bool) -> Self {
        Self {
            path,
            role,
            required,
        }
    }
}

/// Resolves the file list for the `draft` workflow.
///
/// Per LLR-026, this returns, in order:
/// spec.md, contract.yaml, constitution, all ADR files, system overview, and
/// both contract templates (minimal and critical).
///
/// Spec and contract are marked required; all other files are optional context
/// that may be absent. ADRs are discovered via [`fsutil::find_adrs`], which
/// only returns files that exist. Constitution, system overview, and the
/// templates are returned as candidate paths regardless of existence so that
/// callers can decide whether to warn (export) or silently omit (draft prompt).
pub fn resolve_draft_files(feature_id: &str) -> Vec<ResolvedFile> {
    let fp = fsutil::FeaturePaths::new(feature_id);
    let tp = fsutil::TemplatePaths::new();

    let mut files = vec![
        ResolvedFile::new(fp.spec, FileRole::Spec, true),
        ResolvedFile::new(fp.contract, FileRole::Contract, true),
        ResolvedFile::new(
            PathBuf::from("docs/constitution.md"),
            FileRole::Constitution,
            false,
        ),
    ];
    files.extend(
        fsutil::find_adrs()
            .into_iter()
            .map(|adr| ResolvedFile::new(adr, FileRole::Adr, false)),
    );
    files.push(ResolvedFile::new(
        PathBuf::from("docs/system-overview.md"),
        FileRole::SystemOverview,
        false,
    ));
    files.push(ResolvedFile::new(
        tp.minimal,
        FileRole::MinimalTemplate,
        false,
    ));
    files.push(ResolvedFile::new(
        tp.critical,
        FileRole::CriticalTemplate,
        false,
    ));

    files
}

/// Resolves the file list for the `implement` workflow.
///
/// Per LLR-026, this returns, in order:
/// spec.md, contract.yaml, constitution, system overview, and all ADR files.
/// Templates are intentionally excluded for implement.
pub fn resolve_implement_files(feature_id: &str) -> Vec<ResolvedFile> {
    let fp = fsutil::FeaturePaths::new(feature_id);

    let mut files = vec![
        ResolvedFile::new(fp.spec, FileRole::Spec, true),
        ResolvedFile::new(fp.contract, FileRole::Contract, true),
        ResolvedFile::new(
            PathBuf::from("docs/constitution.md"),
            FileRole::Constitution,
            false,
        ),
        ResolvedFile::new(
            PathBuf::from("docs/system-overview.md"),
            FileRole::SystemOverview,
            false,
        ),
    ];
    files.extend(
        fsutil::find_adrs()
            .into_iter()
            .map(|adr| ResolvedFile::new(adr, FileRole::Adr, false)),
    );

    files
}

#[cfg(test)]
mod tests {
    use super::*;

    // These tests assert the static structure of the resolver output (roles,
    // ordering, required flags, template inclusion). They are independent of
    // the working directory: the candidate set always includes the same roles;
    // only which optional files actually exist varies, and that is decided by
    // consumers, not the resolver.

    #[test]
    fn test_resolve_draft_includes_templates() {
        let files = resolve_draft_files("F-009-chat");
        let roles: Vec<FileRole> = files.iter().map(|f| f.role).collect();

        assert_eq!(roles[0], FileRole::Spec);
        assert_eq!(roles[1], FileRole::Contract);
        // Draft must include both contract templates.
        assert!(roles.contains(&FileRole::MinimalTemplate));
        assert!(roles.contains(&FileRole::CriticalTemplate));

        // Spec is first; both templates are last, in minimal-then-critical order.
        assert_eq!(roles[roles.len() - 2], FileRole::MinimalTemplate);
        assert_eq!(roles[roles.len() - 1], FileRole::CriticalTemplate);
    }

    #[test]
    fn test_resolve_implement_excludes_templates() {
        let files = resolve_implement_files("F-009-chat");
        let roles: Vec<FileRole> = files.iter().map(|f| f.role).collect();

        assert_eq!(roles[0], FileRole::Spec);
        assert_eq!(roles[1], FileRole::Contract);
        // Implement must NOT include templates.
        assert!(!roles.contains(&FileRole::MinimalTemplate));
        assert!(!roles.contains(&FileRole::CriticalTemplate));
    }

    #[test]
    fn test_spec_and_contract_required() {
        let files = resolve_draft_files("F-009-chat");
        for f in &files {
            let required = matches!(f.role, FileRole::Spec | FileRole::Contract);
            assert_eq!(f.required, required, "role {:?}", f.role);
        }
    }

    #[test]
    fn test_resolve_uses_feature_local_paths() {
        let files = resolve_draft_files("F-009-chat");
        assert_eq!(
            files[0].path,
            PathBuf::from("docs/features/F-009-chat/spec.md")
        );
        assert_eq!(
            files[1].path,
            PathBuf::from("docs/features/F-009-chat/contract.yaml")
        );
    }
}
