use anyhow::Result;
use lightdom_test::{Dom, HttpRequest, HttpResponse, HttpTransport, Method, StatusCode};
use std::sync::{Arc, Mutex};

/// モック Transport 実装
/// 送信されたリクエストを記録し、検証できるようにする
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

/// デフォルトのモックレスポンスを作成
fn default_response() -> HttpResponse {
    HttpResponse {
        status: StatusCode(200),
        headers: Default::default(),
        body: "OK".to_string(),
    }
}

#[tokio::test]
async fn test_form_locator_by_id() -> Result<()> {
    let html = r#"
        <form id="login-form" action="/login" method="post">
            <input name="username" type="text">
        </form>
    "#;

    let transport = MockTransport::new(default_response());
    let dom = Dom::new(transport.clone()).parse(html.to_string())?;
    let mut form = dom.form("#login-form")?;

    // フォームが正しく取得できることを確認（送信してリクエストを検証）
    form.fill("username", "value")?;
    form.submit().await?;

    let requests = transport.get_captured_requests();
    assert_eq!(requests.len(), 1);
    assert_eq!(requests[0].url, "/login");
    assert_eq!(requests[0].method, Method::Post);
    Ok(())
}

#[tokio::test]
async fn test_form_locator_by_action() -> Result<()> {
    let html = r#"
        <form action="/submit" method="post">
            <input name="data" type="text">
        </form>
    "#;

    let transport = MockTransport::new(default_response());
    let dom = Dom::new(transport.clone()).parse(html.to_string())?;
    let mut form = dom.form("/submit")?;

    form.fill("data", "value")?;
    form.submit().await?;

    let requests = transport.get_captured_requests();
    assert_eq!(requests[0].url, "/submit");
    Ok(())
}

#[tokio::test]
async fn test_form_locator_by_test_id() -> Result<()> {
    let html = r#"
        <form test-id="signup-form" action="/signup" method="post">
            <input name="email" type="email">
        </form>
    "#;

    let transport = MockTransport::new(default_response());
    let dom = Dom::new(transport.clone()).parse(html.to_string())?;
    let mut form = dom.form("@signup-form")?;

    form.fill("email", "test@example.com")?;
    form.submit().await?;

    let requests = transport.get_captured_requests();
    assert_eq!(requests[0].url, "/signup");
    Ok(())
}

#[tokio::test]
async fn test_form_not_found() {
    let html = r#"
        <form id="login-form" action="/login" method="post">
            <input name="username" type="text">
        </form>
    "#;

    let transport = MockTransport::new(default_response());
    let dom = Dom::new(transport).parse(html.to_string()).unwrap();
    let result = dom.form("#nonexistent");

    assert!(result.is_err());
    assert!(result.unwrap_err().to_string().contains("Form not found"));
}

#[tokio::test]
async fn test_form_hidden_fields_auto_collected() -> Result<()> {
    let html = r#"
        <form id="form" action="/submit" method="post">
            <input type="hidden" name="_csrf" value="token123">
            <input type="hidden" name="session_id" value="abc456">
            <input name="username" type="text">
        </form>
    "#;

    let transport = MockTransport::new(default_response());
    let dom = Dom::new(transport.clone()).parse(html.to_string())?;
    let mut form = dom.form("#form")?;

    form.fill("username", "alice")?;

    form.submit().await?;

    let requests = transport.get_captured_requests();
    assert_eq!(requests.len(), 1);

    let body = requests[0].body.as_ref().unwrap();
    // hidden フィールドが自動的に含まれていることを確認
    assert!(body.contains("_csrf=token123"));
    assert!(body.contains("session_id=abc456"));
    assert!(body.contains("username=alice"));
    Ok(())
}

#[tokio::test]
async fn test_form_fill_and_submit() -> Result<()> {
    let html = r#"
        <form id="form" action="/login" method="post">
            <input name="username" type="text">
            <input name="password" type="password">
        </form>
    "#;

    let transport = MockTransport::new(default_response());
    let dom = Dom::new(transport.clone()).parse(html.to_string())?;
    let mut form = dom.form("#form")?;

    form.fill("username", "alice")?
        .fill("password", "secret123")?;

    form.submit().await?;

    let requests = transport.get_captured_requests();
    assert_eq!(requests.len(), 1);

    let req = &requests[0];
    assert_eq!(req.method, Method::Post);
    assert_eq!(req.url, "/login");

    let body = req.body.as_ref().unwrap();
    assert!(body.contains("username=alice"));
    assert!(body.contains("password=secret123"));
    Ok(())
}

