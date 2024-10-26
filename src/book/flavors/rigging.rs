use serde::{Deserialize, Serialize};

use crate::book::Page;

// https://rigging.dreadnode.io/topics/tools/

#[derive(Debug, Deserialize, Serialize)]
pub(crate) struct Parameter {
    pub name: String,
    #[serde(rename = "type")]
    pub param_type: String,
    pub description: String,
    pub examples: Vec<String>,
}

// rigging uses python types
fn rigging_param_type(s: &str) -> String {
    if s == "string" {
        return "str".to_string();
    }

    s.to_string()
}

#[derive(Default, Serialize, Deserialize, Debug)]
pub(crate) struct Function {
    name: String,
    description: String,
    parameters: Vec<Parameter>,
}

#[derive(Default, Serialize, Deserialize, Debug)]
pub(crate) struct Tool {
    pub name: String,
    pub description: Option<String>,
    pub functions: Vec<Function>,
}

impl From<&Page> for Vec<Tool> {
    fn from(page: &Page) -> Self {
        let mut tool = Tool {
            name: page.name.clone(),
            description: page.description.clone(),
            functions: vec![],
        };

        for (func_name, func) in &page.functions {
            tool.functions.push(Function {
                name: func_name.clone(),
                description: func.description.clone(),
                parameters: func
                    .parameters
                    .iter()
                    .map(|p| Parameter {
                        name: p.0.clone(),
                        param_type: rigging_param_type(&p.1.param_type),
                        description: p.1.description.clone(),
                        examples: p.1.examples.clone().unwrap_or_default(),
                    })
                    .collect(),
            });
        }

        vec![tool]
    }
}
