use crate::feature;
use crate::Result;

pub fn run() -> Result<()> {
    let mut args = std::env::args().skip(1);

    let Some(cmd) = args.next() else {
        print_usage();
        return Ok(());
    };

    match cmd.as_str() {
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
    eprintln!("  specdrive new-feature <FEATURE_ID> [--critical]");
    eprintln!("  specdrive help");
}