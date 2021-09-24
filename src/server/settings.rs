use std::path::PathBuf;

use clap::{AppSettings, Clap};
use serde::{Deserialize, Serialize};

use http_server::ApplicationSettings;

pub use crate::core::settings::error::*;
use crate::core::settings::{DatabaseSettings, LoadingOptions, SettingsLoader};

pub mod http_server;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Settings {
    pub application: ApplicationSettings,
    pub database: DatabaseSettings,
}

impl SettingsLoader for Settings {
    type Options = HttpServerCliOptions;
}

#[derive(Clap, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[clap(version = "0.1.0", author = "Damon Rolfs")]
#[clap(setting = AppSettings::ColoredHelp)]
pub struct HttpServerCliOptions {
    /// override environment-based configuration file to load.
    /// Default behavior is to load configuration based on `APP_ENVIRONMENT` envvar.
    #[clap(short, long)]
    config: Option<PathBuf>,

    /// specify path to secrets configuration file
    #[clap(short, long)]
    secrets: Option<PathBuf>,
}

impl LoadingOptions for HttpServerCliOptions {
    fn config_path(&self) -> Option<PathBuf> {
        self.config.clone()
    }

    fn secrets_path(&self) -> Option<PathBuf> {
        self.secrets.clone()
    }
}

#[cfg(test)]
mod tests {
    use claim::assert_ok;
    use config::{Config, FileFormat};
    use pretty_assertions::assert_eq;

    use super::*;

    #[test]
    fn test_load_string_settings() -> anyhow::Result<()> {
        lazy_static::initialize(&crate::tracing::TEST_TRACING);
        let main_span = tracing::info_span!("test_load_string_settings");
        let _ = main_span.enter();

        let config = assert_ok!(Config::builder()
            .add_source(config::File::from_str(
                r###"
application:
  port: 8000
  host: 10.1.2.57
  base_url: "http://10.1.2.57"
database:
  username: postgres
  password: password
  port: 5432
  host: "localhost"
  database_name: "propensity"
  require_ssl: true
                "###,
                FileFormat::Yaml
            ))
            .build());

        tracing::info!(?config, "eligibility config loaded.");

        let actual: Settings = assert_ok!(config.try_into());
        assert_eq!(
            actual,
            Settings {
                application: ApplicationSettings {
                    port: 8000,
                    host: "10.1.2.57".to_string(),
                    base_url: "http://10.1.2.57".to_string(),
                },
                database: DatabaseSettings {
                    username: "postgres".to_string(),
                    password: "password".to_string(),
                    port: 5432,
                    host: "localhost".to_string(),
                    database_name: "propensity".to_string(),
                    require_ssl: true,
                },
            }
        );
        Ok(())
    }

    #[test]
    fn test_settings_load() -> anyhow::Result<()> {
        lazy_static::initialize(&crate::tracing::TEST_TRACING);
        let main_span = tracing::info_span!("test_settings_applications_load");
        let _ = main_span.enter();

        std::env::set_var("APP_ENVIRONMENT", "local");
        tracing::info!("envar: APP_ENVIRONMENT = {:?}", std::env::var("APP_ENVIRONMENT"));

        let mut builder = config::Config::builder();
        builder = assert_ok!(Settings::load_configuration(builder, None));
        builder = Settings::load_secrets(builder, Some("./resources/secrets.yaml".into()));
        let c = assert_ok!(builder.build());
        tracing::info!(config=?c, "loaded configuration file");
        let actual: Settings = assert_ok!(c.try_into());

        let expected = Settings {
            application: ApplicationSettings {
                port: 8000,
                host: "127.0.0.1".to_string(),
                base_url: "http://127.0.0.1".to_string(),
            },
            database: DatabaseSettings {
                username: "postgres".to_string(),
                password: "password".to_string(),
                port: 5432,
                host: "localhost".to_string(),
                database_name: "propensity".to_string(),
                require_ssl: false,
            },
        };

        assert_eq!(actual, expected);
        Ok(())
    }
}
