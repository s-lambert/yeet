use axum::{
    extract::State,
    http::StatusCode,
    routing::{get, post},
    Json, Router,
};
use libsql::{params, Database};
use serde::Deserialize;
use shuttle_secrets::SecretStore;
use std::time::{SystemTime, UNIX_EPOCH};

#[derive(Deserialize)]
struct CoverageReport {
    secret_phrase: String,
    statement_percent: f32,
}

#[derive(Clone)]
struct ServiceConfig {
    secret_phrase: String,
    turso_db_url: String,
    turso_auth_token: String,
}

fn get_epoch_ms() -> u128 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_millis()
}

async fn hello_world() -> &'static str {
    "Hello, world!"
}

async fn update_coverage(
    State(service_config): State<ServiceConfig>,
    Json(payload): Json<CoverageReport>,
) -> StatusCode {
    if payload.secret_phrase != service_config.secret_phrase {
        return StatusCode::BAD_REQUEST;
    }

    let db = Database::open_remote(
        service_config.turso_db_url.clone(),
        service_config.turso_auth_token.clone(),
    )
    .unwrap();

    let time = get_epoch_ms() as u64;
    let statement_percent = payload.statement_percent;
    let conn = db.connect().unwrap();
    let result = conn
        .execute(
            "INSERT INTO code_coverage VALUES (?1, ?2)",
            params![time, statement_percent],
        )
        .await;

    if result.is_err() {
        StatusCode::INTERNAL_SERVER_ERROR
    } else {
        StatusCode::ACCEPTED
    }
}

#[shuttle_runtime::main]
async fn main(#[shuttle_secrets::Secrets] secret_store: SecretStore) -> shuttle_axum::ShuttleAxum {
    let service_config = ServiceConfig {
        secret_phrase: secret_store
            .get("SECRET_PHRASE")
            .expect("Environment variables not supplied"),
        turso_db_url: secret_store
            .get("TURSO_DB_URL")
            .expect("Environment variables not supplied"),
        turso_auth_token: secret_store
            .get("TURSO_DB_AUTH_TOKEN")
            .expect("Environment variables not supplied"),
    };

    let router = Router::new()
        .route("/", get(hello_world))
        .route("/update-coverage", post(update_coverage))
        .with_state(service_config);

    Ok(router.into())
}
