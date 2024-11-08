use std::{collections::BTreeMap, sync::Arc};

use crate::{
    book::{flavors::openai, Book},
    runtime::{self, prompt, ssh::SSHConnection},
};

use super::RunArgs;

pub(crate) async fn run(args: RunArgs) -> anyhow::Result<()> {
    // parse and validate SSH connection string if provided
    let ssh = if let Some(ssh_str) = args.ssh {
        // parse
        let conn = SSHConnection::from_str(&ssh_str, &args.ssh_key, args.ssh_key_passphrase)?;
        // make sure we can connect
        conn.test_connection().await?;

        Some(conn)
    } else {
        None
    };

    let book = Arc::new(Book::from_path(args.path, None)?);
    let function = book.get_function(&args.function)?;

    let mut arguments = BTreeMap::new();

    // convert defines to BTreeMap
    let defines: BTreeMap<String, String> = args.defines.into_iter().collect();

    for arg_name in function.function.parameters.keys() {
        if let Some(value) = defines.get(arg_name) {
            arguments.insert(arg_name.to_string(), value.to_string());
        } else {
            arguments.insert(
                arg_name.to_string(),
                prompt::ask(
                    &format!(">> enter value for argument '{}': ", arg_name),
                    &[],
                )?,
            );
        }
    }

    let call = openai::Call {
        id: None,
        function: openai::FunctionCall {
            name: args.function,
            arguments,
        },
        call_type: "function".to_string(),
    };

    let result = runtime::execute_call(ssh, !args.auto, 10, book, call).await?;

    println!("\n{}", result.content);

    Ok(())
}
