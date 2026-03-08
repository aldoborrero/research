use std::sync::Arc;

use axum::Router;
use clave_core::auth::AuthEvent;
use clave_core::{ClaveClient, Config, Session};
use tokio::sync::broadcast;
use tokio_util::sync::CancellationToken;
use tower_http::cors::CorsLayer;

mod routes;
mod sse;

#[derive(Clone)]
pub struct AppState {
    pub client: Arc<ClaveClient>,
    pub session: Arc<Session>,
    pub event_tx: broadcast::Sender<AuthEvent>,
    pub shutdown: CancellationToken,
}

#[tokio::main]
async fn main() -> std::result::Result<(), Box<dyn std::error::Error>> {
    tracing_subscriber::fmt()
        .with_env_filter("clave_web=info,clave_core=info")
        .init();

    let _config = Config::load()?;
    let session = Session::load()?;
    let client = Arc::new(ClaveClient::new()?);
    let session = Arc::new(session);

    let shutdown = CancellationToken::new();
    let (event_tx, _) = broadcast::channel::<AuthEvent>(64);

    // Start the background listener
    let listener_shutdown = shutdown.clone();
    let listener_client = client.clone();
    let listener_session = session.clone();
    let listener_tx = event_tx.clone();
    tokio::spawn(async move {
        clave_core::auth::run_listener(
            &listener_client,
            &listener_session,
            std::time::Duration::from_secs(5),
            listener_shutdown,
            listener_tx,
        )
        .await;
    });

    let state = AppState {
        client,
        session,
        event_tx,
        shutdown: shutdown.clone(),
    };

    let app = Router::new()
        .nest("/api", routes::api_router())
        .route("/api/events", axum::routing::get(sse::sse_handler))
        .fallback(routes::static_handler)
        .layer(CorsLayer::permissive())
        .with_state(state);

    let addr = std::net::SocketAddr::from(([127, 0, 0, 1], 3000));
    tracing::info!("clave-web listening on http://{}", addr);

    let listener = tokio::net::TcpListener::bind(addr).await?;
    axum::serve(listener, app)
        .with_graceful_shutdown(async move {
            tokio::signal::ctrl_c().await.ok();
            shutdown.cancel();
        })
        .await?;

    Ok(())
}
