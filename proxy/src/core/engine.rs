use async_trait::async_trait;
use hyper::{Request, Response, StatusCode};
use std::collections::HashMap;
use tracing::{error, info, instrument};
use crate::core::config::SharedConfig;
use crate::core::error::AppError;

#[async_trait]
pub trait ProxyModule: Send + Sync {
    /// Process request before sending to backend.
    /// Returns Ok(Some(Response)) to short-circuit (e.g. block/cache),
    /// Ok(None) to continue, or Err to fail.
    async fn on_request(&self, req: &mut Request<hyper::Body>, config: &SharedConfig) -> Result<Option<Response<hyper::Body>>, AppError>;
    
    /// Process response from backend before returning to client.
    async fn on_response(&self, res: &mut Response<hyper::Body>, config: &SharedConfig) -> Result<(), AppError>;
}

pub struct ProxyEngine {
    modules: HashMap<String, Box<dyn ProxyModule>>,
    config: SharedConfig,
}

impl ProxyEngine {
    pub fn new(config: SharedConfig) -> Self {
        Self {
            modules: HashMap::new(),
            config,
        }
    }

    pub fn register_module(&mut self, name: String, module: Box<dyn ProxyModule>) {
        self.modules.insert(name, module);
    }

    #[instrument(skip(self, req), fields(method = %req.method(), uri = %req.uri()))]
    pub async fn handle_request(&self, mut req: Request<hyper::Body>) -> Result<Response<hyper::Body>, hyper::Error> {
        // 1. Determine Destination from Config
        let destination = {
            match self.config.read() {
                Ok(cfg) => cfg.destination.clone(),
                Err(e) => {
                    error!("Failed to acquire config lock: {}", e);
                    return Ok(AppError::Internal("Config lock failed".to_string()).to_response());
                }
            }
        };

        // 2. Find matching module
        if let Some(module) = self.modules.get(&destination.name) {
             // 3. Run on_request pipeline
             match module.on_request(&mut req, &self.config).await {
                Ok(Some(short_circuit_resp)) => {
                    info!("Request short-circuited by module: {}", destination.name);
                    return Ok(short_circuit_resp)
                },
                Ok(None) => { /* Continue */ },
                Err(e) => {
                    error!("Module {} on_request failed: {:?}", destination.name, e);
                    return Ok(e.to_response());
                }
             }
             
             // In a full proxy, we would forward here if not short-circuited.
             // But current architecture seems to rely on module to forward (OpenAIModule returns Some).
             // If we reach here, it means module returned None, so we need a default behavior or error.
             
             let mut res = Response::new(hyper::Body::from("Upstream Response Placeholder"));

             // 4. Run on_response pipeline
             if let Err(e) = module.on_response(&mut res, &self.config).await {
                 error!("Module {} on_response failed: {:?}", destination.name, e);
                 return Ok(e.to_response());
             }

             Ok(res)

        } else {
            error!("No module found for destination: {}", destination.name);
            Ok(AppError::Config(format!("No module found for destination: {}", destination.name)).to_response())
        }
    }
}
