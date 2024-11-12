use crate::book::{runtime::ExecutionFlavor, Book};

use super::ViewArgs;

pub(crate) async fn view(args: ViewArgs) -> anyhow::Result<()> {
    let book = Book::from_path(args.path, args.filter)?;

    for (_, page) in book.pages {
        println!("{} > [{}]", page.categories.join(" > "), page.name);

        for (function_name, function) in page.functions {
            println!("    * {} : {}", function_name, function.description);
            println!(
                "         running with: {}",
                ExecutionFlavor::for_function(&function)?
            );
            println!("         parameters:");
            for (parameter_name, parameter) in &function.parameters {
                println!("            {} : {}", parameter_name, parameter.description);
            }

            println!();
        }
    }

    Ok(())
}
