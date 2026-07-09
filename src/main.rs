pub type Result<T> = std::result::Result<T, Box<dyn std::error::Error + Send + Sync>>;

mod bootstrap;
mod chat;
mod cli;
mod config;
mod draft;
mod feature;
mod feature_spec;
mod fsutil;
mod git;
mod implement;
mod lifecycle;
mod patch;
mod resolve;
mod status;
mod utils;

fn main() {
    if let Err(err) = cli::run() {
        eprintln!("error: {err}");

        // Per F-001, F-002, F-003, F-005, and F-006 contracts: bootstrap, implement, draft, config, and patch use specific exit codes
        // Check if the error is a BootstrapError, ImplementError, DraftError, ConfigError, or PatchError and use its exit code
        let exit_code = if let Some(bootstrap_err) = err.downcast_ref::<bootstrap::BootstrapError>()
        {
            bootstrap_err.exit_code()
        } else if let Some(implement_err) = err.downcast_ref::<implement::ImplementError>() {
            implement_err.exit_code()
        } else if let Some(draft_err) = err.downcast_ref::<draft::DraftError>() {
            draft_err.exit_code()
        } else if let Some(config_err) = err.downcast_ref::<config::ConfigError>() {
            config_err.exit_code()
        } else if let Some(patch_err) = err.downcast_ref::<patch::PatchError>() {
            patch_err.exit_code()
        } else if let Some(chat_err) = err.downcast_ref::<chat::ChatError>() {
            chat_err.exit_code()
        } else if let Some(lifecycle_err) = err.downcast_ref::<lifecycle::LifecycleError>() {
            // Per F-010 contract: lifecycle commands use exit codes 0, 1, or 2
            lifecycle_err.exit_code()
        } else {
            1 // Default exit code for all other errors
        };

        std::process::exit(exit_code);
    }
}
