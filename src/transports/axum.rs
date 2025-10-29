//! Axum framework integration
//!
//! This module provides an HttpTransport implementation for testing axum applications.
//!
//! # Example
//!
//! ```rust,ignore
//! use axum::{Router, routing::post, Form};
//! use lightdom_test::{Dom, transports::AxumTransport};
//! use serde::Deserialize;
//!
//! #[derive(Deserialize)]
//! struct LoginForm {
//!     username: String,
//!     password: String,
//! }
//!
//! async fn login_handler(Form(form): Form<LoginForm>) -> String {
//!     if form.username == "alice" && form.password == "secret" {
//!         "Welcome, alice".to_string()
//!     } else {
//!         "Invalid credentials".to_string()
//!     }
//! }
//!
//! #[tokio::test]
//! async fn test_login() {
//!     let app = Router::new()
//!         .route("/login", post(login_handler));
//!
//!     let transport = AxumTransport::new(app);
//!     let html = r#"<form action="/login" method="post">
//!         <input name="username" type="text">
//!         <input name="password" type="password">
//!     </form>"#;
//!
//!     let mut form = Dom::new(transport)
//!         .parse(html.to_string())
//!         .unwrap()
//!         .form("/login")
//!         .unwrap();
//!
//!     form.fill("username", "alice").unwrap()
//!         .fill("password", "secret").unwrap();
//!
//!     let response = form.submit().await.unwrap();
//!     assert!(response.body.contains("Welcome, alice"));
//! }
//! ```

use crate::{HttpRequest, HttpResponse, HttpTransport, Method, StatusCode};
use anyhow::Result;
use async_trait::async_trait;
use axum::Router;
use http_body_util::BodyExt;
use hyper::body::Bytes;
use tower::ServiceExt;

/// Axum framework transport
///
/// This transport allows you to test axum applications by sending requests
/// directly to the router without starting an HTTP server.
#[derive(Clone)]
pub struct AxumTransport {
    router: Router,
}

impl std::fmt::Debug for AxumTransport {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("AxumTransport")
            .field("router", &"<axum::Router>")
            .finish()
    }
}

impl AxumTransport {
    /// Create a new AxumTransport with the given router
    ///
    /// # Example
    ///
    /// ```rust,ignore
    /// use axum::{Router, routing::get};
    /// use lightdom_test::transports::AxumTransport;
    ///
    /// let app = Router::new()
    ///     .route("/", get(|| async { "Hello, World!" }));
    ///
    /// let transport = AxumTransport::new(app);
    /// ```
    pub fn new(router: Router) -> Self {
        Self { router }
    }
}

#[async_trait]
impl HttpTransport for AxumTransport {
    async fn send(&self, req: HttpRequest) -> Result<HttpResponse> {
        // Convert Method to http::Method
        let method = match req.method {
            Method::Get => hyper::Method::GET,
            Method::Post => hyper::Method::POST,
        };

        // Build the request
        let mut request_builder = hyper::Request::builder().method(method).uri(&req.url);

        // Add headers
        for (key, value) in &req.headers {
            request_builder = request_builder.header(key, value);
        }

        // Add body if present
        let body = req.body.unwrap_or_default();
        let request = request_builder
            .body(body)
            .map_err(|e| anyhow::anyhow!("Failed to build request: {}", e))?;

        // Call the router
        let response = self
            .router
            .clone()
            .oneshot(request)
            .await
            .map_err(|e| anyhow::anyhow!("Request failed: {}", e))?;

        // Extract status
        let status = StatusCode(response.status().as_u16());

        // Extract headers
        let headers = response
            .headers()
            .iter()
            .map(|(k, v)| (k.to_string(), v.to_str().unwrap_or_default().to_string()))
            .collect();

        // Extract body
        let body_bytes: Bytes = response
            .into_body()
            .collect()
            .await
            .map_err(|e| anyhow::anyhow!("Failed to read response body: {}", e))?
            .to_bytes();

        let body = String::from_utf8_lossy(&body_bytes).to_string();

        Ok(HttpResponse {
            status,
            headers,
            body,
        })
    }
}
