pub type Result<T> = std::result::Result<T, Box<dyn std::error::Error + Send + Sync>>;

mod bootstrap;
mod cli;
mod feature;
mod fsutil;
mod git;
mod specify;
mod utils;

fn main() {
    if let Err(err) = cli::run() {
        eprintln!("error: {err}");

        // Per F-001 contract: bootstrap uses specific exit codes (1 for precondition, 2 for filesystem)
        // Check if the error is a BootstrapError and use its exit code
        let exit_code = if let Some(bootstrap_err) = err.downcast_ref::<bootstrap::BootstrapError>()
        {
            bootstrap_err.exit_code()
        } else {
            1 // Default exit code for all other errors
        };

        std::process::exit(exit_code);
    }
}
