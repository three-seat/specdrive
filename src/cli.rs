use crate::Result;
use crate::bootstrap;
use crate::feature;

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
    eprintln!("  specdrive help");
}