#[tokio::test]
async fn test_form_method_get() -> Result<()> {
    let html = r#"
        <form id="search" action="/search" method="get">
            <input name="q" type="text">
        </form>
    "#;

    let transport = MockTransport::new(default_response());
    let dom = Dom::new(transport.clone()).parse(html.to_string())?;
    let mut form = dom.form("#search")?;

    form.fill("q", "rust")?;

    form.submit().await?;

    let requests = transport.get_captured_requests();
    assert_eq!(requests.len(), 1);

    let req = &requests[0];
    assert_eq!(req.method, Method::Get);
    assert_eq!(req.url, "/search");
    Ok(())
}

#[tokio::test]
async fn test_form_default_method_is_get() -> Result<()> {
    let html = r#"
        <form id="form" action="/submit">
            <input name="data" type="text">
        </form>
    "#;

    let transport = MockTransport::new(default_response());
    let dom = Dom::new(transport.clone()).parse(html.to_string())?;
    let mut form = dom.form("#form")?;

    form.fill("data", "test")?;

    form.submit().await?;

    let requests = transport.get_captured_requests();
    let req = &requests[0];
    assert_eq!(req.method, Method::Get);
    Ok(())
}

#[tokio::test]
async fn test_multiple_forms_in_same_page() -> Result<()> {
    let html = r#"
        <form id="form1" action="/action1" method="post">
            <input name="field1" type="text">
        </form>
        <form id="form2" action="/action2" method="post">
            <input name="field2" type="text">
        </form>
    "#;

    let transport1 = MockTransport::new(default_response());
    let transport2 = MockTransport::new(default_response());

    let dom1 = Dom::new(transport1.clone()).parse(html.to_string())?;
    let mut form1 = dom1.form("#form1")?;
    form1.fill("field1", "value1")?;
    form1.submit().await?;

    let dom2 = Dom::new(transport2.clone()).parse(html.to_string())?;
    let mut form2 = dom2.form("#form2")?;
    form2.fill("field2", "value2")?;
    form2.submit().await?;

    let req1 = &transport1.get_captured_requests()[0];
    let req2 = &transport2.get_captured_requests()[0];

    assert_eq!(req1.url, "/action1");
    assert_eq!(req2.url, "/action2");
    Ok(())
}

#[tokio::test]
async fn test_form_fill_nonexistent_field() {
    let html = r#"
        <form id="form" action="/submit" method="post">
            <input name="username" type="text">
        </form>
    "#;

    let transport = MockTransport::new(default_response());
    let dom = Dom::new(transport).parse(html.to_string()).unwrap();
    let mut form = dom.form("#form").unwrap();

    let result = form.fill("nonexistent_field", "value");
    assert!(result.is_err());
    assert!(result.unwrap_err().to_string().contains("does not exist"));
}

#[tokio::test]
async fn test_form_fill_invalid_email() {
    let html = r#"
        <form id="form" action="/submit" method="post">
            <input name="email" type="email">
        </form>
    "#;

    let transport = MockTransport::new(default_response());
    let dom = Dom::new(transport).parse(html.to_string()).unwrap();
    let mut form = dom.form("#form").unwrap();

    let result = form.fill("email", "invalid-email");
    assert!(result.is_err());
    assert!(result
        .unwrap_err()
        .to_string()
        .contains("Invalid email format"));
}

#[tokio::test]
async fn test_form_fill_valid_email() -> Result<()> {
    let html = r#"
        <form id="form" action="/submit" method="post">
            <input name="email" type="email">
        </form>
    "#;

    let transport = MockTransport::new(default_response());
    let dom = Dom::new(transport.clone()).parse(html.to_string())?;
    let mut form = dom.form("#form")?;

    form.fill("email", "user@example.com")?;
    form.submit().await?;

    let requests = transport.get_captured_requests();
    let body = requests[0].body.as_ref().unwrap();
    assert!(body.contains("email=user@example.com"));
    Ok(())
}

#[tokio::test]
async fn test_form_fill_invalid_number() {
    let html = r#"
        <form id="form" action="/submit" method="post">
            <input name="age" type="number">
        </form>
    "#;

    let transport = MockTransport::new(default_response());
    let dom = Dom::new(transport).parse(html.to_string()).unwrap();
    let mut form = dom.form("#form").unwrap();

    let result = form.fill("age", "not-a-number");
    assert!(result.is_err());
    assert!(result
        .unwrap_err()
        .to_string()
        .contains("Invalid number format"));
}

