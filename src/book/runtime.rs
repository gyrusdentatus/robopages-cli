use std::collections::BTreeMap;

use camino::Utf8PathBuf;
use lazy_regex::{lazy_regex, Lazy};
use regex::Regex;
use serde::{Deserialize, Serialize};

use crate::runtime::CommandLine;

use super::{Function, Page};

static ARG_VALUE_PARSER: Lazy<Regex> = lazy_regex!(r"(?m)\$\{\s*([\w\.]+)(\s+or\s+([^}]+))?\}");

const ARG_EXPRESSION_ERROR: &str =
    "argument expression must be in the form of ${name} or ${name or default_value}";

#[derive(Debug, Serialize, Deserialize)]
pub enum ExecutionContext {
    #[serde(rename = "cmdline")]
    CommandLine(Vec<String>),
    #[serde(rename = "platforms")]
    PlatformSpecific(BTreeMap<String, Vec<String>>),
}

impl ExecutionContext {
    pub fn get_command_line(&self) -> anyhow::Result<Vec<String>> {
        match self {
            Self::CommandLine(cmdline) => Ok(cmdline.clone()),
            Self::PlatformSpecific(platforms) => {
                if let Some(cmdline) = platforms.get(std::env::consts::OS) {
                    Ok(cmdline.clone())
                } else {
                    Err(anyhow::anyhow!(
                        "no command line for platform {}",
                        std::env::consts::OS
                    ))
                }
            }
        }
    }
}

#[allow(dead_code)] // we might need path and page in the future
#[derive(Debug)]
pub struct FunctionRef<'a> {
    pub name: String,
    pub path: &'a Utf8PathBuf,
    pub page: &'a Page,
    pub function: &'a Function,
}

impl<'a> FunctionRef<'a> {
    pub fn validate_arguments(
        &self,
        provided_arguments: &BTreeMap<String, String>,
    ) -> anyhow::Result<()> {
        // check for missing required arguments
        for (arg_name, param) in &self.function.parameters {
            if param.required && !provided_arguments.contains_key(arg_name) {
                return Err(anyhow::anyhow!(
                    "missing required argument {} for function {}",
                    arg_name,
                    &self.name
                ));
            }
        }

        // check for extra arguments
        for arg_name in provided_arguments.keys() {
            if !self.function.parameters.contains_key(arg_name) {
                return Err(anyhow::anyhow!(
                    "unknown argument {} for function {}",
                    arg_name,
                    &self.name
                ));
            }
        }

        Ok(())
    }

    pub fn resolve_command_line(
        &self,
        arguments: &BTreeMap<String, String>,
    ) -> anyhow::Result<CommandLine> {
        // determine the command line to execute
        let command_line = self.function.execution.get_command_line()?;

        // interpolate the arguments
        let command_line = {
            let mut interpolated = Vec::new();
            for arg in command_line {
                interpolated.push(if let Some(caps) = ARG_VALUE_PARSER.captures(&arg) {
                    let var_name = caps
                        .get(1)
                        .ok_or(ARG_EXPRESSION_ERROR)
                        .map_err(|e| anyhow!(e))?
                        .as_str();

                    if let Some(value) = arguments.get(var_name) {
                        value.to_string()
                    } else if let Some(default_value) = caps.get(3).map(|m| m.as_str()) {
                        default_value.to_string()
                    } else {
                        return Err(anyhow::anyhow!("argument {} not provided", var_name));
                    }
                } else {
                    arg
                });
            }
            interpolated
        };

        // final parsing
        CommandLine::from_vec(&command_line)
    }
}
