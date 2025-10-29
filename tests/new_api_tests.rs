use anyhow::Result;
use lightdom_test::{Dom, HttpRequest, HttpResponse, HttpTransport, StatusCode};
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

// ============================================
// Exists Check API Tests
// ============================================

#[tokio::test]
async fn test_exists_element_found() -> Result<()> {
    let html = r#"
        <div id="content">Hello</div>
    "#;

    let transport = MockTransport::new(default_response());
    let dom = Dom::new(transport).parse(html.to_string())?;

    assert!(dom.exists("#content"));
    assert!(!dom.exists("#nonexistent"));
    Ok(())
}

#[tokio::test]
async fn test_contains_text() -> Result<()> {
    let html = r#"
        <div>
            <h1>Welcome to my site</h1>
            <p>This is a test page</p>
        </div>
    "#;

    let transport = MockTransport::new(default_response());
    let dom = Dom::new(transport).parse(html.to_string())?;

    assert!(dom.contains_text("Welcome"));
    assert!(dom.contains_text("test page"));
    assert!(!dom.contains_text("Goodbye"));
    Ok(())
}

// ============================================
// Meta Tags API Tests
// ============================================

#[tokio::test]
async fn test_title() -> Result<()> {
    let html = r#"
        <!DOCTYPE html>
        <html>
        <head>
            <title>My Website - Home</title>
        </head>
        <body></body>
        </html>
    "#;

    let transport = MockTransport::new(default_response());
    let dom = Dom::new(transport).parse(html.to_string())?;

    let title = dom.title()?;
    assert_eq!(title, "My Website - Home");
    Ok(())
}

#[tokio::test]
async fn test_meta_name() -> Result<()> {
    let html = r#"
        <!DOCTYPE html>
        <html>
        <head>
            <meta name="description" content="This is my website">
            <meta name="keywords" content="rust, web, testing">
        </head>
        <body></body>
        </html>
    "#;

    let transport = MockTransport::new(default_response());
    let dom = Dom::new(transport).parse(html.to_string())?;

    let description = dom.meta("description")?;
    assert_eq!(description, "This is my website");

    let keywords = dom.meta("keywords")?;
    assert_eq!(keywords, "rust, web, testing");
    Ok(())
}

#[tokio::test]
async fn test_meta_property_ogp() -> Result<()> {
    let html = r#"
        <!DOCTYPE html>
        <html>
        <head>
            <meta property="og:title" content="My Page Title">
            <meta property="og:description" content="Page description">
        </head>
        <body></body>
        </html>
    "#;

    let transport = MockTransport::new(default_response());
    let dom = Dom::new(transport).parse(html.to_string())?;

    let og_title = dom.meta("og:title")?;
    assert_eq!(og_title, "My Page Title");

    let og_description = dom.meta("og:description")?;
    assert_eq!(og_description, "Page description");
    Ok(())
}

#[tokio::test]
async fn test_meta_not_found() {
    let html = r#"
        <!DOCTYPE html>
        <html>
        <head></head>
        <body></body>
        </html>
    "#;

    let transport = MockTransport::new(default_response());
    let dom = Dom::new(transport).parse(html.to_string()).unwrap();
    let result = dom.meta("description");

    assert!(result.is_err());
    assert!(result.unwrap_err().to_string().contains("not found"));
}

// ============================================
// Element State API Tests
// ============================================

#[tokio::test]
async fn test_element_text_contains() -> Result<()> {
    let html = r#"
        <p id="message">Success: Operation completed successfully</p>
    "#;

    let transport = MockTransport::new(default_response());
    let dom = Dom::new(transport).parse(html.to_string())?;

    let element = dom.element("#message")?;
    assert!(element.text_contains("Success"));
    assert!(element.text_contains("Operation completed"));
    assert!(!element.text_contains("Error"));
    Ok(())
}

#[tokio::test]
async fn test_element_is_disabled() -> Result<()> {
    let html = r#"
        <button id="btn1" disabled>Button 1</button>
        <button id="btn2">Button 2</button>
    "#;

    let transport = MockTransport::new(default_response());
    let dom = Dom::new(transport).parse(html.to_string())?;

    let btn1 = dom.element("#btn1")?;
    assert!(btn1.is_disabled());

    let btn2 = dom.element("#btn2")?;
    assert!(!btn2.is_disabled());
    Ok(())
}