#[tokio::test]
async fn test_form_fill_valid_number() -> Result<()> {
    let html = r#"
        <form id="form" action="/submit" method="post">
            <input name="age" type="number">
        </form>
    "#;

    let transport = MockTransport::new(default_response());
    let dom = Dom::new(transport.clone()).parse(html.to_string())?;
    let mut form = dom.form("#form")?;

    form.fill("age", "25")?;
    form.submit().await?;

    let requests = transport.get_captured_requests();
    let body = requests[0].body.as_ref().unwrap();
    assert!(body.contains("age=25"));
    Ok(())
}

#[tokio::test]
async fn test_form_fill_invalid_url() {
    let html = r#"
        <form id="form" action="/submit" method="post">
            <input name="website" type="url">
        </form>
    "#;

    let transport = MockTransport::new(default_response());
    let dom = Dom::new(transport).parse(html.to_string()).unwrap();
    let mut form = dom.form("#form").unwrap();

    let result = form.fill("website", "not-a-url");
    assert!(result.is_err());
    assert!(result
        .unwrap_err()
        .to_string()
        .contains("Invalid URL format"));
}

#[tokio::test]
async fn test_form_fill_valid_url() -> Result<()> {
    let html = r#"
        <form id="form" action="/submit" method="post">
            <input name="website" type="url">
        </form>
    "#;

    let transport = MockTransport::new(default_response());
    let dom = Dom::new(transport.clone()).parse(html.to_string())?;
    let mut form = dom.form("#form")?;

    form.fill("website", "https://example.com")?;
    form.submit().await?;

    let requests = transport.get_captured_requests();
    let body = requests[0].body.as_ref().unwrap();
    assert!(body.contains("website=https://example.com"));
    Ok(())
}

#[tokio::test]
async fn test_form_fill_invalid_tel() {
    let html = r#"
        <form id="form" action="/submit" method="post">
            <input name="phone" type="tel">
        </form>
    "#;

    let transport = MockTransport::new(default_response());
    let dom = Dom::new(transport).parse(html.to_string()).unwrap();
    let mut form = dom.form("#form").unwrap();

    let result = form.fill("phone", "abc-def-ghij");
    assert!(result.is_err());
    assert!(result
        .unwrap_err()
        .to_string()
        .contains("Invalid phone number format"));
}

#[tokio::test]
async fn test_form_fill_valid_tel() -> Result<()> {
    let html = r#"
        <form id="form" action="/submit" method="post">
            <input name="phone" type="tel">
        </form>
    "#;

    let transport = MockTransport::new(default_response());
    let dom = Dom::new(transport.clone()).parse(html.to_string())?;
    let mut form = dom.form("#form")?;

    form.fill("phone", "+1 (555) 123-4567")?;
    form.submit().await?;

    let requests = transport.get_captured_requests();
    let body = requests[0].body.as_ref().unwrap();
    assert!(body.contains("phone=+1 (555) 123-4567"));
    Ok(())
}

#[tokio::test]
async fn test_form_fill_invalid_date() {
    let html = r#"
        <form id="form" action="/submit" method="post">
            <input name="birthday" type="date">
        </form>
    "#;

    let transport = MockTransport::new(default_response());
    let dom = Dom::new(transport).parse(html.to_string()).unwrap();
    let mut form = dom.form("#form").unwrap();

    let result = form.fill("birthday", "2023/01/01");
    assert!(result.is_err());
    assert!(result
        .unwrap_err()
        .to_string()
        .contains("Invalid date format"));
}

#[tokio::test]
async fn test_form_fill_valid_date() -> Result<()> {
    let html = r#"
        <form id="form" action="/submit" method="post">
            <input name="birthday" type="date">
        </form>
    "#;

    let transport = MockTransport::new(default_response());
    let dom = Dom::new(transport.clone()).parse(html.to_string())?;
    let mut form = dom.form("#form")?;

    form.fill("birthday", "2023-01-01")?;
    form.submit().await?;

    let requests = transport.get_captured_requests();
    let body = requests[0].body.as_ref().unwrap();
    assert!(body.contains("birthday=2023-01-01"));
    Ok(())
}

