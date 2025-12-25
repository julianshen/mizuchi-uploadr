//! Configuration loader with environment variable expansion

use super::{Config, ConfigError};
use std::path::Path;

/// Configuration loader
pub struct ConfigLoader;

impl ConfigLoader {
    /// Load configuration from a file
    pub fn load<P: AsRef<Path>>(path: P) -> Result<Config, ConfigError> {
        let content = std::fs::read_to_string(path)?;
        let expanded = Self::expand_env_vars(&content);
        let config: Config = serde_yaml::from_str(&expanded)?;
        config.validate()?;
        Ok(config)
    }

    /// Expand environment variables in the format ${VAR_NAME}
    fn expand_env_vars(content: &str) -> String {
        let mut result = content.to_string();
        let re = regex_lite::Regex::new(r"\$\{([A-Z_][A-Z0-9_]*)\}").unwrap();

        for cap in re.captures_iter(content) {
            let var_name = &cap[1];
            if let Ok(value) = std::env::var(var_name) {
                result = result.replace(&cap[0], &value);
            }
        }

        result
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_expand_env_vars() {
        std::env::set_var("TEST_VAR", "test_value");
        let content = "key: ${TEST_VAR}";
        let expanded = ConfigLoader::expand_env_vars(content);
        assert_eq!(expanded, "key: test_value");
        std::env::remove_var("TEST_VAR");
    }
}
