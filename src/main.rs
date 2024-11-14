use clap::Parser;
use cli::Cli;

mod cli;

fn main() {
    env_logger::init();
    Cli::parse().main();
}