#[tokio::test]
async fn test_form_check_checkbox() -> Result<()> {
    let html = r#"
        <form id="form" action="/submit" method="post">
            <input type="checkbox" name="interests" value="sports">
            <input type="checkbox" name="interests" value="music">
            <input type="checkbox" name="interests" value="reading">
        </form>
    "#;

    let transport = MockTransport::new(default_response());
    let dom = Dom::new(transport.clone()).parse(html.to_string())?;
    let mut form = dom.form("#form")?;

    form.check("interests", "sports")?;
    form.check("interests", "reading")?;
    form.submit().await?;

    let requests = transport.get_captured_requests();
    let body = requests[0].body.as_ref().unwrap();
    assert!(body.contains("interests=sports"));
    assert!(body.contains("interests=reading"));
    assert!(!body.contains("interests=music"));
    Ok(())
}

#[tokio::test]
async fn test_form_uncheck_checkbox() -> Result<()> {
    let html = r#"
        <form id="form" action="/submit" method="post">
            <input type="checkbox" name="agree" value="yes" checked>
        </form>
    "#;

    let transport = MockTransport::new(default_response());
    let dom = Dom::new(transport.clone()).parse(html.to_string())?;
    let form = dom.form("#form")?;

    // 初期状態ではchecked
    form.submit().await?;
    let requests = transport.get_captured_requests();
    assert!(requests[0].body.as_ref().unwrap().contains("agree=yes"));

    // uncheckしてから送信
    let transport2 = MockTransport::new(default_response());
    let dom2 = Dom::new(transport2.clone()).parse(html.to_string())?;
    let mut form2 = dom2.form("#form")?;
    form2.uncheck("agree", "yes")?;
    form2.submit().await?;

    let requests2 = transport2.get_captured_requests();
    assert!(!requests2[0].body.as_ref().unwrap().contains("agree=yes"));
    Ok(())
}

#[tokio::test]
async fn test_form_check_nonexistent_checkbox() {
    let html = r#"
        <form id="form" action="/submit" method="post">
            <input type="text" name="username">
        </form>
    "#;

    let transport = MockTransport::new(default_response());
    let dom = Dom::new(transport).parse(html.to_string()).unwrap();
    let mut form = dom.form("#form").unwrap();

    let result = form.check("nonexistent", "value");
    assert!(result.is_err());
    assert!(result.unwrap_err().to_string().contains("does not exist"));
}

#[tokio::test]
async fn test_form_choose_radio() -> Result<()> {
    let html = r#"
        <form id="form" action="/submit" method="post">
            <input type="radio" name="gender" value="male">
            <input type="radio" name="gender" value="female">
            <input type="radio" name="gender" value="other">
        </form>
    "#;

    let transport = MockTransport::new(default_response());
    let dom = Dom::new(transport.clone()).parse(html.to_string())?;
    let mut form = dom.form("#form")?;

    form.choose("gender", "female")?;
    form.submit().await?;

    let requests = transport.get_captured_requests();
    let body = requests[0].body.as_ref().unwrap();
    assert!(body.contains("gender=female"));
    assert!(!body.contains("gender=male"));
    assert!(!body.contains("gender=other"));
    Ok(())
}

#[tokio::test]
async fn test_form_choose_radio_overwrite() -> Result<()> {
    let html = r#"
        <form id="form" action="/submit" method="post">
            <input type="radio" name="size" value="small">
            <input type="radio" name="size" value="medium" checked>
            <input type="radio" name="size" value="large">
        </form>
    "#;

    let transport = MockTransport::new(default_response());
    let dom = Dom::new(transport.clone()).parse(html.to_string())?;
    let form = dom.form("#form")?;

    // 初期状態ではmediumが選択されている
    form.submit().await?;
    let requests = transport.get_captured_requests();
    assert!(requests[0].body.as_ref().unwrap().contains("size=medium"));

    // largeに変更
    let transport2 = MockTransport::new(default_response());
    let dom2 = Dom::new(transport2.clone()).parse(html.to_string())?;
    let mut form2 = dom2.form("#form")?;
    form2.choose("size", "large")?;
    form2.submit().await?;

    let requests2 = transport2.get_captured_requests();
    let body = requests2[0].body.as_ref().unwrap();
    assert!(body.contains("size=large"));
    assert!(!body.contains("size=medium"));
    Ok(())
}

#[tokio::test]
async fn test_form_choose_nonexistent_radio() {
    let html = r#"
        <form id="form" action="/submit" method="post">
            <input type="text" name="username">
        </form>
    "#;

    let transport = MockTransport::new(default_response());
    let dom = Dom::new(transport).parse(html.to_string()).unwrap();
    let mut form = dom.form("#form").unwrap();

    let result = form.choose("nonexistent", "value");
    assert!(result.is_err());
    assert!(result.unwrap_err().to_string().contains("does not exist"));
}

