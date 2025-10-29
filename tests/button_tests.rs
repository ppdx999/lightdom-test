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
async fn test_button_locator_by_id() -> Result<()> {
    let html = r#"
        <form action="/submit" method="post">
            <button id="submit-btn" type="submit">Submit</button>
        </form>
    "#;

    let transport = MockTransport::new(default_response());
    let dom = Dom::new(transport.clone()).parse(html.to_string())?;
    let button = dom.button("#submit-btn")?;

    button.click().await?;

    let requests = transport.get_captured_requests();
    assert_eq!(requests.len(), 1);
    assert_eq!(requests[0].url, "/submit");
    assert_eq!(requests[0].method, Method::Post);
    Ok(())
}

#[tokio::test]
async fn test_button_locator_by_test_id() -> Result<()> {
    let html = r#"
        <form action="/delete" method="post">
            <button test-id="delete-btn" type="submit">Delete</button>
        </form>
    "#;

    let transport = MockTransport::new(default_response());
    let dom = Dom::new(transport.clone()).parse(html.to_string())?;
    let button = dom.button("@delete-btn")?;

    button.click().await?;

    let requests = transport.get_captured_requests();
    assert_eq!(requests[0].url, "/delete");
    Ok(())
}

#[tokio::test]
async fn test_button_locator_by_text() -> Result<()> {
    let html = r#"
        <form action="/login" method="post">
            <button type="submit">Login</button>
        </form>
    "#;

    let transport = MockTransport::new(default_response());
    let dom = Dom::new(transport.clone()).parse(html.to_string())?;
    let button = dom.button("Login")?;

    button.click().await?;

    let requests = transport.get_captured_requests();
    assert_eq!(requests[0].url, "/login");
    Ok(())
}

#[tokio::test]
async fn test_button_not_found() {
    let html = r#"
        <form action="/submit" method="post">
            <button id="submit-btn">Submit</button>
        </form>
    "#;

    let transport = MockTransport::new(default_response());
    let dom = Dom::new(transport).parse(html.to_string()).unwrap();
    let result = dom.button("#nonexistent");

    assert!(result.is_err());
    assert!(result.unwrap_err().to_string().contains("Button not found"));
}

#[tokio::test]
async fn test_button_without_form() {
    let html = r#"
        <button id="standalone-btn">Click Me</button>
    "#;

    let transport = MockTransport::new(default_response());
    let dom = Dom::new(transport).parse(html.to_string()).unwrap();
    let button = dom.button("#standalone-btn").unwrap();

    let result = button.click().await;
    assert!(result.is_err());
    assert!(result
        .unwrap_err()
        .to_string()
        .contains("not associated with a form"));
}

#[tokio::test]
async fn test_button_in_form_with_get_method() -> Result<()> {
    let html = r#"
        <form action="/search" method="get">
            <button id="search-btn">Search</button>
        </form>
    "#;

    let transport = MockTransport::new(default_response());
    let dom = Dom::new(transport.clone()).parse(html.to_string())?;
    let button = dom.button("#search-btn")?;

    button.click().await?;

    let requests = transport.get_captured_requests();
    assert_eq!(requests[0].method, Method::Get);
    assert_eq!(requests[0].url, "/search");
    Ok(())
}

#[tokio::test]
async fn test_multiple_buttons_in_form() -> Result<()> {
    let html = r#"
        <form action="/action" method="post">
            <button id="btn1">Button 1</button>
            <button id="btn2">Button 2</button>
        </form>
    "#;

    let transport1 = MockTransport::new(default_response());
    let transport2 = MockTransport::new(default_response());

    let dom1 = Dom::new(transport1.clone()).parse(html.to_string())?;
    let button1 = dom1.button("#btn1")?;
    button1.click().await?;

    let dom2 = Dom::new(transport2.clone()).parse(html.to_string())?;
    let button2 = dom2.button("#btn2")?;
    button2.click().await?;

    let req1 = &transport1.get_captured_requests()[0];
    let req2 = &transport2.get_captured_requests()[0];

    // 両方とも同じフォームをトリガーする
    assert_eq!(req1.url, "/action");
    assert_eq!(req2.url, "/action");
    Ok(())
}

#[tokio::test]
async fn test_button_text_with_whitespace() -> Result<()> {
    let html = r#"
        <form action="/submit" method="post">
            <button>
                Submit Form
            </button>
        </form>
    "#;

    let transport = MockTransport::new(default_response());
    let dom = Dom::new(transport.clone()).parse(html.to_string())?;
    let button = dom.button("Submit Form")?;

    button.click().await?;

    let requests = transport.get_captured_requests();
    assert_eq!(requests[0].url, "/submit");
    Ok(())
}