#[tokio::test]
async fn test_element_is_required() -> Result<()> {
    let html = r#"
        <input id="email" type="email" required>
        <input id="name" type="text">
    "#;

    let transport = MockTransport::new(default_response());
    let dom = Dom::new(transport).parse(html.to_string())?;

    let email = dom.element("#email")?;
    assert!(email.is_required());

    let name = dom.element("#name")?;
    assert!(!name.is_required());
    Ok(())
}

#[tokio::test]
async fn test_element_is_readonly() -> Result<()> {
    let html = r#"
        <input id="code" type="text" readonly value="ABC123">
        <input id="text" type="text">
    "#;

    let transport = MockTransport::new(default_response());
    let dom = Dom::new(transport).parse(html.to_string())?;

    let code = dom.element("#code")?;
    assert!(code.is_readonly());

    let text = dom.element("#text")?;
    assert!(!text.is_readonly());
    Ok(())
}

#[tokio::test]
async fn test_element_is_checked() -> Result<()> {
    let html = r#"
        <input id="agree" type="checkbox" checked>
        <input id="disagree" type="checkbox">
    "#;

    let transport = MockTransport::new(default_response());
    let dom = Dom::new(transport).parse(html.to_string())?;

    let agree = dom.element("#agree")?;
    assert!(agree.is_checked());

    let disagree = dom.element("#disagree")?;
    assert!(!disagree.is_checked());
    Ok(())
}

// ============================================
// Form Value API Tests
// ============================================

#[tokio::test]
async fn test_form_get_value() -> Result<()> {
    let html = r#"
        <form id="edit-form">
            <input name="username" type="text" value="alice">
            <input name="email" type="email" value="alice@example.com">
            <input name="token" type="hidden" value="secret123">
        </form>
    "#;

    let transport = MockTransport::new(default_response());
    let dom = Dom::new(transport).parse(html.to_string())?;

    let form = dom.form("#edit-form")?;

    // hidden フィールドの値は自動的に収集される
    let token = form.get_value("token")?;
    assert_eq!(token, "secret123");
    Ok(())
}

#[tokio::test]
async fn test_form_get_value_after_fill() -> Result<()> {
    let html = r#"
        <form id="form">
            <input name="field1" type="text">
        </form>
    "#;

    let transport = MockTransport::new(default_response());
    let dom = Dom::new(transport).parse(html.to_string())?;

    let mut form = dom.form("#form")?;

    // fill で値を設定
    form.fill("field1", "test_value")?;

    // 設定した値を取得
    let value = form.get_value("field1")?;
    assert_eq!(value, "test_value");
    Ok(())
}

#[tokio::test]
async fn test_form_get_value_not_found() {
    let html = r#"
        <form id="form">
            <input name="field1" type="text">
        </form>
    "#;

    let transport = MockTransport::new(default_response());
    let dom = Dom::new(transport).parse(html.to_string()).unwrap();

    let form = dom.form("#form").unwrap();
    let result = form.get_value("nonexistent");

    assert!(result.is_err());
    assert!(result.unwrap_err().to_string().contains("not found"));
}

// ============================================
// Image API Tests
// ============================================

#[tokio::test]
async fn test_image_by_id() -> Result<()> {
    let html = r#"
        <img id="logo" src="/logo.png" alt="Company Logo" width="100" height="50">
    "#;

    let transport = MockTransport::new(default_response());
    let dom = Dom::new(transport).parse(html.to_string())?;

    let img = dom.image("#logo")?;
    assert_eq!(img.src(), "/logo.png");
    assert_eq!(img.alt(), Some("Company Logo".to_string()));
    assert_eq!(img.width(), Some("100".to_string()));
    assert_eq!(img.height(), Some("50".to_string()));
    Ok(())
}

