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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::book::{Function, Page, Parameter};
    use std::collections::BTreeMap;

    fn create_test_page() -> Page {
        let mut functions = BTreeMap::new();
        let mut parameters = BTreeMap::new();
        parameters.insert(
            "param1".to_string(),
            Parameter {
                param_type: "string".to_string(),
                description: "Test parameter".to_string(),
                required: true,
                examples: None,
            },
        );

        functions.insert(
            "test_function".to_string(),
            Function {
                description: "A test function".to_string(),
                parameters,
                execution: crate::book::runtime::ExecutionContext::CommandLine(vec![
                    "echo".to_string(),
                    "test".to_string(),
                ]),
                container: None,
            },
        );

        Page {
            name: "TestPage".to_string(),
            description: Some("A test page".to_string()),
            categories: vec!["test".to_string()],
            functions,
        }
    }

    #[test]
    fn test_page_to_function_group() {
        let page = create_test_page();
        let function_groups: Vec<FunctionGroup> = (&page).into();

        assert_eq!(function_groups.len(), 1);
        let group = &function_groups[0];

        assert_eq!(group.name, "TestPage");
        assert_eq!(group.description, Some("A test page".to_string()));
        assert_eq!(group.actions.len(), 1);

        let action = &group.actions[0];
        assert_eq!(action.name, "test_function");
        assert_eq!(action.description, "A test function");
        assert_eq!(action.tool, "TestPage.test_function@robopages");

        let args = action.args.as_ref().unwrap();
        assert_eq!(args.len(), 1);
        assert_eq!(args.get("param1"), Some(&"Test parameter".to_string()));
    }

    #[test]
    fn test_empty_page() {
        let page = Page {
            name: "EmptyPage".to_string(),
            description: None,
            categories: vec![],
            functions: BTreeMap::new(),
        };

        let function_groups: Vec<FunctionGroup> = (&page).into();

        assert_eq!(function_groups.len(), 1);
        let group = &function_groups[0];

        assert_eq!(group.name, "EmptyPage");
        assert_eq!(group.description, None);
        assert_eq!(group.actions.len(), 0);
    }

    #[test]
    fn test_multiple_functions() {
        let mut page = create_test_page();
        page.functions.insert(
            "another_function".to_string(),
            Function {
                description: "Another test function".to_string(),
                parameters: BTreeMap::new(),
                execution: crate::book::runtime::ExecutionContext::CommandLine(vec![
                    "echo".to_string(),
                    "another".to_string(),
                ]),
                container: None,
            },
        );

        let function_groups: Vec<FunctionGroup> = (&page).into();

        assert_eq!(function_groups.len(), 1);
        let group = &function_groups[0];

        assert_eq!(group.actions.len(), 2);
        assert!(group.actions.iter().any(|a| a.name == "test_function"));
        assert!(group.actions.iter().any(|a| a.name == "another_function"));
    }
}
