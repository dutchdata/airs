use airs::server::{Databases, SearchIndex, import_if_empty, start_server};
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing_subscriber::{EnvFilter, fmt, prelude::*};

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    tracing_subscriber::registry()
        .with(fmt::layer().with_ansi(true))
        .with(EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info")))
        .init();

    let dbs = Databases::init().expect("failed to init lmdb");

    import_if_empty(&dbs).expect("failed to import conversations.json");

    let index = Arc::new(RwLock::new(
        SearchIndex::build(&dbs).expect("failed to build search index"),
    ));

    let bind = std::env::var("AIRS_BIND").unwrap_or_else(|_| "127.0.0.1:8080".into());

    tracing::info!("starting airs on http://{}", bind);
    start_server(dbs, index, bind).await
}
