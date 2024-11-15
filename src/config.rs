use std::{
    ops::{Deref, DerefMut},
    path::Path,
};

use log::info;
use serde::{Deserialize, Serialize};

use crate::Program;

#[derive(Debug, Serialize, Deserialize)]
pub(crate) struct Testcases(Vec<Testcase>);

impl Testcases {
    pub(crate) fn from_file<P>(path: P) -> Self
    where
        P: AsRef<Path>,
    {
        info!("Testcases from file: {}", path.as_ref().display());
        let cnt = std::fs::read_to_string(path).expect("File not found");
        serde_yaml::from_str(&cnt).expect("File Content Format Error")
    }
}

impl Deref for Testcases {
    type Target = Vec<Testcase>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl DerefMut for Testcases {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub(crate) struct Testcase {
    #[serde(rename = "description")]
    desc: String,

    tags: Vec<String>,

    features: Vec<String>,
    #[serde(rename = "type")]
    ty: String,
    #[serde(rename = "value")]
    val: String,
    #[serde(rename = "POS")]
    pos: Case, // Positive Case
    #[serde(rename = "NEG")]
    neg: Case, // Negative Case
}

impl Testcase {
    pub(crate) fn cases(&self, expr: Option<&String>) -> [Program; 2] {
        let pos = Program::new(self.pos.nest(expr), "".to_string());
        let neg = Program::new(self.neg.nest(expr), "".to_string());
        [pos, neg]
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub(crate) struct Case {
    #[serde(rename = "source")]
    src: String,
    code: String,
}

impl Case {
    /// Nest `expr` to `self.src`, and then nest `self.src` to `self.code`
    pub(crate) fn nest(&self, expr: Option<&String>) -> String {
        let source = match expr {
            Some(e) => format!("{{\n{}\n}}", e.replace("SOURCE!()", &self.src)),
            None => self.src.clone(),
        };
        self.code.replace("SOURCE!()", source.trim())
    }
}

//TODO: flow impl

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn test_from_file() {
        let testcases = Testcases::from_file("config/testcases.yaml");
        assert_eq!(testcases[0].ty, String::from("Layout"));
    }

    #[test]
    fn test_generate() {
        let case = Case {
            src: String::from("Layout::from_size_align(0, 1).unwrap()"),
            code: String::from(
                r#"
    use std::alloc::{alloc, dealloc, Layout};
    fn main() {
        let layout = SOURCE!();
        let ptr = unsafe { alloc(layout) }; // SINK
        unsafe { dealloc(ptr, layout) };
    }"#,
            ),
        };

        let expr = String::from(
            r#"
    fn call(param: TYPE!()) -> TYPE!() {
        EXPRE!(param)
    }
    call(SOURCE!())
        "#,
        );

        println!("{}", case.nest(Some(&expr)));
    }
}
