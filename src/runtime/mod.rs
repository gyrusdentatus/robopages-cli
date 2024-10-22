use std::sync::Arc;

use serde::{Deserialize, Serialize};

use crate::book::{openai, Book};

mod docker;
mod cmd;

pub(crate) mod prompt;

pub(crate) use cmd::CommandLine;


#[derive(Debug, Serialize, Deserialize)]
pub enum ContainerSource {
    #[serde(rename = "image")]
    Image(String),
    #[serde(rename = "build")]
    Build { name: String, path: String },
}

impl ContainerSource {
    pub async fn resolve(&self) -> anyhow::Result<()> {
        match self {
            Self::Image(image) => docker::pull_image(image).await,
            Self::Build { name, path } => docker::build_image(name, path).await,
        }
    }

    pub fn image(&self) -> &str {
        match self {
            Self::Image(image) => image,
            Self::Build { name, .. } => name,
        }
    }
}


// TODO: make sure parallelism is respected

// TODO: add unit tests for validation
pub(crate) async fn execute_call(
    interactive: bool,
    book: Arc<Book>,
    call: openai::Call,
) -> anyhow::Result<openai::CallResultMessage> {
    log::debug!("processing call: {:?}", call);

    let function = book.get_function(&call.function.name)?;

    log::debug!("{} resolved to: {:?}", &call.function.name, &function);

    // fail early if the arguments are invalid
    function.validate_arguments(&call.function.arguments)?;

    log::debug!("arguments validated");

    let command_line = function.resolve_command_line(&call.function.arguments)?;

    log::debug!("command line: {:?}", command_line);

    // validate runtime requirements
    let container = function.function.container.as_ref();
    let needs_container = 
        // we're running in non-interactive mode, can't sudo
        (command_line.sudo && !interactive) || 
        // app not in $PATH
        !command_line.app_in_path ||
        // forced container use
        (container.is_some() && container.unwrap().force);

    let command_line = if needs_container {
        let container = match container {
            Some(c) => c,
            None => return Err(anyhow::anyhow!(
                "container required for function {}", call.function.name
            )),
        };

        log::debug!("using container: {:?}", container);

        // build or pull the image if needed
        container.source.resolve().await?;

        // wrap the command line
        container.wrap(command_line)?
    } else {
        // keep it as it is
        command_line
    };

    log::warn!("executing: {}", &command_line);

    if interactive {
        if prompt::ask(">> enter 'y' to proceed or any other key to cancel: ", &["y", "n"])?  != "y" {
            return Ok(openai::CallResultMessage {
                role: "tool".to_string(),
                call_id: call.id.clone(),
                content: "<command execution cancelled by user>".to_string(),
            })
        }
    }
    
    // finally execute the command line
    let content = command_line.execute().await?;
    Ok(openai::CallResultMessage {
        role: "tool".to_string(),
        call_id: call.id.clone(),
        content,
    })
}

pub(crate) async fn execute(
    interactive: bool,
    book: Arc<Book>,
    calls: Vec<openai::Call>,
) -> anyhow::Result<Vec<openai::CallResultMessage>> {
    let mut results = Vec::new();

    for call in calls {
        results.push(execute_call(interactive, book.clone(), call).await?);
    }

    Ok(results)
}
