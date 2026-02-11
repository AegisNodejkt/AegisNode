use async_trait::async_trait;
use hyper::{Request, Response, Body};
use tracing::{error, instrument};
use crate::core::engine::ProxyModule;
use crate::core::upstream::UpstreamClient;
use crate::core::config::SharedConfig;
use crate::core::scanner::PIIScanner;
use crate::core::error::AppError;

pub struct OpenAIModule;

#[async_trait]
impl ProxyModule for OpenAIModule {
    #[instrument(skip(self, req, config), fields(module = "openai"))]
    async fn on_request(&self, req: &mut Request<Body>, config: &SharedConfig) -> Result<Option<Response<Body>>, AppError> {
        
        let client = UpstreamClient::new();
        
        let (destination_endpoint, rules, api_key) = {
            let cfg = config.read().map_err(|e| AppError::Internal(format!("Config lock error: {}", e)))?;
            (cfg.destination.endpoint.clone(), cfg.rules.clone(), cfg.destination.api_key.clone())
        };

        // 1. Initialize Scanner
        let scanner = PIIScanner::new(rules);

        // 2. Scan and Redact Request Body
        // This modifies the request in-place if sensitive data is found
        if let Err(e) = scanner.scan_and_redact_request(req).await {
            error!("OpenAI Module: Error scanning request: {}", e);
            // In strict mode, we might want to return AppError::Parsing(e) here.
            // For now, we log and proceed, or maybe return error if critical.
            // Let's assume scanning failure is critical for privacy.
            return Err(AppError::Parsing(format!("PII Scanning failed: {}", e)));
        }

        req.headers_mut().insert(
            "Authorization",
            format!("Bearer {}", api_key)
                .parse()
                .map_err(|e| AppError::Config(format!("Invalid API key format: {}", e)))?,
        );

        // Take ownership of the request to forward it
        let req_owned = std::mem::take(req);
        
        let response = client.forward(req_owned, &destination_endpoint).await?;
        
        Ok(Some(response))
    }
    
    async fn on_response(&self, _res: &mut Response<Body>, _config: &SharedConfig) -> Result<(), AppError> {
        // Logic to inspect/redact response body would go here
        Ok(())
    }
}
