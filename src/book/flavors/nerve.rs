use std::collections::HashMap;

use serde::{Deserialize, Serialize};

use crate::book::Page;

// https://github.com/evilsocket/nerve/blob/main/nerve-core/src/agent/task/tasklet.rs#L205

#[derive(Default, Serialize, Deserialize, Debug, Clone)]
pub(crate) struct Action {
    name: String,
    description: String,
    args: Option<HashMap<String, String>>,
    example_payload: Option<String>,
    tool: String,
}

#[derive(Default, Serialize, Deserialize, Debug, Clone)]
pub(crate) struct FunctionGroup {
    pub name: String,
    pub description: Option<String>,
    pub actions: Vec<Action>,
}

impl From<&Page> for Vec<FunctionGroup> {
    fn from(page: &Page) -> Self {
        let mut group = FunctionGroup {
            name: page.name.clone(),
            description: page.description.clone(),
            actions: vec![],
        };

        for (func_name, func) in &page.functions {
            let mut args = HashMap::new();
            for (param_name, param) in &func.parameters {
                args.insert(param_name.clone(), param.description.clone());
            }

            group.actions.push(Action {
                name: func_name.clone(),
                description: func.description.clone(),
                args: Some(args),
                example_payload: None,
                tool: format!("{}.{}@robopages", page.name, func_name),
            });
        }

        vec![group]
    }
}
