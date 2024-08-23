use std::process::Command;
use std::str;
use thiserror::Error;
use serde_json::Value;
use teller::SecretProvider;
use async_trait::async_trait;

pub struct FlyIoProvider;

impl FlyIoProvider {
    pub fn new() -> Self {
        FlyIoProvider {}
    }

    async fn execute_fly_command(args: &[&str]) -> Result<String, FlyIoError> {
        let output = Command::new("fly")
            .args(args)
            .output()
            .map_err(FlyIoError::CommandError)?;

        if !output.status.success() {
            return Err(FlyIoError::CommandError(std::io::Error::new(
                std::io::ErrorKind::Other,
                "fly command failed",
            )));
        }

        let output_str = str::from_utf8(&output.stdout).map_err(FlyIoError::Utf8Error)?;
        Ok(output_str.to_string())
    }
}

#[derive(Error, Debug)]
pub enum FlyIoError {
    #[error("Failed to execute fly command")]
    CommandError(#[from] std::io::Error),
    #[error("Secret {0} not found or command failed")]
    SecretNotFound(String),
    #[error("Secret {0} could not be set")]
    SecretNotSet(String),
    #[error("Secret {0} could not be deleted")]
    SecretNotDeleted(String),
    #[error("Invalid UTF-8 sequence")]
    Utf8Error(#[from] std::str::Utf8Error),
}

#[async_trait]
impl SecretProvider for FlyIoProvider {
    type Error = FlyIoError;

    async fn get(&self, secret_name: &str) -> Result<String, Self::Error> {
        let output = FlyIoProvider::execute_fly_command(&["secrets", "list", "--json"]).await?;
        
        let secrets: Value = serde_json::from_str(&output).map_err(|_| FlyIoError::SecretNotFound(secret_name.to_string()))?;

        if let Some(secret_value) = secrets.get(secret_name) {
            Ok(secret_value.as_str().unwrap_or_default().to_string())
        } else {
            Err(FlyIoError::SecretNotFound(secret_name.to_string()))
        }
    }

    async fn put(&self, secret_name: &str, secret_value: &str) -> Result<(), Self::Error> {
        let result = FlyIoProvider::execute_fly_command(&["secrets", "set", &format!("{}={}", secret_name, secret_value)]).await;

        if result.is_ok() {
            Ok(())
        } else {
            Err(FlyIoError::SecretNotSet(secret_name.to_string()))
        }
    }

    async fn delete(&self, secret_name: &str) -> Result<(), Self::Error> {
        let result = FlyIoProvider::execute_fly_command(&["secrets", "unset", secret_name]).await;

        if result.is_ok() {
            Ok(())
        } else {
            Err(FlyIoError::SecretNotDeleted(secret_name.to_string()))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_get_secret_success() {
        let provider = FlyIoProvider::new();
        let result = provider.get("EXISTING_SECRET").await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_get_secret_not_found() {
        let provider = FlyIoProvider::new();
        let result = provider.get("NON_EXISTENT_SECRET").await;
        assert!(matches!(result, Err(FlyIoError::SecretNotFound(_))));
    }

    #[tokio::test]
    async fn test_put_secret_success() {
        let provider = FlyIoProvider::new();
        let result = provider.put("NEW_SECRET", "secret_value").await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_delete_secret_success() {
        let provider = FlyIoProvider::new();
        let result = provider.delete("NEW_SECRET").await;
        assert!(result.is_ok());
    }
}