#[tokio::test]
async fn test_form_select_option() -> Result<()> {
    let html = r#"
        <form id="form" action="/submit" method="post">
            <select name="country">
                <option value="us">United States</option>
                <option value="uk">United Kingdom</option>
                <option value="jp">Japan</option>
            </select>
        </form>
    "#;

    let transport = MockTransport::new(default_response());
    let dom = Dom::new(transport.clone()).parse(html.to_string())?;
    let mut form = dom.form("#form")?;

    form.select("country", "jp")?;
    form.submit().await?;

    let requests = transport.get_captured_requests();
    let body = requests[0].body.as_ref().unwrap();
    assert!(body.contains("country=jp"));
    Ok(())
}

#[tokio::test]
async fn test_form_select_nonexistent() {
    let html = r#"
        <form id="form" action="/submit" method="post">
            <input type="text" name="username">
        </form>
    "#;

    let transport = MockTransport::new(default_response());
    let dom = Dom::new(transport).parse(html.to_string()).unwrap();
    let mut form = dom.form("#form").unwrap();

    let result = form.select("nonexistent", "value");
    assert!(result.is_err());
    assert!(result.unwrap_err().to_string().contains("does not exist"));
}

#[tokio::test]
async fn test_form_complex_submission() -> Result<()> {
    let html = r#"
        <form id="form" action="/submit" method="post">
            <input type="text" name="username">
            <input type="email" name="email">
            <input type="checkbox" name="notifications" value="email">
            <input type="checkbox" name="notifications" value="sms">
            <input type="radio" name="plan" value="free">
            <input type="radio" name="plan" value="premium">
            <select name="country">
                <option value="us">US</option>
                <option value="jp">JP</option>
            </select>
        </form>
    "#;

    let transport = MockTransport::new(default_response());
    let dom = Dom::new(transport.clone()).parse(html.to_string())?;
    let mut form = dom.form("#form")?;

    form.fill("username", "alice")?
        .fill("email", "alice@example.com")?
        .check("notifications", "email")?
        .check("notifications", "sms")?
        .choose("plan", "premium")?
        .select("country", "jp")?;

    form.submit().await?;

    let requests = transport.get_captured_requests();
    let body = requests[0].body.as_ref().unwrap();
    assert!(body.contains("username=alice"));
    assert!(body.contains("email=alice@example.com"));
    assert!(body.contains("notifications=email"));
    assert!(body.contains("notifications=sms"));
    assert!(body.contains("plan=premium"));
    assert!(body.contains("country=jp"));
    Ok(())
}

#[tokio::test]
async fn test_form_is_exist() -> Result<()> {
    let html = r#"
        <form id="form" action="/submit" method="post">
            <input type="text" name="username">
            <input type="email" name="email">
            <input type="checkbox" name="agree" value="yes">
            <input type="radio" name="gender" value="male">
            <select name="country">
                <option value="us">US</option>
            </select>
            <textarea name="bio"></textarea>
        </form>
    "#;

    let transport = MockTransport::new(default_response());
    let dom = Dom::new(transport).parse(html.to_string())?;
    let form = dom.form("#form")?;

    // 存在するフィールド
    assert!(form.is_exist("username"));
    assert!(form.is_exist("email"));
    assert!(form.is_exist("agree"));
    assert!(form.is_exist("gender"));
    assert!(form.is_exist("country"));
    assert!(form.is_exist("bio"));

    // 存在しないフィールド
    assert!(!form.is_exist("nonexistent"));
    assert!(!form.is_exist("password"));
    assert!(!form.is_exist(""));

    Ok(())
}

#[tokio::test]
async fn test_form_is_exist_with_hidden() -> Result<()> {
    let html = r#"
        <form id="form" action="/submit" method="post">
            <input type="hidden" name="_csrf" value="token">
            <input type="text" name="username">
        </form>
    "#;

    let transport = MockTransport::new(default_response());
    let dom = Dom::new(transport).parse(html.to_string())?;
    let form = dom.form("#form")?;

    // hiddenフィールドも検出できる
    assert!(form.is_exist("_csrf"));
    assert!(form.is_exist("username"));
    assert!(!form.is_exist("other"));

    Ok(())
}
