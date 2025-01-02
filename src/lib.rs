mod config;
mod eval_tree;
mod utils;

use config::{Flows, Testcases};
use core::fmt;
use eval_tree::{EvalNode, EvalTree};
use log::{error, info};
use rand::Rng as _;
use rayon::iter::{IntoParallelRefIterator as _, ParallelIterator as _};
use serde::{Serialize, Serializer};
use std::{
    collections::VecDeque,
    ops::{Deref, DerefMut},
    path::PathBuf,
    process::{Command, Output},
};
use tabled::{Table, Tabled};

pub struct Evaluator {
    executor: Executor,
    config: Config,
    targets: Vec<usize>,
    output: PathBuf,
}

impl Evaluator {
    pub fn new(
        tool: PathBuf,
        config: PathBuf,
        targets: Vec<usize>,
        length: usize,
        depth: usize,
        output: PathBuf,
    ) -> Self {
        utils::is_executable(&tool);
        let harness = output.join("harness");
        let output = output.join(tool.file_stem().unwrap());
        std::fs::create_dir_all(&output).expect(" std::fs::create_dir_all failed");
        Evaluator {
            executor: Executor::new(tool, harness),
            config: Config::new(config, length, depth),
            targets,
            output,
        }
    }

    pub fn main(&self) {
        // 确保 targets 的生命周期独立
        let targets = if self.targets.is_empty() {
            (0..self.config.testcases.len()).collect()
        } else {
            self.targets.clone()
        };
        // 并行处理每个任务
        let summaries: Vec<_> = targets
            .par_iter() // 使用并行迭代器
            .map(|&idx| self.evaluate_one(idx))
            .collect();

        let report = EvalReport::report(self.executor.name(), &summaries);

        // 写入结果
        utils::serialize_to_csv(&summaries, self.output.join("EvalSummary.csv")).unwrap();

        // 写入报告
        println!("{}", Table::new(vec![report]));
    }

    pub(crate) fn evaluate_one(&self, idx: usize) -> EvalSummary {
        if idx >= self.config.testcases.len() {
            error!(
                "Error: Index {} is out of bounds. Valid range is 0-{}",
                idx,
                self.config.testcases.len() - 1
            );
            std::process::exit(1);
        } else {
            utils::generate_harness(self.executor.harness.join(format!("harness-{}", idx)));
            self.evaluate(idx)
        }
    }

    pub(crate) fn evaluate(&self, idx: usize) -> EvalSummary {
        let process = |expr: &Expr, (pos, neg): (Program, Program)| -> EvalResults {
            // 写入文件
            info!(
                "Write testcase-{:03} with expression-{} into file system",
                idx, &expr.num
            );
            utils::write(
                self.output
                    .join(format!("testcase-{:03}", idx))
                    .join(&expr.num),
                (&pos, &neg),
            );

            // 执行评估
            let outputs = (
                self.executor.execute(idx, pos),
                self.executor.execute(idx, neg),
            );

            utils::evaluate(outputs)
        };

        // 获取要评估的 testcase
        let testcase = &self.config.testcases[idx];
        // 初始化 EvalSummary 和 Eval EvalTree
        let mut summary = EvalSummary::new(idx);
        let mut tree = EvalTree::new();

        // 评估 testcase
        let src_expr = Expr::source();
        let programs = testcase.into_programs(&src_expr.code);
        let res = process(&src_expr, programs);
        summary.count(&res);

        // BFS 遍历所有可行的 flow 的组合方案
        let root = EvalNode::new(&src_expr.num, res);
        tree.set_root(root);
        // 评估嵌套 flow 后的 testcase
        if let EvalResults(EvalResult::TP, EvalResult::TN) = res {
            // exprs 初始化
            let mut exprs = Exprs::new();
            exprs.push(Expr::source());

            // sources 队列
            let mut sources = VecDeque::new();
            sources.push_back(Expr::source());
            while !sources.is_empty() {
                let src = sources.pop_front().unwrap();
                for flow in self.config.flows.iter() {
                    let expr = flow.into_expr(tree.count_nodes(), &src, &exprs, testcase);
                    let programs = testcase.into_programs(&expr.code);
                    let res = process(&expr, programs);
                    summary.count(&res); // 统计
                    tree.add_child(&src.num, &expr.num, res).unwrap(); // 插入评估树

                    if let EvalResults(EvalResult::TP, EvalResult::TN) = res {
                        if expr.length < self.config.length && expr.depth < self.config.depth {
                            sources.push_back(expr.clone());
                            exprs.push(expr);
                        }
                    }
                }
            }
        }
        tree.to_json(self.output.join(format!("testcase-{:03}", idx))).unwrap();
        utils::generate_image_from_dot(
            &tree.to_dot(),
            self.output
                .join(format!("testcase-{:03}", idx))
                .join("evalTree.png"),
        )
        .unwrap();
        return summary;
    }
}

pub(crate) struct Executor {
    tool: PathBuf,
    harness: PathBuf,
}

impl Executor {
    pub(crate) fn new(tool: PathBuf, harness: PathBuf) -> Self {
        std::fs::create_dir_all(&harness).expect(" std::fs::create_dir_all failed");
        Executor { tool, harness }
    }

    pub(crate) fn name(&self) -> String {
        self.tool
            .file_stem()
            .unwrap()
            .to_os_string()
            .into_string()
            .unwrap()
    }

