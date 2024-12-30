use std::{
    cell::RefCell,
    collections::HashMap,
    rc::Rc,
};

use crate::{EvalResult, EvalResults};

pub(crate) struct EvalNode {
    name: String,
    res: EvalResults,
    children: Vec<Rc<RefCell<EvalNode>>>,
}

impl EvalNode {
    /// 创建新节点
    pub(crate) fn new(name: &str, res: EvalResults) -> Rc<RefCell<Self>> {
        Rc::new(RefCell::new(Self {
            name: name.to_string(),
            res,
            children: Vec::new(),
        }))
    }

    /// 递归生成 DOT 格式字符串
    pub(crate) fn to_dot(&self, dot: &mut String, parent_id: Option<usize>, counter: &mut usize) {
        let node_id = *counter; // 当前节点的唯一 ID
        *counter += 1;

        // 定义节点的颜色
        let color = match (self.res.0, self.res.1) {
            (EvalResult::Err, _) | (_, EvalResult::Err) => "red",
            (EvalResult::TP, EvalResult::TN) => "green",
            (EvalResult::TP, EvalResult::FP) => "blue", // 误报
            (EvalResult::FN, EvalResult::FP) => "gray", // 漏报 + 误报
            (EvalResult::FN, EvalResult::TN) => "orange", // 漏报
            _ => unreachable!()
        };

        // 添加当前节点
        dot.push_str(&format!(
            "node{} [label=\"{}\" style=filled fillcolor={}];\n",
            node_id, self.name, color
        ));

        // 如果有父节点，连接边
        if let Some(parent_id) = parent_id {
            dot.push_str(&format!("node{} -> node{};\n", parent_id, node_id));
        }

        // 递归处理子节点
        for child in &self.children {
            child.borrow().to_dot(dot, Some(node_id), counter);
        }
    }
}

pub(crate) struct EvalTree {
    root: Option<Rc<RefCell<EvalNode>>>,
    node_map: HashMap<String, Rc<RefCell<EvalNode>>>,
}

impl EvalTree {
    /// 创建空树
    pub(crate) fn new() -> Self {
        Self {
            root: None,
            node_map: HashMap::new(),
        }
    }

    /// 获取节点数
    pub(crate) fn count_nodes(&self) -> usize {
        self.node_map.len()
    }

    /// 设置根节点
    pub(crate) fn set_root(&mut self, root: Rc<RefCell<EvalNode>>) {
        self.node_map
            .insert(root.borrow().name.clone(), Rc::clone(&root));
        self.root = Some(root);
    }

    /// 根据 name 查找节点
    pub(crate) fn get_node(&self, name: &str) -> Option<Rc<RefCell<EvalNode>>> {
        self.node_map.get(name).cloned()
    }

    /// 添加子节点
    pub(crate) fn add_child(
        &mut self,
        parent_name: &str,
        child_name: &str,
        child_res: EvalResults,
    ) -> Result<(), String> {
        if let Some(parent) = self.get_node(parent_name) {
            let child = EvalNode::new(child_name, child_res);
            parent.borrow_mut().children.push(Rc::clone(&child));
            self.node_map
                .insert(child_name.to_string(), Rc::clone(&child));
            Ok(())
        } else {
            Err(format!("Parent node '{}' not found", parent_name))
        }
    }

    pub(crate) fn to_dot(&self) -> String {
        let mut dot = String::from("digraph EvalTree {\n");
        dot.push_str("node [shape=ellipse];\n"); // 设置全局节点格式

        if let Some(root) = &self.root {
            let mut counter = 0;
            root.borrow().to_dot(&mut dot, None, &mut counter);
        }

        dot.push('}');
        dot
    }
}

#[cfg(test)]
mod test {
    use super::*;
    #[test]
    fn test() {
        // 创建树并设置根节点
        let mut tree = EvalTree::new();
        let root = EvalNode::new("Root", EvalResults(EvalResult::TP, EvalResult::TN));
        tree.set_root(Rc::clone(&root));

        // 添加子节点
        tree.add_child("Root", "Child1", EvalResults(EvalResult::TP, EvalResult::TN)).unwrap();
        tree.add_child("Root", "Child2", EvalResults(EvalResult::TP, EvalResult::FP)).unwrap();
        tree.add_child("Child1", "GrandChild1", EvalResults(EvalResult::FN, EvalResult::TN)).unwrap();
        tree.add_child("Child2", "GrandChild2", EvalResults(EvalResult::FN, EvalResult::FP)).unwrap();

        // 生成 DOT 文件内容
        let dot_content = tree.to_dot();
        println!("DOT Representation:\n{}", dot_content);

        // 保存到文件
        let dot_path = "tree.dot";
        std::fs::write(dot_path, dot_content).expect("Failed to write DOT file");

        // 调用 Graphviz 将 DOT 转为图片
        let output = std::process::Command::new("dot")
            .args(&["-Tpng", "-o", "tree.png", dot_path])
            .output()
            .expect("Failed to execute Graphviz");
        if output.status.success() {
            println!("EvalTree image generated: tree.png");
        } else {
            eprintln!(
                "Error generating image: {}",
                String::from_utf8_lossy(&output.stderr)
            );
        }
    }
}