use clap::{AppSettings, Clap, ValueHint};
use serde::{Deserialize, Serialize};
use settings_loader::common::database::DatabaseSettings;
use settings_loader::{LoadingOptions, SettingsError, SettingsLoader};
use std::fmt;
use std::path::PathBuf;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Settings {
    pub database: DatabaseSettings,
}

impl SettingsLoader for Settings {
    type Options = LoaderCliOptions;
}

#[derive(Clap, Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[clap(version = "0.1.1", author = "Damon Rolfs")]
#[clap(setting = AppSettings::ColoredHelp)]
pub struct LoaderCliOptions {
    /// Override environment-based configuration file to load.
    /// Default behavior is to load configuration based on `APP_ENVIRONMENT` envvar
    /// with "local" or "production" possible values.
    #[clap(short, long, parse(from_os_str), value_hint= ValueHint::AnyPath)]
    pub config: Option<PathBuf>,

    /// Specify path to optional secrets configuration file that may be introduced during deployment
    /// from a separate, secure repository.
    #[clap(short, long, parse(from_os_str), value_hint=ValueHint::AnyPath)]
    pub secrets: Option<PathBuf>,

    #[clap(subcommand)]
    pub sub_command: SubCommand,
}

impl LoadingOptions for LoaderCliOptions {
    type Error = SettingsError;

    fn config_path(&self) -> Option<PathBuf> {
        self.config.clone()
    }

    fn secrets_path(&self) -> Option<PathBuf> {
        self.secrets.clone()
    }
}

#[derive(Clap, Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[clap(version = "0.1.1", author = "Damon Rolfs")]
#[clap(setting = AppSettings::ColoredHelp)]
pub enum SubCommand {
    /// Specify path to property data file in CSV format
    #[clap(name = "property")]
    Property {
        /// Input property data file in CSV format
        #[clap(name = "FILE", parse(from_os_str), value_hint = ValueHint::AnyPath)]
        file: PathBuf,
        // Specify file to output propensity distribution visualization
        // #[clap(short, long, parse(from_os_str), value_hint = ValueHint::FilePath)]
        // distribution: Option<PathBuf>,
    },

    /// Specify path to propensity data file in CSV format
    #[clap(name = "propensity")]
    Propensity {
        /// Input propensity data file in CSV format
        #[clap(name = "FILE", parse(from_os_str), value_hint = ValueHint::AnyPath)]
        file: PathBuf,
        // /// Specify file to output propensity distribution visualization
        // #[clap(short, long, parse(from_os_str), value_hint = ValueHint::FilePath)]
        // distribution: Option<PathBuf>,
    },
}

impl fmt::Display for SubCommand {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let label = match self {
            Self::Property { file: _ } => "property",
            Self::Propensity { file: _ } => "propensity",
        };

        write!(f, "{}", label)
    }
}
