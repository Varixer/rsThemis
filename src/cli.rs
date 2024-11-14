use std::path::PathBuf;

use clap::Parser;
use eval::Evaluator;
#[derive(Parser)]
pub(crate) struct Cli {
    /// Tools to be evaluated
    target: PathBuf,

    /// Configuration file
    #[arg(short, long, value_name = "DIR")]
    config: Option<PathBuf>,

    /// Output path
    #[arg(short, long, value_name = "DIR")]
    output: Option<PathBuf>,
}

impl Cli {
    pub(crate) fn main(self) {
        let current_dir = std::env::current_dir().unwrap();
        let output = self.output.unwrap_or(current_dir.join("output"));
        let config = self.config.unwrap_or(current_dir.join("config"));
        Evaluator::new(self.target, config, output).main();
    }
}
