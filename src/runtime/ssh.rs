use std::{borrow::Cow, path::PathBuf};

use async_ssh2_tokio::{AuthMethod, Client, ServerCheckMethod};

#[derive(Debug, Clone)]
pub struct SSHConnection {
    host: String,
    port: u16,
    user: String,
    auth_method: AuthMethod,
}

impl SSHConnection {
    pub fn from_str(s: &str, public_key: &str, passphrase: Option<String>) -> anyhow::Result<Self> {
        let mut user = std::env::var("USER").unwrap_or_else(|_| "root".to_string());
        let host;
        let mut port = 22;

        if s.is_empty() {
            return Err(anyhow::anyhow!("SSH connection string cannot be empty"));
        }

        // split on @ first to separate user if present
        let parts: Vec<&str> = s.split('@').collect();
        match parts.len() {
            // only.host or only.host:port
            1 => {
                let host_parts: Vec<&str> = parts[0].split(':').collect();
                match host_parts.len() {
                    1 => host = host_parts[0].to_string(),
                    2 => {
                        host = host_parts[0].to_string();
                        port = host_parts[1].parse()?;
                    }
                    _ => return Err(anyhow::anyhow!("invalid host format")),
                }
            }
            // user@host or user@host:port
            2 => {
                user = parts[0].to_string();
                let host_parts: Vec<&str> = parts[1].split(':').collect();
                match host_parts.len() {
                    1 => host = host_parts[0].to_string(),
                    2 => {
                        host = host_parts[0].to_string();
                        port = host_parts[1].parse()?;
                    }
                    _ => return Err(anyhow::anyhow!("invalid host format")),
                }
            }
            _ => return Err(anyhow::anyhow!("invalid SSH connection string format")),
        }

        let public_key = shellexpand::full(public_key)?.to_string();
        let public_key = PathBuf::from(public_key);
        if !public_key.exists() {
            return Err(anyhow::anyhow!(
                "public key file {} does not exist",
                public_key.display()
            ));
        }
        let public_key = public_key.canonicalize()?.to_string_lossy().to_string();

        let auth_method = AuthMethod::with_key_file(&public_key, passphrase.as_deref());

        Ok(Self {
            host,
            port,
            user,
            auth_method,
        })
    }

    async fn client(&self) -> anyhow::Result<Client> {
        Client::connect(
            (self.host.as_str(), self.port),
            self.user.as_str(),
            self.auth_method.clone(),
            ServerCheckMethod::NoCheck,
        )
        .await
        .map_err(|e| anyhow::anyhow!("failed to connect to SSH server: {:?}", e))
    }

    fn create_command_line(with_sudo: bool, app: &str, args: &Vec<String>) -> String {
        let mut command = String::new();
        if with_sudo {
            command.push_str("sudo ");
        }

        command.push_str(&shell_escape::escape(Cow::Borrowed(app)));

        for arg in args {
            command.push(' ');
            command.push_str(&shell_escape::escape(Cow::Borrowed(arg)));
        }

        command
    }

    pub(crate) async fn execute(
        &self,
        with_sudo: bool,
        app: &str,
        args: &Vec<String>,
    ) -> anyhow::Result<String> {
        let command_line = Self::create_command_line(with_sudo, app, args);
        let result = self.client().await?.execute(&command_line).await?;

        let mut parts = vec![];

        if result.exit_status != 0 {
            parts.push(format!("EXIT CODE: {}", &result.exit_status));
        }

        if !result.stdout.is_empty() {
            parts.push(result.stdout.to_string());
        }

        if !result.stderr.is_empty() {
            if result.exit_status == 0 {
                parts.push(result.stderr.to_string());
            } else {
                parts.push(format!("ERROR: {}", result.stderr));
            }
        }

        Ok(parts.join("\n"))
    }

    pub(crate) async fn test_connection(&self) -> anyhow::Result<()> {
        log::info!("testing ssh connection to {}:{} ...", self.host, self.port);
        let result = self.client().await?.execute("echo robopages").await?;
        if result.exit_status != 0 {
            return Err(anyhow::anyhow!("failed to execute command: {:?}", result));
        } else if result.stdout != "robopages\n" {
            return Err(anyhow::anyhow!("unexpected output: {:?}", result));
        }

        Ok(())
    }
}

impl std::fmt::Display for SSHConnection {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}@{}:{}", self.user, self.host, self.port)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_from_str_host_only() {
        let conn = SSHConnection::from_str("example.com", "/dev/null", None).unwrap();
        assert_eq!(conn.host, "example.com");
        assert_eq!(conn.port, 22);
        assert_eq!(
            conn.user,
            std::env::var("USER").unwrap_or_else(|_| "root".to_string())
        );
    }

    #[test]
    fn test_from_str_host_and_port() {
        let conn = SSHConnection::from_str("example.com:2222", "/dev/null", None).unwrap();
        assert_eq!(conn.host, "example.com");
        assert_eq!(conn.port, 2222);
        assert_eq!(
            conn.user,
            std::env::var("USER").unwrap_or_else(|_| "root".to_string())
        );
    }

    #[test]
    fn test_from_str_user_and_host() {
        let conn = SSHConnection::from_str("testuser@example.com", "/dev/null", None).unwrap();
        assert_eq!(conn.host, "example.com");
        assert_eq!(conn.port, 22);
        assert_eq!(conn.user, "testuser");
    }

    #[test]
    fn test_from_str_full() {
        let conn = SSHConnection::from_str("testuser@example.com:2222", "/dev/null", None).unwrap();
        assert_eq!(conn.host, "example.com");
        assert_eq!(conn.port, 2222);
        assert_eq!(conn.user, "testuser");
    }

    #[test]
    fn test_from_str_empty() {
        assert!(SSHConnection::from_str("", "/dev/null", None).is_err());
    }

    #[test]
    fn test_from_str_invalid_port() {
        assert!(SSHConnection::from_str("example.com:invalid", "/dev/null", None).is_err());
    }

    #[test]
    fn test_from_str_invalid_format() {
        assert!(SSHConnection::from_str("user@host@extra", "/dev/null", None).is_err());
        assert!(SSHConnection::from_str("host:port:extra", "/dev/null", None).is_err());
    }

    #[test]
    fn test_from_str_nonexistent_key() {
        assert!(SSHConnection::from_str("example.com", "/nonexistent/key/path", None).is_err());
    }
}
