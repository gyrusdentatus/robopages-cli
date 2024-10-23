use std::fmt;

#[derive(Debug)]
pub struct CommandLine {
    pub sudo: bool,
    pub app: String,
    pub app_in_path: bool,
    pub args: Vec<String>,
}

impl CommandLine {
    pub fn from_vec(vec: &Vec<String>) -> anyhow::Result<Self> {
        if vec.is_empty() {
            return Err(anyhow::anyhow!("empty command line"));
        }

        let mut sudo = false;
        let mut app = String::new();
        let mut args = Vec::new();
        let mut app_in_path = false;

        for arg in vec {
            if arg == "sudo" {
                sudo = true;
            } else if app.is_empty() {
                app = arg.to_string();
            } else {
                args.push(arg.to_string());
            }
        }

        if app.is_empty() {
            return Err(anyhow::anyhow!(
                "could not determine application name from command line: {:?}",
                vec
            ));
        }

        if let Ok(path) = which::which(&app) {
            app_in_path = true;
            app = path.to_string_lossy().to_string();
        } else {
            app_in_path = false;
        }

        Ok(Self {
            sudo,
            app,
            args,
            app_in_path,
        })
    }

    pub async fn execute(&self) -> anyhow::Result<String> {
        let output = tokio::process::Command::new(&self.app)
            .args(&self.args)
            .output()
            .await?;

        let mut parts = vec![];

        let stdout = String::from_utf8_lossy(&output.stdout);
        let stderr = String::from_utf8_lossy(&output.stderr);

        if !output.status.success() {
            parts.push(format!("EXIT CODE: {}", &output.status));
        }

        if !stdout.is_empty() {
            parts.push(stdout.to_string());
        }

        if !stderr.is_empty() {
            if output.status.success() {
                parts.push(stderr.to_string());
            } else {
                parts.push(format!("ERROR: {}", stderr));
            }
        }

        Ok(parts.join("\n"))
    }
}

impl fmt::Display for CommandLine {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let mut command = String::new();

        if self.sudo {
            command.push_str("sudo ");
        }

        command.push_str(&self.app);

        for arg in &self.args {
            command.push(' ');
            command.push_str(arg);
        }

        write!(f, "{}", command)
    }
}
