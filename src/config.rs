//! Layered configuration loading.
//!
//! Layering order (lowest → highest precedence):
//!
//! 1. compiled-in defaults — the seed value the caller passes to [`load`].
//! 2. `service.toml` in the process's working directory, if present.
//! 3. environment variables prefixed `<SERVICE_NAME>_` (the service name
//!    upper-cased, non-alphanumeric characters replaced with `_`), e.g.
//!    `MYSVC_PORT=9000` overrides `port` for a service named `mysvc`.
//!
//! [`LayeredConfig`] has no required schema beyond `serde` — every service
//! defines its own config struct and seeds it with defaults (`Default` or
//! any other constructor) before calling [`load`].

use figment::{
    providers::{Env, Format, Serialized, Toml},
    Figment,
};
use serde::{de::DeserializeOwned, Serialize};

use crate::AppError;

/// Marker trait for config types loadable through [`load`]. Blanket
/// implementation — any `Serialize + DeserializeOwned` type qualifies.
pub trait LayeredConfig: DeserializeOwned + Serialize {}

impl<T> LayeredConfig for T where T: DeserializeOwned + Serialize {}

/// Load a `T`, layering `defaults` under `service.toml` under
/// `<SERVICE_NAME>_`-prefixed environment variables. See the module docs
/// for precedence.
pub fn load<T: LayeredConfig>(service_name: &str, defaults: T) -> Result<T, AppError> {
    Figment::from(Serialized::defaults(defaults))
        .merge(Toml::file("service.toml"))
        .merge(Env::prefixed(&env_prefix(service_name)))
        .extract()
        .map_err(|e| AppError::Config(e.to_string()))
}

/// `<SERVICE_NAME>_` env-var prefix for a given service name: upper-cased,
/// with non-alphanumeric characters replaced by `_`.
fn env_prefix(service_name: &str) -> String {
    let mut prefix: String = service_name
        .chars()
        .map(|c| {
            if c.is_ascii_alphanumeric() {
                c.to_ascii_uppercase()
            } else {
                '_'
            }
        })
        .collect();
    prefix.push('_');
    prefix
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde::{Deserialize, Serialize};

    #[derive(Debug, Deserialize, Serialize, PartialEq)]
    struct TestConfig {
        port: u16,
        name: String,
    }

    impl Default for TestConfig {
        fn default() -> Self {
            Self {
                port: 8080,
                name: "default".into(),
            }
        }
    }

    #[test]
    fn env_prefix_normalizes_service_name() {
        assert_eq!(env_prefix("my-svc"), "MY_SVC_");
        assert_eq!(env_prefix("MySvc"), "MYSVC_");
    }

    #[test]
    fn load_falls_back_to_defaults_with_no_overrides() {
        // Service name unlikely to collide with any real env var.
        let cfg: TestConfig = load("svckit_test_defaults_only", TestConfig::default()).unwrap();
        assert_eq!(cfg, TestConfig::default());
    }

    #[test]
    fn env_var_overrides_default() {
        let prefix = env_prefix("svckit_test_env_override");
        let key = format!("{prefix}PORT");

        // SAFETY: test-only env var with a key unique to this test; no
        // other test reads or writes it.
        unsafe {
            std::env::set_var(&key, "9999");
        }

        let result: Result<TestConfig, AppError> =
            load("svckit_test_env_override", TestConfig::default());

        unsafe {
            std::env::remove_var(&key);
        }

        let cfg = result.unwrap();
        assert_eq!(cfg.port, 9999, "env override should win over the default");
        assert_eq!(cfg.name, "default", "untouched fields keep their default");
    }
}
