use std::convert::{TryFrom, TryInto};
use std::fmt::{Debug, Display};
use std::path::PathBuf;

use config::builder::DefaultState;
use config::ConfigBuilder;
pub use database::*;
pub use error::*;
use serde::de::DeserializeOwned;

pub mod database;
pub mod error;

#[allow(dead_code)]
const ENV_APP_ENVIRONMENT: &'static str = "APP_ENVIRONMENT";
#[allow(dead_code)]
const RESOURCES_DIR: &'static str = "resources";
#[allow(dead_code)]
const APP_CONFIG: &'static str = "application";

pub trait LoadingOptions: Debug {
    fn config_path(&self) -> Option<PathBuf>;
    fn secrets_path(&self) -> Option<PathBuf>;
}

#[derive(Debug, Display, PartialEq)]
enum Environment {
    Local,
    Production,
}

impl AsRef<str> for Environment {
    fn as_ref(&self) -> &str {
        match self {
            Environment::Local => "local",
            Environment::Production => "production",
        }
    }
}

impl TryFrom<String> for Environment {
    type Error = SettingsError;

    fn try_from(value: String) -> Result<Self, Self::Error> {
        match value.to_lowercase().as_str() {
            "local" => Ok(Self::Local),
            "production" => Ok(Self::Production),
            other => Err(SettingsError::Bootstrap {
                message: format!("{} environment unrecognized", other),
                setting: "environment identification".to_string(),
            }),
        }
    }
}

pub trait SettingsLoader: Debug + Sized {
    type Options: LoadingOptions;

    #[tracing::instrument(level = "info")]
    fn load(options: Self::Options) -> Result<Self, SettingsError>
    where
        Self: DeserializeOwned,
    {
        tracing::info!(?options, "loading settings based on CLI options.");
        let mut config_builder = config::Config::builder();
        config_builder = Self::load_configuration(config_builder, options.config_path())?;
        config_builder = Self::load_secrets(config_builder, options.secrets_path());
        config_builder = Self::load_environment(config_builder);
        let config = config_builder.build()?;
        tracing::info!(?config, "configuration loaded");
        let settings = config.try_into()?;
        tracing::info!(?settings, "settings built for application.");
        Ok(settings)
    }

    #[tracing::instrument(level = "info", skip(config,))]
    fn load_configuration(
        config: ConfigBuilder<DefaultState>, specific_config_path: Option<PathBuf>,
    ) -> Result<ConfigBuilder<DefaultState>, SettingsError> {
        match specific_config_path {
            Some(explicit_path) => {
                // match FileSourceFile::new(explicit_path).resolve(None) {
                //     Ok((uri, contents, format)) => {
                //         tracing::info!(%contents, "adding explicit configuration {:?} source from {:?}", format, uri);
                //     },
                //     Err(err) => {
                //         tracing::error!(error=?err, "cannot find explicit configuration source at {:?}", explicit_path);
                //     }
                // }

                let config = config.add_source(config::File::from(explicit_path).required(true));
                Ok(config)
            }

            None => {
                let resources_path = std::env::current_dir()?.join(RESOURCES_DIR);
                let config_path = resources_path.join(APP_CONFIG);
                tracing::debug!("looking for {} config in: {:?}", APP_CONFIG, resources_path);
                // match FileSourceFile::new(config_path).resolve(None) {
                //     Ok((uri, contents, format)) => {
                //         tracing::info!(%contents, "adding base configuration {:?} source from {:?}", format, uri);
                //     },
                //     Err(err) => {
                //         tracing::warn!(error=?err, "cannot find base configuration source at {:?}", config_path);
                //     }
                // }
                let config = config.add_source(
                    // config::File::new(config_path.to_string_lossy().as_ref(), config::FileFormat::Yaml, )
                    config::File::with_name(config_path.to_string_lossy().as_ref()).required(true),
                );

                match std::env::var(ENV_APP_ENVIRONMENT) {
                    Ok(rep) => {
                        let environment: Environment = rep.try_into()?;
                        let env_config_path = resources_path.join(environment.as_ref());
                        tracing::debug!("looking for {} config in: {:?}", environment, resources_path);
                        // match FileSourceFile::new(env_config_path).resolve(None) {
                        //     Ok((uri, contents, format)) => {
                        //         tracing::info!(%contents, "adding {} environment override configuration {:?} source from {:?}", environment, format, uri);
                        //     },
                        //     Err(err) => {
                        //         tracing::warn!(error=?err, "cannot find {} environment configuration source at {:?}", environment, env_config_path);
                        //     }
                        // }
                        let config = config.add_source(
                            config::File::with_name(env_config_path.to_string_lossy().as_ref()).required(true),
                        );
                        Ok(config)
                    }

                    Err(std::env::VarError::NotPresent) => {
                        tracing::warn!(
                            "no environment variable override on settings specified at env var, {}",
                            ENV_APP_ENVIRONMENT
                        );
                        Ok(config)
                    }

                    Err(err) => Err(err.into()),
                }
            }
        }
    }

    #[tracing::instrument(level = "info", skip(config))]
    fn load_secrets(config: ConfigBuilder<DefaultState>, secrets_path: Option<PathBuf>) -> ConfigBuilder<DefaultState> {
        if let Some(path) = secrets_path {
            tracing::debug!(
                "looking for secrets configuration at: {:?} -- exists:{}",
                path,
                path.as_path().exists()
            );
            if path.as_path().exists() {
                tracing::info!("adding secrets override configuration source from {:?}", path);
            } else {
                tracing::error!("cannot find secrets override configuration at {:?}", path);
            }
            config.add_source(config::File::from(path).required(true))
        } else {
            config
        }
    }

    #[tracing::instrument(level = "info", skip(config))]
    fn load_environment(config: ConfigBuilder<DefaultState>) -> ConfigBuilder<DefaultState> {
        let config_env = config::Environment::with_prefix("app").separator("__");
        tracing::info!("loading environment properties with prefix: {:?}", config_env);
        config.add_source(config_env)
    }
}
