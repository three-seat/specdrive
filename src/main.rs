pub type Result<T> = std::result::Result<T, Box<dyn std::error::Error + Send + Sync>>;

mod cli;
mod feature;
mod fsutil;
mod git;
mod specify;
mod utils;

fn main() {
    if let Err(err) = cli::run() {
        eprintln!("error: {err}");
        std::process::exit(1);
    }
}