    pub(crate) fn execute(&self, idx: usize, program: Program) -> Output {
        program.into_harness(&self.harness.join(format!("harness-{}", idx)));
        Command::new(&self.tool)
            .arg(&self.harness.join(format!("harness-{}", idx)))
            .output()
            .expect("Tool failed to execute")
    }
}

#[derive(Debug, Clone, Copy)]
pub(crate) struct EvalResults(EvalResult, EvalResult);

impl Serialize for EvalResults {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let serialized_value = match (self.0, self.1) {
            (EvalResult::Err, _) | (_, EvalResult::Err) => "Error",
            (EvalResult::TP, EvalResult::TN) => "True Positive & Negative",
            (EvalResult::TP, EvalResult::FP) => "False Positive",
            (EvalResult::FN, EvalResult::FP) => "False Positive & Negative",
            (EvalResult::FN, EvalResult::TN) => "False Negative",
            _ => unreachable!(),
        };

        serializer.serialize_str(serialized_value)
    }
}

#[derive(Debug, Clone, Copy, Serialize)]
pub(crate) enum EvalResult {
    Err, // 工具执行出错
    TP,
    FP, // 误报
    FN, // 漏报
    TN,
}

#[derive(serde::Serialize)]
pub(crate) struct EvalSummary {
    #[serde(
        rename = "编号",
        serialize_with = "EvalSummary::format_with_leading_zeros"
    )]
    idx: usize,
    #[serde(rename = "变体")]
    variant_count: usize,
    #[serde(rename = "RD")]
    robust_count: usize,
    #[serde(rename = "TP")]
    tp_count: usize,
    #[serde(rename = "FN")]
    fn_count: usize,
    #[serde(rename = "FP")]
    fp_count: usize,
    #[serde(rename = "TN")]
    tn_count: usize,
    #[serde(rename = "ER")]
    err_count: usize,
}

impl EvalSummary {
    /// Init Eval Summary (All are 0)
    pub(crate) fn new(idx: usize) -> Self {
        EvalSummary {
            idx,
            variant_count: 0,
            robust_count: 0,
            tp_count: 0,
            fp_count: 0,
            fn_count: 0,
            tn_count: 0,
            err_count: 0,
        }
    }

    /// Count based on res enumeration
    pub(crate) fn count(&mut self, res: &EvalResults) {
        self.variant_count += 1;
        match res.0 {
            EvalResult::Err => self.err_count += 1,
            EvalResult::TP => self.tp_count += 1,
            EvalResult::FN => self.fn_count += 1,
            _ => unreachable!(), // POS Case 不存在 TN 和 FP
        }
        match res.1 {
            EvalResult::Err => self.err_count += 1,
            EvalResult::FP => self.fp_count += 1,
            EvalResult::TN => self.tn_count += 1,
            _ => unreachable!(), // NEG Case 不存在 TP 和 FN
        }
        if let EvalResults(EvalResult::TP, EvalResult::TN) = res {
            self.robust_count += 1;
        }
    }

    // Custom function to serialize numbers with leading zeros
    pub(crate) fn format_with_leading_zeros<S>(
        num: &usize,
        serializer: S,
    ) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        let formatted = format!("{:03}", num); // Format number with leading zeros, width = 3
        serializer.serialize_str(&formatted)
    }
}

#[derive(Default)]
pub(crate) struct Metric {
    normal: usize,
    absolute: usize,
}

impl fmt::Display for Metric {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        // 定义格式为 "normal (absolute)"
        write!(f, "{} ({})", self.normal, self.absolute)
    }
}

impl Metric {
    pub(crate) fn count(&mut self, src: usize, tar: usize) {
        if src != 0 {
            self.normal += 1;
            if src == tar {
                self.absolute += 1;
            }
        }
    }
}

#[derive(Default, Tabled)]
pub(crate) struct EvalReport {
    #[tabled(rename = "工具")]
    tool: String,
    #[tabled(rename = "鲁棒检测 (RD)")]
    robust_detection: Metric,
    #[tabled(rename = "真正例 (TP)")]
    true_positive: Metric,
    #[tabled(rename = "漏报 (FN)")]
    false_negative: Metric,
    #[tabled(rename = "误报 (FP)")]
    false_postive: Metric,
    #[tabled(rename = "真反例 (TN)")]
    true_negative: Metric,
    #[tabled(rename = "错误 (ER)")]
    error: Metric,
}

impl EvalReport {
    pub(crate) fn new(tool: String) -> Self {
        Self {
            tool,
            ..Default::default()
        }
    }
    pub(crate) fn report(tool: String, summaries: &[EvalSummary]) -> Self {
        let mut report = EvalReport::new(tool);
        summaries.iter().for_each(|s| {
            report
                .robust_detection
                .count(s.robust_count, s.variant_count);
            report.true_positive.count(s.tp_count, s.variant_count);
            report.false_negative.count(s.fn_count, s.variant_count);
            report.false_postive.count(s.fp_count, s.variant_count);
            report.true_negative.count(s.tn_count, s.variant_count);
            report.error.count(s.err_count, s.variant_count * 2);
        });
        report
    }
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
    testcases: Testcases,
    flows: Flows,
    length: usize,
    depth: usize,
}

impl Config {
    pub(crate) fn new(config: PathBuf, length: usize, depth: usize) -> Self {
        let testcases = Testcases::from_file(config.join("testcases.yaml"));
        let flows = Flows::from_file(config.join("expressions.yaml"));
        Config {
            testcases,
            flows,
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

        let output = executor.execute(0, program);
        println!("{:#?}", output);
    }
}
