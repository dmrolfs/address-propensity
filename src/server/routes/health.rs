use actix_web::HttpResponse;

#[tracing::instrument(level = "info")]
pub async fn health_check() -> HttpResponse {
    //todo!("check database");
    HttpResponse::Ok().finish()
}
