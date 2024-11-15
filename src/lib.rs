mod config;
mod utils;

use std::{path::PathBuf, process::{Command, Output}};

use config::Testcases;

pub struct Evaluator {
    executor: Executor,
    config: Config,
    output: PathBuf,
}

impl Evaluator {
    pub fn new(tool: PathBuf, config: PathBuf, output: PathBuf) -> Self {
        utils::is_executable(&tool);
        let harness = output.join("harness");
        let output = output.join(tool.file_stem().unwrap());
        Evaluator {
            executor: Executor::new(tool, harness),
            config: Config::new(config),
            output,
        }
    }

    pub fn main(self) {
        let testcases = Testcases::from_file(&self.config.case);
        for testcase in testcases[0..10].iter() {
            let cases = testcase.cases(None);
            let outputs =  cases.map(|c| self.executor.execute(c));
            let res = utils::evaluate(outputs);
            println!("{:#?}\n{:?}\n=====================", testcase, res);  
        }
    }

}

pub(crate) struct Executor {
    tool: PathBuf,
    harness: PathBuf,
}

impl Executor {
    pub(crate) fn new(tool: PathBuf, harness: PathBuf) -> Self {
        utils::generate_harness(&harness);
        Executor { tool, harness }
    }

    pub(crate) fn execute(&self, program: Program) -> Output {
        program.into_harness(&self.harness);
        Command::new(&self.tool)
            .arg(&self.harness)
            .output()
            .expect("Tool failed to execute")
    }
}

#[derive(Debug)]
pub(crate) enum Res {
    Err, // 工具执行出错
    Pass, // 通过
    FP, // 误报
    FN, // 漏报
}

pub(crate) struct Program {
    code: String,
    metadata: String, // 注释格式的程序信息
}

impl Program {
    pub(crate) fn new(code: String, metadata: String) -> Self {
        Program {
            code,
            metadata,
        }
    }

    /// Merge `metadata` and `code`
    fn merge(&self) -> String {
        self.code.clone()
    }

    pub(crate) fn into_harness(&self, harness: &PathBuf) {
        std::fs::write(harness.join("src/main.rs"), self.merge()).expect("Failed to write");
    }
}

pub(crate) struct Config {
    case: PathBuf,
    flow: PathBuf,
}

impl Config {
    pub(crate) fn new(config: PathBuf) -> Self {
        Config {
            case: config.join("testcases.yaml"),
            flow: config.join("expressions.yaml"),
        }
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn test_execute() {
        let executor = Executor::new("../tools/eval_home/shell/Safedrop".into(), "./output/harness".into());
        let program = Program::new(r#"fn main() {
    println!("Hello, world!");
}
"#.to_string(), "".to_string());

        let output = executor.execute(program);
        println!("{:#?}", output);
    }
}
