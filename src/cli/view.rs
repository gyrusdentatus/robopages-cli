use camino::Utf8PathBuf;
use comfy_table::Table;

use crate::book::Book;

pub(crate) async fn view(path: Utf8PathBuf, filter: Option<String>) -> anyhow::Result<()> {
    let book = Book::from_path(path, filter)?;

    let mut table = Table::new();

    table.set_header(vec!["page", "function", "description"]);

    for (_, page) in book.pages {
        let mut first_page = true;
        for (function_name, function) in page.functions {
            if first_page {
                table.add_row(vec![
                    format!("{} > {}", page.categories.join(" > "), &page.name),
                    function_name,
                    function.description,
                ]);
                first_page = false;
            } else {
                table.add_row(vec!["".to_owned(), function_name, function.description]);
            }
        }
    }

    println!("\n{}", table);

    Ok(())
}
