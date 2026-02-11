mod core;
mod modules;

use std::sync::{Arc, RwLock as StdRwLock};
use tokio::sync::RwLock as TokioRwLock;
use std::convert::Infallible;
use std::net::SocketAddr;
use tracing::{info, error, Level};
use tracing_subscriber::FmtSubscriber;

use hyper::{Server, service::{make_service_fn, service_fn}};
use crate::core::config::{load_config, watch_config};
use crate::core::engine::ProxyEngine;
use crate::modules::openai::OpenAIModule;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // 0. Initialize Logging
    let subscriber = FmtSubscriber::builder()
        .with_max_level(Level::INFO)
        .finish();
    tracing::subscriber::set_global_default(subscriber)
        .expect("setting default subscriber failed");

    // 1. Load Config
    let config_path = std::env::var("CONFIG_PATH").unwrap_or_else(|_| "config.yaml".to_string());
    info!("Loading config from: {}", config_path);
    
    let config = load_config(&config_path);
    // SharedConfig expects Arc<StdRwLock<AppConfig>>
    let shared_config = Arc::new(StdRwLock::new(config));

    // 2. Start Watcher
    watch_config(config_path.clone(), shared_config.clone()).await;

    // 3. Initialize Engine & Register Modules
    // Use Tokio RwLock for Engine to allow holding guard across await
    let shared_engine = Arc::new(TokioRwLock::new(ProxyEngine::new(shared_config.clone())));
    
    // Register OpenAI module
    {
        let mut engine = shared_engine.write().await;
        engine.register_module("openai".to_string(), Box::new(OpenAIModule));
        info!("Registered module: openai");
    }

    // 4. Setup Hyper Server
    let addr = SocketAddr::from(([0, 0, 0, 0], 8080));
    info!("AegisNode (Hyper) listening on {}", addr);

    let make_svc = make_service_fn(move |_conn| {
        let engine = shared_engine.clone();
        async move {
            Ok::<_, Infallible>(service_fn(move |req| {
                let engine = engine.clone();
                async move {
                    // Lock asynchronously, guard is Send
                    let engine_guard = engine.read().await;
                    engine_guard.handle_request(req).await
                }
            }))
        }
    });

    let server = Server::bind(&addr).serve(make_svc);

    if let Err(e) = server.await {
        error!("server error: {}", e);
    }

    Ok(())
}
