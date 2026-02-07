pub type Result<T> = std::result::Result<T, Box<dyn std::error::Error + Send + Sync>>;

mod bootstrap;
mod cli;
mod config;
mod draft;
mod feature;
mod fsutil;
mod git;
mod implement;
mod specify;
mod utils;

fn main() {
    if let Err(err) = cli::run() {
        eprintln!("error: {err}");

        // Per F-001, F-002, F-003, and F-005 contracts: bootstrap, implement, draft, and config use specific exit codes
        // Check if the error is a BootstrapError, ImplementError, DraftError, or ConfigError and use its exit code
        let exit_code = if let Some(bootstrap_err) = err.downcast_ref::<bootstrap::BootstrapError>()
        {
            bootstrap_err.exit_code()
        } else if let Some(implement_err) = err.downcast_ref::<implement::ImplementError>() {
            implement_err.exit_code()
        } else if let Some(draft_err) = err.downcast_ref::<draft::DraftError>() {
            draft_err.exit_code()
        } else if let Some(config_err) = err.downcast_ref::<config::ConfigError>() {
            config_err.exit_code()
        } else {
            1 // Default exit code for all other errors
        };

        std::process::exit(exit_code);
    }
}
