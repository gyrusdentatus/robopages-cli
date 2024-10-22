use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

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

impl From<&super::Page> for Vec<Tool> {
    fn from(page: &super::Page) -> Self {
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

                // TODO: check if we can add examples

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
    pub id: CallId,
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
    pub call_id: CallId,
    pub content: String,
}

fn default_result_message_role() -> String {
    "tool".to_string()
}
