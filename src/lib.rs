use std::path::PathBuf;

mod utils;
pub struct Evaluator {
    target: PathBuf,
    harness: PathBuf,
    config: PathBuf,
    output: PathBuf,
}

impl Evaluator {
    pub fn new(target: PathBuf, config: PathBuf, output: PathBuf) -> Self {
        let harness = output.join("harness");
        utils::generate_harness(&harness);
        Evaluator {
            target,
            harness,
            config,
            output,
        }
    }

    pub fn main(self) {
        
    }
}
