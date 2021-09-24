use std::net::TcpListener;

use actix_web::dev::Server;
use actix_web::web::{self, Data};
use actix_web::{App, HttpServer};
use sqlx::PgPool;
use tracing_actix_web::TracingLogger;

use settings::Settings;

pub mod errors;
pub mod routes;
pub mod settings;

#[derive(Debug)]
pub struct ApplicationBaseUrl(pub String);

pub struct Application {
    port: u16,
    server: Server,
}

impl Application {
    #[tracing::instrument(level = "info")]
    pub async fn build(settings: Settings) -> Result<Self, std::io::Error> {
        let connection_pool = crate::core::get_connection_pool(&settings.database)
            .await
            .expect("Failed to connect to Postgres database.");

        let address = format!("{}:{}", settings.application.host, settings.application.port);
        let listener = TcpListener::bind(&address)?;
        let port = listener.local_addr().unwrap().port();
        let server = run(listener, connection_pool, /*settings.application.base_url*/)?;
        Ok(Self { port, server })
    }

    pub fn port(&self) -> u16 {
        self.port
    }

    pub async fn run_until_stopped(self) -> Result<(), std::io::Error> {
        self.server.await
    }
}

fn run(listener: TcpListener, db_pool: PgPool, /*base_url: String*/) -> Result<Server, std::io::Error> {
    let db_pool = Data::new(db_pool);
    // let base_url = Data::new(ApplicationBaseUrl(base_url));
    let server = HttpServer::new(move || {
        App::new()
            .wrap(TracingLogger::default())
            .route("/propensity", web::get().to(routes::propensity_search))
            .route("/health_check", web::get().to(routes::health_check))
            .app_data(db_pool.clone())
            // .app_data(base_url.clone())
    })
    .listen(listener)?
    .run();

    Ok(server)
}
