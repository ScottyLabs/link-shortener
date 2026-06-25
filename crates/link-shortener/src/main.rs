use anyhow::{Context, Result};
use link_shortener_api::OidcConfig;
use link_shortener_store::Store;
use sea_orm::Database;
use sea_orm_migration::MigratorTrait;
use std::sync::Arc;
use tokio::signal;
use tracing_subscriber::{EnvFilter, layer::SubscriberExt, util::SubscriberInitExt};

#[tokio::main]
async fn main() -> Result<()> {
    dotenvy::dotenv().ok();

    tracing_subscriber::registry()
        .with(EnvFilter::new(
            std::env::var("RUST_LOG").unwrap_or_else(|_| "link_shortener=debug".into()),
        ))
        .with(tracing_subscriber::fmt::layer())
        .try_init()?;

    let database_url = std::env::var("DATABASE_URL").context("DATABASE_URL must be set")?;
    let port = std::env::var("PORT").unwrap_or_else(|_| "3000".into());

    let oidc_config = OidcConfig {
        keycloak_url: std::env::var("KEYCLOAK_URL").context("KEYCLOAK_URL must be set")?,
        keycloak_realm: std::env::var("KEYCLOAK_REALM").context("KEYCLOAK_REALM must be set")?,
        client_id: std::env::var("OIDC_CLIENT_ID").context("OIDC_CLIENT_ID must be set")?,
        client_secret: std::env::var("OIDC_CLIENT_SECRET")
            .context("OIDC_CLIENT_SECRET must be set")?,
        app_url: std::env::var("APP_URL").context("APP_URL must be set")?,
        oauth_relay_url: std::env::var("OAUTH_RELAY_URL").context("OAUTH_RELAY_URL must be set")?,
        project_group: std::env::var("PROJECT_GROUP").context("PROJECT_GROUP must be set")?,
        project_admin_group: std::env::var("PROJECT_ADMIN_GROUP")
            .context("PROJECT_ADMIN_GROUP must be set")?,
    };

    let db = Database::connect(&database_url).await?;
    tracing::info!("connected to database");

    migration::Migrator::up(&db, None).await?;
    tracing::info!("migrations applied");

    let store = Arc::new(Store::new(db));
    let app = link_shortener_api::router(store, oidc_config).await?;

    let addr = format!("0.0.0.0:{port}");
    let listener = tokio::net::TcpListener::bind(&addr).await?;
    tracing::info!("listening on {addr}");

    axum::serve(listener, app)
        .with_graceful_shutdown(shutdown_signal())
        .await?;

    Ok(())
}

async fn shutdown_signal() {
    let ctrl_c = async {
        signal::ctrl_c()
            .await
            .expect("failed to install Ctrl+C handler");
    };

    #[cfg(unix)]
    let terminate = async {
        signal::unix::signal(signal::unix::SignalKind::terminate())
            .expect("failed to install SIGTERM handler")
            .recv()
            .await;
    };

    #[cfg(not(unix))]
    let terminate = std::future::pending::<()>();

    tokio::select! {
        _ = ctrl_c => {},
        _ = terminate => {},
    }
}