#[tokio::test]
async fn test_image_without_alt() -> Result<()> {
    let html = r#"
        <img id="banner" src="/banner.jpg">
    "#;

    let transport = MockTransport::new(default_response());
    let dom = Dom::new(transport).parse(html.to_string())?;

    let img = dom.image("#banner")?;
    assert_eq!(img.src(), "/banner.jpg");
    assert_eq!(img.alt(), None);
    assert_eq!(img.width(), None);
    assert_eq!(img.height(), None);
    Ok(())
}

#[tokio::test]
async fn test_images_multiple() -> Result<()> {
    let html = r#"
        <div>
            <img class="thumbnail" src="/img1.png" alt="Image 1">
            <img class="thumbnail" src="/img2.png" alt="Image 2">
            <img class="thumbnail" src="/img3.png" alt="Image 3">
        </div>
    "#;

    let transport = MockTransport::new(default_response());
    let dom = Dom::new(transport).parse(html.to_string())?;

    let images = dom.images(".thumbnail");
    assert_eq!(images.len(), 3);
    assert_eq!(images[0].src(), "/img1.png");
    assert_eq!(images[1].src(), "/img2.png");
    assert_eq!(images[2].src(), "/img3.png");
    Ok(())
}

#[tokio::test]
async fn test_images_all() -> Result<()> {
    let html = r#"
        <img src="/img1.png">
        <img src="/img2.png">
    "#;

    let transport = MockTransport::new(default_response());
    let dom = Dom::new(transport).parse(html.to_string())?;

    let images = dom.images("img");
    assert_eq!(images.len(), 2);
    Ok(())
}

// ============================================
// Select Element API Tests
// ============================================

#[tokio::test]
async fn test_select_element_options() -> Result<()> {
    let html = r#"
        <select id="country">
            <option value="us">United States</option>
            <option value="jp">Japan</option>
            <option value="uk">United Kingdom</option>
        </select>
    "#;

    let transport = MockTransport::new(default_response());
    let dom = Dom::new(transport).parse(html.to_string())?;

    let select = dom.select_element("#country")?;
    let options = select.options();

    assert_eq!(options.len(), 3);
    assert_eq!(options[0].value(), "us");
    assert_eq!(options[0].text(), "United States");
    assert_eq!(options[1].value(), "jp");
    assert_eq!(options[1].text(), "Japan");
    assert_eq!(options[2].value(), "uk");
    assert_eq!(options[2].text(), "United Kingdom");
    Ok(())
}

#[tokio::test]
async fn test_select_element_selected_option() -> Result<()> {
    let html = r#"
        <select id="lang">
            <option value="en">English</option>
            <option value="ja" selected>Japanese</option>
            <option value="fr">French</option>
        </select>
    "#;

    let transport = MockTransport::new(default_response());
    let dom = Dom::new(transport).parse(html.to_string())?;

    let select = dom.select_element("#lang")?;
    let selected = select.selected_option()?;

    assert_eq!(selected.value(), "ja");
    assert_eq!(selected.text(), "Japanese");
    assert!(selected.is_selected());
    Ok(())
}

#[tokio::test]
async fn test_select_element_option_without_value() -> Result<()> {
    let html = r#"
        <select id="color">
            <option>Red</option>
            <option>Green</option>
            <option>Blue</option>
        </select>
    "#;

    let transport = MockTransport::new(default_response());
    let dom = Dom::new(transport).parse(html.to_string())?;

    let select = dom.select_element("#color")?;
    let options = select.options();

    // value 属性がない場合はテキストが value になる
    assert_eq!(options[0].value(), "Red");
    assert_eq!(options[0].text(), "Red");
    Ok(())
}

#[tokio::test]
async fn test_select_element_no_selection() {
    let html = r#"
        <select id="choice">
            <option value="a">Option A</option>
            <option value="b">Option B</option>
        </select>
    "#;

    let transport = MockTransport::new(default_response());
    let dom = Dom::new(transport).parse(html.to_string()).unwrap();

    let select = dom.select_element("#choice").unwrap();
    let result = select.selected_option();

    assert!(result.is_err());
    assert!(result.unwrap_err().to_string().contains("No option is selected"));
}
