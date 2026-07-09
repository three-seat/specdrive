use crate::Result;
use crate::bootstrap;
use crate::chat;
use crate::draft;
use crate::feature;
use crate::implement;
use crate::lifecycle;
use crate::patch;
use crate::status;

pub fn run() -> Result<()> {
    let mut args = std::env::args().skip(1);

    let Some(cmd) = args.next() else {
        print_usage();
        return Ok(());
    };

    match cmd.as_str() {
        "bootstrap" => {
            // Per F-001 contract: bootstrap exits with specific codes (0, 1, or 2)
            // We need to convert BootstrapError to the generic Result type
            bootstrap::run().map_err(|e| {
                let err: Box<dyn std::error::Error + Send + Sync> = Box::new(e);
                err
            })
        }
        "new-feature" => {
            let Some(feature_id) = args.next() else {
                return Err("new-feature requires <FEATURE_ID>".into());
            };
            let critical = args.any(|a| a == "--critical");
            feature::new_feature(&feature_id, critical)
        }
        "implement" => {
            let Some(feature_id) = args.next() else {
                return Err("implement requires <FEATURE_ID>".into());
            };
            // Per F-002 contract: implement exits with specific codes (0, 1, or 2)
            implement::implement_feature(&feature_id)
        }
        "draft" => {
            let Some(feature_id) = args.next() else {
                return Err("draft requires <FEATURE_ID>".into());
            };
            // Per F-003 contract: draft exits with specific codes (0, 1, or 2)
            draft::draft_feature(&feature_id)
        }
        "patch" => {
            let Some(action) = args.next() else {
                return Err("patch requires an action (emit)".into());
            };
            match action.as_str() {
                "emit" => {
                    let Some(feature_id) = args.next() else {
                        return Err("patch emit requires <FEATURE_ID>".into());
                    };
                    // Per F-006 contract: patch emit exits with specific codes (0, 1, or 2)
                    patch::patch_emit_feature(&feature_id)
                }
                _ => Err(format!("unknown patch action: {}", action).into()),
            }
        }
        "chat" => {
            // Per F-009 contract: chat export|import <draft|implement> <FEATURE_ID>
            let Some(action) = args.next() else {
                return Err("chat requires an action (export|import)".into());
            };
            let Some(workflow) = args.next() else {
                return Err(format!("chat {action} requires <draft|implement>").into());
            };
            let Some(feature_id) = args.next() else {
                return Err(format!("chat {action} {workflow} requires <FEATURE_ID>").into());
            };
            chat::run(&action, &workflow, &feature_id)
        }
        // --- F-010 lifecycle commands ---
        "status" => {
            // status <FEATURE_ID> | status --all
            let rest: Vec<String> = args.collect();
            if rest.iter().any(|a| a == "--all") {
                status::run_all()
            } else {
                match first_positional(&rest) {
                    Some(feature_id) => status::run_one(feature_id),
                    None => Err("status requires <FEATURE_ID> or --all".into()),
                }
            }
        }
        "review" => single_feature_command(args, "review", lifecycle::commands::review),
        "done" => single_feature_command(args, "done", lifecycle::commands::done),
        "unblock" => single_feature_command(args, "unblock", lifecycle::commands::unblock),
        "resume" => single_feature_command(args, "resume", lifecycle::commands::resume),
        "block" => reason_feature_command(args, "block", lifecycle::commands::block),
        "defer" => reason_feature_command(args, "defer", lifecycle::commands::defer),
        "help" | "-h" | "--help" => {
            print_usage();
            Ok(())
        }
        _ => {
            print_usage();
            Ok(())
        }
    }
}

/// Returns the first argument that is not an option flag (does not start with
/// `--`).
fn first_positional(args: &[String]) -> Option<&str> {
    args.iter()
        .find(|a| !a.starts_with("--"))
        .map(|s| s.as_str())
}

/// Parses `--reason <value>` or `--reason=<value>` from the arguments.
fn parse_reason(args: &[String]) -> Option<String> {
    let mut it = args.iter();
    while let Some(arg) = it.next() {
        if arg == "--reason" {
            return it.next().cloned();
        }
        if let Some(rest) = arg.strip_prefix("--reason=") {
            return Some(rest.to_string());
        }
    }
    None
}

/// Dispatches a lifecycle command that takes a single `<FEATURE_ID>`.
fn single_feature_command(
    args: impl Iterator<Item = String>,
    command: &str,
    run: fn(&str) -> Result<()>,
) -> Result<()> {
    let rest: Vec<String> = args.collect();
    match first_positional(&rest) {
        Some(feature_id) => run(feature_id),
        None => Err(format!("{command} requires <FEATURE_ID>").into()),
    }
}

/// Dispatches a lifecycle command that takes `<FEATURE_ID> --reason "<reason>"`.
fn reason_feature_command(
    args: impl Iterator<Item = String>,
    command: &str,
    run: fn(&str, Option<&str>) -> Result<()>,
) -> Result<()> {
    let rest: Vec<String> = args.collect();
    let Some(feature_id) = first_positional(&rest) else {
        return Err(format!("{command} requires <FEATURE_ID>").into());
    };
    let reason = parse_reason(&rest);
    run(feature_id, reason.as_deref())
}

fn print_usage() {
    eprintln!("specdrive");
    eprintln!();
    eprintln!("Usage:");
    eprintln!("  specdrive bootstrap");
    eprintln!("  specdrive new-feature <FEATURE_ID> [--critical]");
    eprintln!("  specdrive implement <FEATURE_ID>");
    eprintln!("  specdrive draft <FEATURE_ID>");
    eprintln!("  specdrive patch emit <FEATURE_ID>");
    eprintln!("  specdrive chat export <draft|implement> <FEATURE_ID>");
    eprintln!("  specdrive chat import <draft|implement> <FEATURE_ID>");
    eprintln!("  specdrive status <FEATURE_ID>");
    eprintln!("  specdrive status --all");
    eprintln!("  specdrive review <FEATURE_ID>");
    eprintln!("  specdrive done <FEATURE_ID>");
    eprintln!("  specdrive block <FEATURE_ID> --reason \"<reason>\"");
    eprintln!("  specdrive defer <FEATURE_ID> --reason \"<reason>\"");
    eprintln!("  specdrive unblock <FEATURE_ID>");
    eprintln!("  specdrive resume <FEATURE_ID>");
    eprintln!("  specdrive help");
}
