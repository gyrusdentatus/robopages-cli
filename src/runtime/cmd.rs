use std::{collections::BTreeMap, fmt};

#[derive(Debug)]
pub struct CommandLine {
    pub sudo: bool,
    pub app: String,
    pub app_in_path: bool,
    pub args: Vec<String>,
    pub env: BTreeMap<String, String>,

    // used to keep a valid reference to this while the command is running
    pub temp_env_file: Option<tempfile::NamedTempFile>,
}

impl CommandLine {
    pub fn from_vec(vec: &Vec<String>) -> anyhow::Result<Self> {
        if vec.is_empty() {
            return Err(anyhow::anyhow!("empty command line"));
        }

        let mut sudo = false;
        let mut app = String::new();
        let mut args = Vec::new();

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

        let app_in_path = if let Ok(path) = which::which(&app) {
            app = path.to_string_lossy().to_string();
            true
        } else {
            false
        };

        Ok(Self {
            sudo,
            app,
            args,
            app_in_path,
            env: BTreeMap::new(),
            temp_env_file: None,
        })
    }

    pub fn from_vec_with_env(
        vec: &Vec<String>,
        env: BTreeMap<String, String>,
    ) -> anyhow::Result<Self> {
        let mut cmd = Self::from_vec(vec)?;
        cmd.env = env;
        Ok(cmd)
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_command_line_display() {
        let cmd = CommandLine {
            sudo: false,
            app: "ls".to_string(),
            args: vec!["-l".to_string(), "-a".to_string()],
            app_in_path: true,
            env: BTreeMap::new(),
            temp_env_file: None,
        };
        assert_eq!(format!("{}", cmd), "ls -l -a");

        let cmd_with_sudo = CommandLine {
            sudo: true,
            app: "apt".to_string(),
            args: vec!["install".to_string(), "package".to_string()],
            app_in_path: true,
            env: BTreeMap::new(),
            temp_env_file: None,
        };
        assert_eq!(format!("{}", cmd_with_sudo), "sudo apt install package");
    }

    #[tokio::test]
    async fn test_command_line_execute_success() {
        let cmd = CommandLine {
            sudo: false,
            app: "echo".to_string(),
            args: vec!["-n".to_string(), "Hello, World!".to_string()],
            app_in_path: true,
            env: BTreeMap::new(),
            temp_env_file: None,
        };
        let result = cmd.execute().await.unwrap();
        assert_eq!(result, "Hello, World!");
    }

    #[tokio::test]
    async fn test_command_line_execute_failure() {
        let cmd = CommandLine {
            sudo: false,
            app: "ls".to_string(),
            args: vec!["nonexistent_file".to_string()],
            app_in_path: true,
            env: BTreeMap::new(),
            temp_env_file: None,
        };
        let result = cmd.execute().await.unwrap();
        assert!(result.contains("EXIT CODE:"));
        assert!(result.contains("ERROR:"));
    }

    #[tokio::test]
    async fn test_command_line_execute_with_stderr() {
        let cmd = CommandLine {
            sudo: false,
            app: "sh".to_string(),
            args: vec![
                "-c".to_string(),
                "echo 'Hello' && echo 'Error' >&2".to_string(),
            ],
            app_in_path: true,
            env: BTreeMap::new(),
            temp_env_file: None,
        };
        let result = cmd.execute().await.unwrap();
        assert!(result.contains("Hello"));
        assert!(result.contains("Error"));
    }

    #[tokio::test]
    async fn test_command_line_empty_app() {
        let cmd = CommandLine {
            sudo: false,
            app: "".to_string(),
            args: vec!["arg1".to_string(), "arg2".to_string()],
            app_in_path: true,
            env: BTreeMap::new(),
            temp_env_file: None,
        };
        let result = cmd.execute().await;
        assert!(result.is_err());
    }
}
