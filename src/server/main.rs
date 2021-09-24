use address_propensity::core::settings::SettingsLoader;
use address_propensity::server::errors::PropensityError;
use address_propensity::server::settings::{HttpServerCliOptions, Settings};
use address_propensity::server::Application;
use address_propensity::tracing::{get_subscriber, init_subscriber};
use clap::Clap;

#[actix_web::main]
async fn main() -> Result<(), PropensityError> {
    let subscriber = get_subscriber("address-propensity", "info", std::io::stdout);
    init_subscriber(subscriber);

    let main_span = tracing::info_span!("main");
    let _main_span_guard = main_span.enter();

    let options: HttpServerCliOptions = HttpServerCliOptions::parse();
    let settings = Settings::load(options)?;
    let application = Application::build(settings).await?;
    application.run_until_stopped().await?;
    Ok(())
}
