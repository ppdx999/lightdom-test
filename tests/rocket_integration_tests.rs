#![cfg(feature = "rocket")]

use anyhow::Result;
use lightdom_test::{transports::RocketTransport, Dom};
use rocket::{form::Form, routes};

// ============================================
// Form Handlers
// ============================================

#[derive(rocket::form::FromForm)]
struct LoginForm {
    username: String,
    password: String,
    #[field(default = String::new())]
    _csrf: String,
}

#[rocket::post("/login", data = "<form>")]
async fn login_handler(form: Form<LoginForm>) -> String {
    if form.username == "alice" && form.password == "secret" {
        "<h1>Welcome, alice</h1>".to_string()
    } else {
        "<h1>Invalid credentials</h1>".to_string()
    }
}

#[derive(rocket::form::FromForm)]
struct RegistrationForm {
    email: String,
    gender: String,
    country: String,
}

#[rocket::post("/register", data = "<form>")]
async fn register_handler(form: Form<RegistrationForm>) -> String {
    format!(
        "<p>Registered: {}</p><p>Gender: {}</p><p>Country: {}</p>",
        form.email, form.gender, form.country
    )
}

#[rocket::get("/")]
async fn home_handler() -> &'static str {
    "<h1>Home Page</h1>"
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
async fn test_rocket_login_success() -> Result<()> {
    let rocket = rocket::build().mount("/", routes![login_handler]);
    let transport = RocketTransport::new(rocket).await?;

    let html = login_page();
    let mut form = Dom::new(transport).parse(html)?.form("#login-form")?;

    form.fill("username", "alice")?.fill("password", "secret")?;

    let response = form.submit().await?;

    assert!(response.status.is_success());
    assert!(response.body.contains("Welcome, alice"));
    Ok(())
}

#[tokio::test]
async fn test_rocket_login_failure() -> Result<()> {
    let rocket = rocket::build().mount("/", routes![login_handler]);
    let transport = RocketTransport::new(rocket).await?;

    let html = login_page();
    let mut form = Dom::new(transport).parse(html)?.form("#login-form")?;

    form.fill("username", "alice")?.fill("password", "wrong")?;

    let response = form.submit().await?;

    assert!(response.status.is_success());
    assert!(response.body.contains("Invalid credentials"));
    Ok(())
}

#[tokio::test]
async fn test_rocket_hidden_fields() -> Result<()> {
    let rocket = rocket::build().mount("/", routes![login_handler]);
    let transport = RocketTransport::new(rocket).await?;

    let html = login_page();
    let form = Dom::new(transport).parse(html)?.form("#login-form")?;

    let csrf = form.get_value("_csrf")?;
    assert_eq!(csrf, "token123");
    Ok(())
}

#[tokio::test]
async fn test_rocket_complex_form() -> Result<()> {
    let rocket = rocket::build().mount("/", routes![register_handler]);
    let transport = RocketTransport::new(rocket).await?;

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
async fn test_rocket_link_click() -> Result<()> {
    let rocket = rocket::build().mount("/", routes![home_handler]);
    let transport = RocketTransport::new(rocket).await?;

    let html = page_with_links();
    let dom = Dom::new(transport).parse(html)?;

    let link = dom.link("#home-link")?;
    let response = link.click().await?;

    assert!(response.status.is_success());
    assert!(response.body.contains("Home Page"));
    Ok(())
}

#[tokio::test]
async fn test_rocket_form_validation() -> Result<()> {
    let rocket = rocket::build().mount("/", routes![register_handler]);
    let transport = RocketTransport::new(rocket).await?;

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
