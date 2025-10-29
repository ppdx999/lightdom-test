#![cfg(feature = "axum")]

use anyhow::Result;
use axum::{
    extract::Form,
    response::{Html, IntoResponse},
    routing::{get, post},
    Router,
};
use lightdom_test::{transports::AxumTransport, Dom};
use serde::Deserialize;

// ============================================
// Form Handlers
// ============================================

#[derive(Deserialize)]
struct LoginForm {
    username: String,
    password: String,
    #[serde(default)]
    _csrf: String,
}

async fn login_handler(Form(form): Form<LoginForm>) -> impl IntoResponse {
    if form.username == "alice" && form.password == "secret" {
        Html("<h1>Welcome, alice</h1>")
    } else {
        Html("<h1>Invalid credentials</h1>")
    }
}

#[derive(Deserialize)]
struct RegistrationForm {
    email: String,
    gender: String,
    country: String,
}

async fn register_handler(Form(form): Form<RegistrationForm>) -> impl IntoResponse {
    Html(format!(
        "<p>Registered: {}</p><p>Gender: {}</p><p>Country: {}</p>",
        form.email, form.gender, form.country
    ))
}

async fn home_handler() -> impl IntoResponse {
    Html("<h1>Home Page</h1>")
}

// ============================================
// Helper Functions
// ============================================

fn login_page() -> String {
    r#"
    <html>
    <body>
        <form id="login-form" action="/login" method="post">
            <input type="hidden" name="_csrf" value="token123">
            <label for="u">Username</label>
            <input id="u" type="text" name="username">
            <label for="p">Password</label>
            <input id="p" type="password" name="password">
            <button type="submit">Login</button>
        </form>
    </body>
    </html>
    "#
    .to_string()
}

fn registration_page() -> String {
    r#"
    <html>
    <body>
        <form id="register-form" action="/register" method="post">
            <input type="email" name="email">
            <input type="checkbox" name="notifications" value="email">
            <input type="checkbox" name="notifications" value="sms">
            <input type="radio" name="gender" value="male">
            <input type="radio" name="gender" value="female">
            <select name="country">
                <option value="us">United States</option>
                <option value="jp">Japan</option>
                <option value="uk">United Kingdom</option>
            </select>
            <button type="submit">Register</button>
        </form>
    </body>
    </html>
    "#
    .to_string()
}

fn page_with_links() -> String {
    r#"
    <html>
    <body>
        <h1>Welcome</h1>
        <a href="/" id="home-link">Home</a>
    </body>
    </html>
    "#
    .to_string()
}

// ============================================
// Tests
// ============================================

#[tokio::test]
async fn test_axum_login_success() -> Result<()> {
    let app = Router::new().route("/login", post(login_handler));
    let transport = AxumTransport::new(app);

    let html = login_page();
    let mut form = Dom::new(transport).parse(html)?.form("#login-form")?;

    form.fill("username", "alice")?.fill("password", "secret")?;

    let response = form.submit().await?;

    assert!(response.status.is_success());
    assert!(response.body.contains("Welcome, alice"));
    Ok(())
}

#[tokio::test]
async fn test_axum_login_failure() -> Result<()> {
    let app = Router::new().route("/login", post(login_handler));
    let transport = AxumTransport::new(app);

    let html = login_page();
    let mut form = Dom::new(transport).parse(html)?.form("#login-form")?;

    form.fill("username", "alice")?.fill("password", "wrong")?;

    let response = form.submit().await?;

    assert!(response.status.is_success());
    assert!(response.body.contains("Invalid credentials"));
    Ok(())
}

#[tokio::test]
async fn test_axum_hidden_fields() -> Result<()> {
    let app = Router::new().route("/login", post(login_handler));
    let transport = AxumTransport::new(app);

    let html = login_page();
    let form = Dom::new(transport).parse(html)?.form("#login-form")?;

    // hidden フィールドは自動的に収集される
    let csrf = form.get_value("_csrf")?;
    assert_eq!(csrf, "token123");
    Ok(())
}

#[tokio::test]
async fn test_axum_complex_form() -> Result<()> {
    let app = Router::new().route("/register", post(register_handler));
    let transport = AxumTransport::new(app);

    let html = registration_page();
    let mut form = Dom::new(transport).parse(html)?.form("#register-form")?;

    form.fill("email", "alice@example.com")?
        .choose("gender", "female")?
        .select("country", "jp")?;

    let response = form.submit().await?;

    assert!(response.status.is_success());
    assert!(response.body.contains("alice@example.com"));
    assert!(response.body.contains("female"));
    assert!(response.body.contains("jp"));
    Ok(())
}

#[tokio::test]
async fn test_axum_button_via_form_submit() -> Result<()> {
    // Button::click() has design limitations:
    // it can only send default/hidden field values, not values set via Form::fill()
    // For complete form submission with filled values, use Form::submit() instead
    let app = Router::new().route("/login", post(login_handler));
    let transport = AxumTransport::new(app);

    let html = login_page();
    let dom = Dom::new(transport).parse(html)?;

    // Recommended approach: use Form::submit() after filling values
    let mut form = dom.form("#login-form")?;
    form.fill("username", "alice")?.fill("password", "secret")?;

    let response = form.submit().await?;

    assert!(response.status.is_success());
    assert!(response.body.contains("Welcome, alice"));
    Ok(())
}

#[tokio::test]
async fn test_axum_link_click() -> Result<()> {
    let app = Router::new().route("/", get(home_handler));
    let transport = AxumTransport::new(app);

    let html = page_with_links();
    let dom = Dom::new(transport).parse(html)?;

    let link = dom.link("#home-link")?;
    let response = link.click().await?;

    assert!(response.status.is_success());
    assert!(response.body.contains("Home Page"));
    Ok(())
}

#[tokio::test]
async fn test_axum_form_validation() -> Result<()> {
    let app = Router::new().route("/register", post(register_handler));
    let transport = AxumTransport::new(app);

    let html = registration_page();
    let mut form = Dom::new(transport).parse(html)?.form("#register-form")?;

    // email のバリデーション
    let result = form.fill("email", "invalid-email");
    assert!(result.is_err());
    assert!(result
        .unwrap_err()
        .to_string()
        .contains("Invalid email format"));

    // 正しい email でフォームを送信
    form.fill("email", "test@example.com")?
        .choose("gender", "male")?
        .select("country", "us")?;
    let response = form.submit().await?;

    assert!(response.status.is_success());
    assert!(response.body.contains("test@example.com"));
    Ok(())
}
