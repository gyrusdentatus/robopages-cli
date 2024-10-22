use std::collections::BTreeMap;

use camino::Utf8PathBuf;

use crate::book::{runtime::ExecutionContext, Function, Page, Parameter};

pub(crate) async fn create(name: Utf8PathBuf) -> anyhow::Result<()> {
    if name.exists() {
        return Err(anyhow::anyhow!("{:?} already exists", name));
    }

    // TODO: interactive mode asking for a template, the function name, description, parameters, etc.

    log::info!("creating {:?}", name);

    let mut parameters = BTreeMap::new();
    parameters.insert(
        "foo".to_string(),
        Parameter {
            param_type: "string".to_string(),
            description: "An example paramter named foo.".to_string(),
            required: true,
            examples: Some(vec!["bar".to_string(), "baz".to_string()]),
        },
    );

    let mut functions = BTreeMap::new();
    functions.insert(
        "example_function_name".to_string(),
        Function {
            description: "This is an example function describing a command line.".to_string(),
            parameters,
            container: None,
            execution: ExecutionContext::CommandLine(vec![
                "echo".to_string(),
                "${foo}".to_string(),
            ]),
        },
    );

    let page = Page {
        description: Some("You can use this for a description.".to_string()),
        functions,
        categories: Vec::new(),
        name: String::new(),
    };

    let yaml = serde_yaml::to_string(&page)?;
    std::fs::write(&name, yaml)?;

    Ok(())
}
