pub type Result<T> = std::result::Result<T, Box<dyn std::error::Error + Send + Sync>>;

mod bootstrap;
mod cli;
mod feature;
mod fsutil;
mod git;
mod implement;
mod specify;
mod utils;

fn main() {
    if let Err(err) = cli::run() {
        eprintln!("error: {err}");

        // Per F-001 and F-002 contracts: bootstrap and implement use specific exit codes
        // Check if the error is a BootstrapError or ImplementError and use its exit code
        let exit_code = if let Some(bootstrap_err) = err.downcast_ref::<bootstrap::BootstrapError>()
        {
            bootstrap_err.exit_code()
        } else if let Some(implement_err) = err.downcast_ref::<implement::ImplementError>() {
            implement_err.exit_code()
        } else {
            1 // Default exit code for all other errors
        };

        std::process::exit(exit_code);
    }
}
