mod config;
mod utils;

use std::{
    path::PathBuf,
    process::{Command, Output},
};

use config::Testcases;
use log::info;

pub struct Evaluator {
    executor: Executor,
    config: Config,
    output: PathBuf,
}

impl Evaluator {
    pub fn new(
        tool: PathBuf,
        config: PathBuf,
        length: usize,
        depth: usize,
        output: PathBuf,
    ) -> Self {
        utils::is_executable(&tool);
        let harness = output.join("harness");
        let output = output.join(tool.file_stem().unwrap());
        Evaluator {
            executor: Executor::new(tool, harness),
            config: Config::new(config, length, depth),
            output,
        }
    }

    pub fn main(self) {
        let testcases = Testcases::from_file(&self.config.case);
        // for (idx, testcase) in testcases.iter().enumerate() {
        for (idx, testcase) in testcases[0..10].iter().enumerate() {
            // 输出目录
            let output = self.output.join(format!("testcase-{:03}", idx));

            // exprs 初始化
            let mut exprs = Vec::new();
            exprs.push(Expr::source());

            let expr = &exprs[0];
            let cases = testcase.cases(&expr.code);

            info!(
                "Write testcase-{:03} with expression-{} into file system",
                idx, &expr.num
            );
            utils::write(output.join(&expr.num), &cases);
            let outputs = cases.map(|c| self.executor.execute(c));
            let res = utils::evaluate(outputs);

            if let Res::Pass = res {
                // Todo
                if expr.length < self.config.length && expr.depth < self.config.depth {
                    // Todo: new_expr 加入到 exprs中

                }
            }
            // Todo: 插入评估树
            println!("{:?}", res);
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
    Err,  // 工具执行出错
    Pass, // 通过
    FP,   // 误报
    FN,   // 漏报
}

pub(crate) struct Expr {
    num: String,
    code: String,
    length: usize,
    depth: usize,
    metadata: String,
}

impl Expr {
    pub(crate) fn new(
        num: usize,
        code: String,
        length: usize,
        depth: usize,
        metadata: String,
    ) -> Self {
        let num = format!("{:04}-{}-{}", num, length, depth);
        Expr {
            num,
            code,
            length,
            depth,
            metadata,
        }
    }

    /// SOURCE!()
    pub(crate) fn source() -> Self {
        Expr::new(0, String::from("SOURCE!()"), 0, 0, String::from(""))
    }
}

pub(crate) struct Program {
    code: String,
    metadata: String, // 注释格式的程序信息
}

impl Program {
    pub(crate) fn new(code: String, metadata: String) -> Self {
        Program { code, metadata }
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
    length: usize,
    depth: usize,
}

impl Config {
    pub(crate) fn new(config: PathBuf, length: usize, depth: usize) -> Self {
        Config {
            case: config.join("testcases.yaml"),
            flow: config.join("expressions.yaml"),
            length,
            depth,
        }
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn test_execute() {
        let executor = Executor::new(
            "../tools/eval_home/shell/Safedrop".into(),
            "./output/harness".into(),
        );
        let program = Program::new(
            r#"
fn main() {
    println!("Hello, world!");
}
"#
            .to_string(),
            "".to_string(),
        );

        let output = executor.execute(program);
        println!("{:#?}", output);
    }
}
