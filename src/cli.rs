use std::path::PathBuf;

use clap::Parser;
use eval::Evaluator;
#[derive(Parser)]
pub(crate) struct Cli {
    /// Tools to be evaluated
    tool: PathBuf,

    /// Configuration file
    #[arg(short, long, value_name = "DIR")]
    config: Option<PathBuf>,

    /// Expression nesting depth
    #[arg(short, long, value_name = "NUM", default_value_t = 3)]
    depth: usize,

    /// Expression sequence length
    #[arg(short, long, value_name = "NUM", default_value_t = 2)]
    length: usize,

    /// Output path
    #[arg(short, long, value_name = "DIR")]
    output: Option<PathBuf>,
}

impl Cli {
    pub(crate) fn main(self) {
        let current_dir = std::env::current_dir().unwrap();
        let output = self.output.unwrap_or(current_dir.join("output"));
        let config = self.config.unwrap_or(current_dir.join("config"));
        Evaluator::new(self.tool, config, self.length, self.depth,  output).main();
    }
}
