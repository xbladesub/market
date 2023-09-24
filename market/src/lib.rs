pub mod api;
pub mod app_config;
pub mod broker_client;
pub mod middleware;
pub mod strategy;
pub mod strategy_manager;

use std::{error::Error, sync::Arc, time::Duration};

use apca::{ApiInfo, Client as AlpacaClient};
use api::*;
use app_config::AppConfig;
use axum::{
    middleware::{from_fn, from_fn_with_state},
    routing::{get, post},
    Router,
};
use broker_client::Clients;
use sqlx::{postgres::PgConnectOptions, Error as SqlxError, PgPool};
use tower::ServiceBuilder;

pub struct App {
    pub db: PgPool,
    pub clients: Arc<Clients>,
    pub config: AppConfig,
}

pub async fn build_state(config: AppConfig, clients: Arc<Clients>) -> Result<App, SqlxError> {
    let opts = config.database.url.parse::<PgConnectOptions>()?;

    let pool = sqlx::pool::PoolOptions::new()
        .max_connections(10)
        .acquire_timeout(Duration::from_secs(5))
        .idle_timeout(None)
        .min_connections(1)
        .connect_with(opts)
        .await?;

    match sqlx::migrate!("./migrations").run(&pool).await {
        Ok(_) => tracing::info!("successfully run db migrations"),
        Err(err) => {
            tracing::error!("failed to run db migrations, error: {:?}", err);
            std::process::exit(1);
        }
    }

    let app = App {
        db: pool,
        clients,
        config,
    };

    Ok(app)
}

pub fn build_broker_clients(config: &AppConfig) -> Result<Arc<Clients>, Box<dyn Error>> {
    let alpaca = AlpacaClient::new(ApiInfo::from_parts(
        &config.alpaca.apca_api_base_url,
        &config.alpaca.apca_api_key_id,
        &config.alpaca.apca_api_secret_key,
    )?);

    Ok(Arc::new(Clients {
        alpaca: Arc::new(alpaca),
    }))
}

pub fn build_routes(app_state: Arc<App>) -> Router {
    Router::new()
        .route("/webhook", post(handlers::receive_webhook_alert))
        .route("/account", get(handlers::get_account))
        .route("/health", get(handlers::check_health))
        .layer(
            ServiceBuilder::new()
                .layer(from_fn_with_state(app_state.clone(), middleware::auth))
                .layer(from_fn(middleware::log_request))
                .layer(from_fn(middleware::log_response)),
        )
        .with_state(Arc::clone(&app_state))
}
