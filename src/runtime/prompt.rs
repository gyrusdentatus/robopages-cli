use std::io::Write;

pub(crate) fn ask<'a>(prompt: &str, choices: &'a [&'a str]) -> anyhow::Result<String> {
    loop {
        print!("{}", prompt);
        std::io::stdout().flush()?;

        let mut user_input = String::new();
        std::io::stdin().read_line(&mut user_input)?;
        println!();

        let choice = user_input.trim().to_lowercase();
        if choices.is_empty() || choices.contains(&choice.as_str()) {
            return Ok(choice);
        } else {
            log::error!("valid choices are: {}", choices.join(", "));
        }
    }
}
