use anyhow::Result;
use lightdom_test::{Dom, HttpRequest, HttpResponse, HttpTransport, Method, StatusCode};
use std::sync::{Arc, Mutex};

/// モック Transport 実装
#[derive(Clone, Debug)]
struct MockTransport {
    captured_requests: Arc<Mutex<Vec<HttpRequest>>>,
    response: HttpResponse,
}

impl MockTransport {
    fn new(response: HttpResponse) -> Self {
        Self {
            captured_requests: Arc::new(Mutex::new(Vec::new())),
            response,
        }
    }

    fn get_captured_requests(&self) -> Vec<HttpRequest> {
        self.captured_requests.lock().unwrap().clone()
    }
}

#[async_trait::async_trait]
impl HttpTransport for MockTransport {
    async fn send(&self, req: HttpRequest) -> Result<HttpResponse> {
        self.captured_requests.lock().unwrap().push(req.clone());
        Ok(self.response.clone())
    }
}

fn default_response() -> HttpResponse {
    HttpResponse {
        status: StatusCode(200),
        headers: Default::default(),
        body: "OK".to_string(),
    }
}

#[tokio::test]
async fn test_link_locator_by_id() -> Result<()> {
    let html = r#"
        <a id="home-link" href="/home">Home</a>
    "#;

    let transport = MockTransport::new(default_response());
    let dom = Dom::new(transport.clone()).parse(html.to_string())?;
    let link = dom.link("#home-link")?;

    link.click().await?;

    let requests = transport.get_captured_requests();
    assert_eq!(requests.len(), 1);
    assert_eq!(requests[0].url, "/home");
    assert_eq!(requests[0].method, Method::Get);
    Ok(())
}

#[tokio::test]
async fn test_link_locator_by_test_id() -> Result<()> {
    let html = r#"
        <a test-id="about-link" href="/about">About</a>
    "#;

    let transport = MockTransport::new(default_response());
    let dom = Dom::new(transport.clone()).parse(html.to_string())?;
    let link = dom.link("@about-link")?;

    link.click().await?;

    let requests = transport.get_captured_requests();
    assert_eq!(requests[0].url, "/about");
    Ok(())
}

#[tokio::test]
async fn test_link_locator_by_text() -> Result<()> {
    let html = r#"
        <nav>
            <a href="/contact">Contact Us</a>
        </nav>
    "#;

    let transport = MockTransport::new(default_response());
    let dom = Dom::new(transport.clone()).parse(html.to_string())?;
    let link = dom.link("Contact Us")?;

    link.click().await?;

    let requests = transport.get_captured_requests();
    assert_eq!(requests[0].url, "/contact");
    Ok(())
}

#[tokio::test]
async fn test_link_not_found() {
    let html = r#"
        <a id="home-link" href="/home">Home</a>
    "#;

    let transport = MockTransport::new(default_response());
    let dom = Dom::new(transport).parse(html.to_string()).unwrap();
    let result = dom.link("#nonexistent");

    assert!(result.is_err());
    assert!(result.unwrap_err().to_string().contains("Link not found"));
}

#[tokio::test]
async fn test_link_without_href() {
    let html = r#"
        <a id="invalid-link">No Href</a>
    "#;

    let transport = MockTransport::new(default_response());
    let dom = Dom::new(transport).parse(html.to_string()).unwrap();
    let result = dom.link("#invalid-link");

    assert!(result.is_err());
    assert!(
        result
            .unwrap_err()
            .to_string()
            .contains("no href attribute")
    );
}

#[tokio::test]
async fn test_link_with_absolute_url() -> Result<()> {
    let html = r#"
        <a id="external" href="https://example.com">Example</a>
    "#;

    let transport = MockTransport::new(default_response());
    let dom = Dom::new(transport.clone()).parse(html.to_string())?;
    let link = dom.link("#external")?;

    link.click().await?;

    let requests = transport.get_captured_requests();
    assert_eq!(requests[0].url, "https://example.com");
    Ok(())
}

#[tokio::test]
async fn test_multiple_links() -> Result<()> {
    let html = r#"
        <nav>
            <a id="link1" href="/page1">Page 1</a>
            <a id="link2" href="/page2">Page 2</a>
            <a id="link3" href="/page3">Page 3</a>
        </nav>
    "#;

    let transport1 = MockTransport::new(default_response());
    let transport2 = MockTransport::new(default_response());
    let transport3 = MockTransport::new(default_response());

    let dom1 = Dom::new(transport1.clone()).parse(html.to_string())?;
    let link1 = dom1.link("#link1")?;
    link1.click().await?;

    let dom2 = Dom::new(transport2.clone()).parse(html.to_string())?;
    let link2 = dom2.link("#link2")?;
    link2.click().await?;

    let dom3 = Dom::new(transport3.clone()).parse(html.to_string())?;
    let link3 = dom3.link("#link3")?;
    link3.click().await?;

    assert_eq!(transport1.get_captured_requests()[0].url, "/page1");
    assert_eq!(transport2.get_captured_requests()[0].url, "/page2");
    assert_eq!(transport3.get_captured_requests()[0].url, "/page3");
    Ok(())
}

#[tokio::test]
async fn test_link_text_with_whitespace() -> Result<()> {
    let html = r#"
        <a href="/dashboard">
            Dashboard
        </a>
    "#;

    let transport = MockTransport::new(default_response());
    let dom = Dom::new(transport.clone()).parse(html.to_string())?;
    let link = dom.link("Dashboard")?;

    link.click().await?;

    let requests = transport.get_captured_requests();
    assert_eq!(requests[0].url, "/dashboard");
    Ok(())
}

#[tokio::test]
async fn test_link_always_get_method() -> Result<()> {
    let html = r#"
        <a href="/resource">Resource</a>
    "#;

    let transport = MockTransport::new(default_response());
    let dom = Dom::new(transport.clone()).parse(html.to_string())?;
    let link = dom.link("Resource")?;

    link.click().await?;

    let requests = transport.get_captured_requests();
    // リンクは常にGETメソッド
    assert_eq!(requests[0].method, Method::Get);
    Ok(())
}

#[tokio::test]
async fn test_link_with_query_params() -> Result<()> {
    let html = r#"
        <a href="/search?q=rust&lang=en">Search</a>
    "#;

    let transport = MockTransport::new(default_response());
    let dom = Dom::new(transport.clone()).parse(html.to_string())?;
    let link = dom.link("Search")?;

    link.click().await?;

    let requests = transport.get_captured_requests();
    assert_eq!(requests[0].url, "/search?q=rust&lang=en");
    Ok(())
}

#[tokio::test]
async fn test_link_with_hash() -> Result<()> {
    let html = r#"
        <a href="/docs#introduction">Documentation</a>
    "#;

    let transport = MockTransport::new(default_response());
    let dom = Dom::new(transport.clone()).parse(html.to_string())?;
    let link = dom.link("Documentation")?;

    link.click().await?;

    let requests = transport.get_captured_requests();
    assert_eq!(requests[0].url, "/docs#introduction");
    Ok(())
}
