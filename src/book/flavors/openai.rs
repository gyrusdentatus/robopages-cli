use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

use crate::book::Page;

// https://platform.openai.com/docs/guides/function-calling

#[derive(Debug, Serialize)]
pub(crate) struct Tool {
    #[serde(rename = "type")]
    #[serde(default = "function")]
    pub tool_type: String,
    pub function: Function,
}

#[derive(Debug, Serialize)]
pub(crate) struct Function {
    pub name: String,
    pub description: String,
    pub parameters: Parameters,
}

#[derive(Debug, Serialize)]
pub(crate) struct Parameters {
    #[serde(rename = "type")]
    #[serde(default = "object")]
    pub params_type: String,
    pub properties: BTreeMap<String, Parameter>,
    pub required: Vec<String>,
}

#[derive(Debug, Serialize)]
pub(crate) struct Parameter {
    #[serde(rename = "type")]
    pub param_type: String,
    pub description: String,
}

impl From<&Page> for Vec<Tool> {
    fn from(page: &Page) -> Self {
        page.functions
            .iter()
            .map(|(func_name, func)| {
                let mut properties = BTreeMap::new();
                let mut required = Vec::new();

                for (param_name, param) in &func.parameters {
                    properties.insert(
                        param_name.clone(),
                        Parameter {
                            param_type: param.param_type.clone(),
                            description: param.description.clone(),
                        },
                    );

                    if param.required {
                        required.push(param_name.clone());
                    }
                }

                // NOTE: it'd be nice if we could add examples

                Tool {
                    tool_type: "function".to_string(),
                    function: Function {
                        name: func_name.clone(),
                        description: func.description.clone(),
                        parameters: Parameters {
                            params_type: "object".to_string(),
                            properties,
                            required,
                        },
                    },
                }
            })
            .collect()
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub(crate) struct FunctionCall {
    pub name: String,
    pub arguments: BTreeMap<String, String>,
}

type CallId = String;

#[derive(Debug, Serialize, Deserialize)]
pub(crate) struct Call {
    pub id: Option<CallId>,
    #[serde(rename = "type")]
    #[serde(default = "default_call_type")]
    pub call_type: String,
    pub function: FunctionCall,
}

fn default_call_type() -> String {
    "function".to_string()
}

#[derive(Debug, Serialize, Deserialize)]
pub(crate) struct CallResultMessage {
    #[serde(default = "default_result_message_role")]
    pub role: String,
    pub call_id: Option<CallId>,
    pub content: String,
}

fn default_result_message_role() -> String {
    "tool".to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_function_call() {
        let mut arguments = BTreeMap::new();
        arguments.insert("arg1".to_string(), "value1".to_string());
        arguments.insert("arg2".to_string(), "value2".to_string());

        let function_call = FunctionCall {
            name: "test_function".to_string(),
            arguments,
        };

        assert_eq!(function_call.name, "test_function");
        assert_eq!(function_call.arguments.len(), 2);
        assert_eq!(
            function_call.arguments.get("arg1"),
            Some(&"value1".to_string())
        );
        assert_eq!(
            function_call.arguments.get("arg2"),
            Some(&"value2".to_string())
        );
    }

    #[test]
    fn test_call() {
        let function_call = FunctionCall {
            name: "test_function".to_string(),
            arguments: BTreeMap::new(),
        };

        let call = Call {
            id: Some("test_id".to_string()),
            call_type: "function".to_string(),
            function: function_call,
        };

        assert_eq!(call.id, Some("test_id".to_string()));
        assert_eq!(call.call_type, "function");
        assert_eq!(call.function.name, "test_function");
    }

    #[test]
    fn test_call_default_type() {
        let function_call = FunctionCall {
            name: "test_function".to_string(),
            arguments: BTreeMap::new(),
        };

        let call = Call {
            id: None,
            call_type: default_call_type(),
            function: function_call,
        };

        assert_eq!(call.call_type, "function");
    }

    #[test]
    fn test_call_result_message() {
        let message = CallResultMessage {
            role: "custom_role".to_string(),
            call_id: Some("test_id".to_string()),
            content: "Test content".to_string(),
        };

        assert_eq!(message.role, "custom_role");
        assert_eq!(message.call_id, Some("test_id".to_string()));
        assert_eq!(message.content, "Test content");
    }

    #[test]
    fn test_call_result_message_default_role() {
        let message = CallResultMessage {
            role: default_result_message_role(),
            call_id: None,
            content: "Test content".to_string(),
        };

        assert_eq!(message.role, "tool");
    }
}
