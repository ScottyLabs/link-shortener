use anyhow::Context;
use fred::{
    prelude::{ClientLike, Config, Pool},
    types::config::PerformanceConfig,
};
use std::time::Duration;
use tower_sessions_redis_store::RedisStore;

pub type SessionStore = RedisStore<Pool>;

/// Connects to the Valkey database that holds login sessions
pub async fn connect(url: &str) -> anyhow::Result<SessionStore> {
    let config = Config::from_url(url).context("invalid VALKEY_URL")?;
    let performance = PerformanceConfig {
        default_command_timeout: Duration::from_secs(5),
        ..Default::default()
    };
    let pool = Pool::new(config, Some(performance), None, None, 1)
        .context("failed to configure the Valkey pool")?;

    pool.connect();
    tokio::time::timeout(Duration::from_secs(10), pool.wait_for_connect())
        .await
        .context("timed out connecting to Valkey")?
        .context("failed to connect to Valkey")?;

    Ok(RedisStore::new(pool))
}
