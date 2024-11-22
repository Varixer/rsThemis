mod config;
mod eval_tree;
mod utils;

use config::{Flows, Testcases};
use eval_tree::{Node, Tree};
use log::info;
use rand::Rng as _;
use std::{
    collections::VecDeque,
    ops::{Deref, DerefMut},
    path::PathBuf,
    process::{Command, Output},
};

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
        let flows = Flows::from_file(&self.config.flow);

        let process = |idx: usize, expr: &Expr, cases: [Program; 2]| -> Res {
            // 写入文件
            info!(
                "Write testcase-{:03} with expression-{} into file system",
                idx, &expr.num
            );
            utils::write(
                self.output
                    .join(format!("testcase-{:03}", idx))
                    .join(&expr.num),
                &cases,
            );

            // 执行评估
            let outputs = cases.map(|c| self.executor.execute(c));
            utils::evaluate(outputs)
        };

        // Todo: 取消注释
        // for (idx, testcase) in testcases.iter().enumerate() {
        for (idx, testcase) in testcases.iter().enumerate() {
            let mut eval_tree = Tree::new();
            // 评估 testcase
            let src_expr = Expr::source();
            let base_cases = testcase.cases(&src_expr.code);
            let base_res = process(idx, &src_expr, base_cases);
            let root = Node::new(&src_expr.num, base_res);
            eval_tree.set_root(root);

            // 评估嵌套 flow 后的 testcase
            if let Res::Pass = base_res {
                // exprs 初始化
                let mut exprs = Exprs::new();
                exprs.push(Expr::source());

                // sources 队列
                let mut sources = VecDeque::new();
                sources.push_back(Expr::source());
                while !sources.is_empty() {
                    let src = sources.pop_front().unwrap();
                    for flow in flows.iter() {
                        let expr = flow.into_expr(eval_tree.count_nodes(), &src, &exprs, testcase);
                        let cases = testcase.cases(&expr.code);

                        let res = process(idx, &expr, cases);

                        // Todo: 插入评估树
                        eval_tree.add_child(&src.num, &expr.num, res).unwrap();
                        // println!("src: {}, res: {:?}", src.num, res);
                        if let Res::Pass = res {
                            if expr.length < self.config.length && expr.depth < self.config.depth {
                                sources.push_back(expr.clone());
                                exprs.push(expr);
                            }
                        }
                    }
                }
            }
            utils::generate_image_from_dot(
                &eval_tree.to_dot(),
                self.output.join(format!("testcase-{:03}.png", idx)),
            )
            .unwrap();
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

#[derive(Debug, Clone, Copy)]
pub(crate) enum Res {
    Err,  // 工具执行出错
    Pass, // 通过
    FP,   // 误报
    FN,   // 漏报
}

pub(crate) struct Exprs(Vec<Expr>);

impl Exprs {
    pub(crate) fn new() -> Self {
        Exprs(Vec::new())
    }

    /// 随机返回 `Exprs` 实例中一个 `Expr` 的共享引用
    pub(crate) fn random_expr(&self) -> Option<&Expr> {
        if self.0.is_empty() {
            None // 如果没有任何元素，返回 None
        } else {
            let mut rng = rand::thread_rng();
            let index = rng.gen_range(0..self.0.len()); // 随机生成索引
            self.0.get(index) // 返回共享引用
        }
    }
}

impl Deref for Exprs {
    type Target = Vec<Expr>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl DerefMut for Exprs {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

#[derive(Debug, Clone)]
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

    /// SOURCE!() 替换
    pub(crate) fn fill_source(&self, src: &String) -> String {
        self.code.replace("SOURCE!()", src)
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
