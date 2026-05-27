use crate::Result;
use crate::bootstrap;
use crate::draft;
use crate::feature;
use crate::implement;
use crate::patch;

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

fn print_usage() {
    eprintln!("specdrive");
    eprintln!();
    eprintln!("Usage:");
    eprintln!("  specdrive bootstrap");
    eprintln!("  specdrive new-feature <FEATURE_ID> [--critical]");
    eprintln!("  specdrive implement <FEATURE_ID>");
    eprintln!("  specdrive draft <FEATURE_ID>");
    eprintln!("  specdrive patch emit <FEATURE_ID>");
    eprintln!("  specdrive help");
}
