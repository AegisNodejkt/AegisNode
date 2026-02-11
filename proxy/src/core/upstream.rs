use hyper::{Client, Request, Response, Body, Uri};
use hyper_tls::HttpsConnector;
use std::str::FromStr;
use crate::core::error::AppError;
use tracing::{error, debug};

#[derive(Clone)]
pub struct UpstreamClient {
    client: Client<HttpsConnector<hyper::client::HttpConnector>>,
}

impl UpstreamClient {
    pub fn new() -> Self {
        let https = HttpsConnector::new();
        let client = Client::builder().build::<_, Body>(https);
        Self { client }
    }

    pub async fn forward(&self, mut req: Request<Body>, destination_endpoint: &str) -> Result<Response<Body>, AppError> {
        // 1. Construct new URI
        let forward_uri_str = format!("{}{}", destination_endpoint, req.uri().path_and_query().map(|x| x.as_str()).unwrap_or(""));
        debug!("Forwarding request to: {}", forward_uri_str);
        
        let forward_uri = Uri::from_str(&forward_uri_str)
            .map_err(|e| AppError::Internal(format!("Invalid URI construction: {}", e)))?;

        *req.uri_mut() = forward_uri;
        
        // 2. Clean up headers (remove host to let hyper set it, or ensure it's correct)
        req.headers_mut().remove(hyper::header::HOST);

        // 3. Send request
        match self.client.request(req).await {
            Ok(res) => Ok(res),
            Err(e) => {
                error!("Upstream request failed: {}", e);
                Err(AppError::Destination(format!("Upstream request failed: {}", e)))
            },
        }
    }
}
