//! Rocket framework integration
//!
//! This module provides an HttpTransport implementation for testing Rocket applications.

use crate::{HttpRequest, HttpResponse, HttpTransport, Method, StatusCode};
use anyhow::Result;
use async_trait::async_trait;
use rocket::local::asynchronous::Client;
use std::sync::Arc;

/// Rocket framework transport
///
/// This transport allows you to test Rocket applications by sending requests
/// directly to the rocket instance without starting an HTTP server.
#[derive(Clone)]
pub struct RocketTransport {
    client: Arc<Client>,
}

impl std::fmt::Debug for RocketTransport {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("RocketTransport")
            .field("client", &"<rocket::Client>")
            .finish()
    }
}

impl RocketTransport {
    /// Create a new RocketTransport with the given Rocket instance
    ///
    /// # Example
    ///
    /// ```rust,ignore
    /// use rocket::{routes, get};
    /// use lightdom_test::transports::RocketTransport;
    ///
    /// #[get("/")]
    /// fn index() -> &'static str {
    ///     "Hello, World!"
    /// }
    ///
    /// #[tokio::test]
    /// async fn test() {
    ///     let rocket = rocket::build().mount("/", routes![index]);
    ///     let transport = RocketTransport::new(rocket).await.unwrap();
    /// }
    /// ```
    pub async fn new(rocket: rocket::Rocket<rocket::Build>) -> Result<Self> {
        let client = Client::tracked(rocket)
            .await
            .map_err(|e| anyhow::anyhow!("Failed to create Rocket client: {}", e))?;

        Ok(Self {
            client: Arc::new(client),
        })
    }
}

#[async_trait]
impl HttpTransport for RocketTransport {
    async fn send(&self, req: HttpRequest) -> Result<HttpResponse> {
        // Build the request
        let request = match req.method {
            Method::Get => self.client.get(&req.url),
            Method::Post => {
                let mut post_req = self.client.post(&req.url);

                // Add headers
                for (key, value) in &req.headers {
                    post_req = post_req.header(rocket::http::Header::new(
                        key.to_string(),
                        value.to_string(),
                    ));
                }

                // Add body if present
                if let Some(body) = req.body {
                    post_req = post_req.body(body);
                }

                post_req
            }
        };

        // Send the request
        let response = request.dispatch().await;

        // Extract status
        let status = StatusCode(response.status().code);

        // Extract headers
        let headers = response
            .headers()
            .iter()
            .map(|h| (h.name.to_string(), h.value.to_string()))
            .collect();

        // Extract body
        let body = response.into_string().await.unwrap_or_else(String::new);

        Ok(HttpResponse {
            status,
            headers,
            body,
        })
    }
}
