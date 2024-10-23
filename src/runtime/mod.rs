use std::sync::{atomic::AtomicUsize, Arc};

use crate::book::{openai, Book};

mod cmd;
mod docker;

pub(crate) mod prompt;

pub(crate) use cmd::CommandLine;
pub(crate) use docker::ContainerSource;

static ACTIVE_TASKS: AtomicUsize = AtomicUsize::new(0);

// https://stackoverflow.com/questions/29963449/golang-like-defer-in-rust
struct ScopeCall<F: FnOnce()> {
    c: Option<F>,
}
impl<F: FnOnce()> Drop for ScopeCall<F> {
    fn drop(&mut self) {
        self.c.take().unwrap()()
    }
}

macro_rules! expr {
    ($e: expr) => {
        $e
    };
} // tt hack
macro_rules! defer {
    ($($data: tt)*) => (
        let _scope_call = ScopeCall {
            c: Some(|| -> () { expr!({ $($data)* }) })
        };
    )
}

async fn wait_for_available_tasks(max_running_tasks: usize) {
    let wait = std::time::Duration::from_secs(1);
    loop {
        let active_tasks = ACTIVE_TASKS.load(std::sync::atomic::Ordering::Relaxed);
        if active_tasks < max_running_tasks {
            break;
        }

        log::debug!("waiting for our turn, {} active tasks", active_tasks);
        tokio::time::sleep(wait).await;
    }
}

// TODO: add unit tests for validation
pub(crate) async fn execute_call(
    interactive: bool,
    max_running_tasks: usize,
    book: Arc<Book>,
    call: openai::Call,
) -> anyhow::Result<openai::CallResultMessage> {
    wait_for_available_tasks(max_running_tasks).await;

    // increment the active tasks counter
    ACTIVE_TASKS.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
    defer! {
        log::debug!("decrementing active tasks counter");
        ACTIVE_TASKS.fetch_sub(1, std::sync::atomic::Ordering::Relaxed);
    }

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
            None => {
                return Err(anyhow::anyhow!(
                    "container required for function {}",
                    call.function.name
                ))
            }
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

    if interactive
        && prompt::ask(
            ">> enter 'y' to proceed or any other key to cancel: ",
            &["y", "n"],
        )? != "y"
    {
        return Ok(openai::CallResultMessage {
            role: "tool".to_string(),
            call_id: call.id.clone(),
            content: "<command execution cancelled by user>".to_string(),
        });
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
    max_running_tasks: usize,
) -> anyhow::Result<Vec<openai::CallResultMessage>> {
    let mut futures = Vec::new();
    for call in calls {
        futures.push(tokio::spawn(execute_call(
            interactive,
            max_running_tasks,
            book.clone(),
            call,
        )));
    }

    let mut results = Vec::new();
    for future_result in futures::future::join_all(futures).await {
        match future_result {
            Ok(result) => match result {
                Ok(result) => results.push(result),
                Err(e) => log::error!("error executing call: {:?}", e),
            },
            Err(e) => log::error!("error joining task: {:?}", e),
        }
    }

    Ok(results)
}
