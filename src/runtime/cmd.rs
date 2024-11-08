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

impl Clone for CommandLine {
    fn clone(&self) -> Self {
        Self {
            sudo: self.sudo,
            app: self.app.clone(),
            app_in_path: self.app_in_path,
            args: self.args.clone(),
            env: self.env.clone(),
            temp_env_file: None,  // Don't clone the temp file, create a new one if needed
        }
    }
}

impl CommandLine {
    pub fn from_vec(vec: &Vec<String>) -> anyhow::Result<Self> {
        log::debug!("Creating CommandLine from vector: {:?}", vec);
        
        if vec.is_empty() {
            log::error!("Empty command line vector provided");
            return Err(anyhow::anyhow!("empty command line"));
        }

        let mut sudo = false;
        let mut app = String::new();
        let mut args = Vec::new();

        for arg in vec {
            log::trace!("Processing argument: {}", arg);
            if arg == "sudo" {
                log::debug!("Sudo flag detected");
                sudo = true;
            } else if app.is_empty() {
                log::debug!("Setting application name: {}", arg);
                app = arg.to_string();
            } else {
                log::trace!("Adding argument: {}", arg);
                args.push(arg.to_string());
            }
        }

        if app.is_empty() {
            log::error!("Could not determine application name from: {:?}", vec);
            return Err(anyhow::anyhow!(
                "could not determine application name from command line: {:?}",
                vec
            ));
        }

        let app_in_path = if let Ok(path) = which::which(&app) {
            log::debug!("Found application in path: {}", path.display());
            app = path.to_string_lossy().to_string();
            true
        } else {
            log::debug!("Application '{}' not found in PATH", app);
            false
        };

        log::debug!("Created CommandLine: sudo={}, app={}, app_in_path={}, args={:?}", 
            sudo, app, app_in_path, args);

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
        log::debug!("Creating CommandLine with environment variables");
        log::trace!("Environment variables: {:?}", env);
        let mut cmd = Self::from_vec(vec)?;
        cmd.env = env;
        Ok(cmd)
    }

    fn interpolate_variables(&mut self) -> anyhow::Result<()> {
        log::debug!("Interpolating variables from environment: {:?}", self.env);
        
        self.args = self.args.iter().map(|arg| {
            let mut result = arg.clone();
            for (key, value) in &self.env {
                let pattern = format!("${{{}}}", key);
                if result.contains(&pattern) {
                    log::debug!("Replacing {} with {}", pattern, value);
                    result = result.replace(&pattern, value);
                }
            }
            result
        }).collect();

        log::debug!("After interpolation: {:?}", self.args);
        Ok(())
    }

    pub async fn execute(&self) -> anyhow::Result<String> {
        log::info!("Executing command: {}", self);
        log::debug!("Full command details: {:?}", self);

        // Create a mutable copy for interpolation
        let mut cmd = self.clone();
        cmd.interpolate_variables()?;

        let mut command = tokio::process::Command::new(&cmd.app);
        command.args(&cmd.args);

        // Log environment variables if present
        if !cmd.env.is_empty() {
            log::debug!("Setting environment variables: {:?}", cmd.env);
            command.envs(&cmd.env);
        }

        let output = command.output().await?;
        log::debug!("Command completed with status: {:?}", output.status);

        let mut parts = vec![];

        let stdout = String::from_utf8_lossy(&output.stdout);
        let stderr = String::from_utf8_lossy(&output.stderr);

        if !output.status.success() {
            log::warn!("Command failed with exit code: {}", output.status);
            parts.push(format!("EXIT CODE: {}", &output.status));
        }

        if !stdout.is_empty() {
            log::trace!("Command stdout: {}", stdout);
            parts.push(stdout.to_string());
        }

        if !stderr.is_empty() {
            if output.status.success() {
                log::debug!("Command stderr (success): {}", stderr);
                parts.push(stderr.to_string());
            } else {
                log::error!("Command stderr (failure): {}", stderr);
                parts.push(format!("ERROR: {}", stderr));
            }
        }

        let result = parts.join("\n");
        log::debug!("Command execution completed, output length: {}", result.len());
        log::trace!("Command output: {}", result);

        Ok(result)
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
