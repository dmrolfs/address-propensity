use address_propensity::core::settings::SettingsLoader;
use address_propensity::loader::settings::{LoaderCliOptions, Settings, SubCommand};
use address_propensity::loader::{propensity_loader, property_loader};
use address_propensity::tracing::{get_subscriber, init_subscriber};
use clap::Clap;

#[tokio::main(flavor = "multi_thread")]
async fn main() {
    let subscriber = get_subscriber("propensity-data-loader", "info", std::io::stdout);
    init_subscriber(subscriber);

    let main_span = tracing::info_span!("main");
    let _main_span_guard = main_span.enter();

    let options: LoaderCliOptions = LoaderCliOptions::parse();
    tracing::info!(?options, "Options parsed");
    let command = options.sub_command.clone();
    let command_label = format!("{}", command);
    let settings = Settings::load(options).expect("failed to load settings");
    match command {
        SubCommand::Property { file } => property_loader::load_property_data(file, settings).await,
        SubCommand::Propensity { file } => {
            propensity_loader::load_propensity_data(file, settings).await
        }
    }
    .expect(format!("failure in {} loading", command_label).as_str());
}
