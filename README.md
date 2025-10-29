# lightdom-test

A lightweight Rust library for testing HTML interactions without browser automation.

---


## Quick Start

### 1) Prepare HTML

```rust
fn login_page() -> String {
    r#"
    <form id="login-form" action="/login" method="post">
        <input type="hidden" name="_csrf" value="fixed-token">
        <label for="u">User</label>
        <input id="u" type="text" name="username">
        <label for="p">Pass</label>
        <input id="p" type="password" name="password">
        <button type="submit">Login</button>
    </form>
    "#.to_string()
}
```

### 2) Implement MockTransport

```rust
use lightdom_test::{HttpTransport, HttpRequest, HttpResponse, StatusCode};
use anyhow::Result;

struct MockTransport;

#[async_trait::async_trait]
impl HttpTransport for MockTransport {
    async fn send(&self, req: HttpRequest) -> Result<HttpResponse> {
        // Return response based on request content
        if req.url == "/login" {
            if let Some(body) = &req.body {
                if body.contains("username=alice") && body.contains("password=secret") {
                    return Ok(HttpResponse {
                        status: StatusCode(200),
                        headers: Default::default(),
                        body: "Welcome, alice".to_string(),
                    });
                }
            }
            Ok(HttpResponse {
                status: StatusCode(401),
                headers: Default::default(),
                body: "Invalid credentials".to_string(),
            })
        } else {
            Ok(HttpResponse {
                status: StatusCode(404),
                headers: Default::default(),
                body: "Not Found".to_string(),
            })
        }
    }
}
```

### 3) Write Tests

```rust
use lightdom_test::Dom;

#[tokio::test]
async fn test_login_flow() -> Result<()> {
    let html = login_page();
    let transport = MockTransport;

    let mut form = Dom::new(transport)
        .parse(html)?
        .form("#login-form")?;

    form.fill("username", "alice")?
        .fill("password", "secret")?;

    let response = form.submit().await?;
    assert!(response.status.is_success());
    assert!(response.body.contains("Welcome, alice"));
    Ok(())
}
```


## API

### Dom
`Dom` is the entry point for parsing HTML documents and manipulating forms and buttons.

| Method | Type | Description |
|----------|------|------------------------------------|
| new | `(transport: impl HttpTransport) -> Dom` | Creates a new `Dom` instance. |
| parse | `(html: String) -> anyhow::Result<Dom>` | Parses an HTML string and returns a `Dom` instance. |
| form | `(locator: &str) -> anyhow::Result<Form>` | Gets a form based on the specified locator. |
| button | `(locator: &str) -> anyhow::Result<Button>` | Gets a button based on the specified locator. |
| link | `(locator: &str) -> anyhow::Result<Link>` | Gets a link based on the specified locator. |
| element | `(locator: &str) -> anyhow::Result<Element>` | Gets an element with the specified locator. |
| elements | `(locator: &str) -> Vec<Element>` | Gets all elements matching the specified locator. |
| text | `(locator: &str) -> anyhow::Result<String>` | Gets the text of the element with the specified locator. |
| texts | `(locator: &str) -> Vec<String>` | Gets the text of all elements matching the specified locator. |
| inner_html | `(locator: &str) -> anyhow::Result<String>` | Gets the inner HTML of the element with the specified locator. |
| table | `(locator: &str) -> anyhow::Result<Table>` | Gets the table with the specified locator. |
| list | `(locator: &str) -> anyhow::Result<List>` | Gets the list with the specified locator. |
| title | `() -> anyhow::Result<String>` | Gets the content of the `<title>` tag. |
| meta | `(name: &str) -> anyhow::Result<String>` | Gets the content attribute of `<meta name="...">` or `<meta property="...">`. |
| exists | `(locator: &str) -> bool` | Checks if an element with the specified locator exists. |
| contains_text | `(text: &str) -> bool` | Checks if an element containing the specified text exists. |
| select_element | `(locator: &str) -> anyhow::Result<SelectElement>` | Gets the select element with the specified locator. |
| image | `(locator: &str) -> anyhow::Result<Image>` | Gets the image with the specified locator. |
| images | `(locator: &str) -> Vec<Image>` | Gets all images matching the specified locator. |


Locator types that can be specified for `form`:

| Locator | Description |
|----------|------|
| @login-form | Identifies a form with `test-id` attribute `login-form`. |
| #login-form | Identifies a form with `id` attribute `login-form`. |
| /login | Identifies a form with `action` attribute `/login`. |

Locator types that can be specified for `button`:

| Locator | Description |
|----------|------|
| @submit-btn | Identifies a button with `test-id` attribute `submit-btn`. |
| #submit-btn | Identifies a button with `id` attribute `submit-btn`. |
| Login | Identifies a button with display text `Login`. |

Locator types that can be specified for `link`:

| Locator | Description |
|----------|------|
| @home-link | Identifies a link with `test-id` attribute `home-link`. |
| #home-link | Identifies a link with `id` attribute `home-link`. |
| Home | Identifies a link with display text `Home`. |

### Form

`Form` represents an HTML form and provides methods for filling fields and submitting the form.

| Method | Type | Description |
|----------|------|------------------------------------|
| is_exist | `(field_name: &str) -> bool` | Checks if the specified field exists in the form. |
| get_value | `(field_name: &str) -> anyhow::Result<String>` | Gets the current value of the specified field. |
| fill | `(field_name: &str, value: &str) -> anyhow::Result<&mut Form>` | Fills the specified field with a value. Returns an error if the field doesn't exist or the value doesn't match the field type. |
| check | `(field_name: &str, value: &str) -> anyhow::Result<&mut Form>` | Checks a checkbox. For checkboxes with multiple values, call this method multiple times to select multiple options. |
| uncheck | `(field_name: &str, value: &str) -> anyhow::Result<&mut Form>` | Unchecks a checkbox. |
| choose | `(field_name: &str, value: &str) -> anyhow::Result<&mut Form>` | Selects a radio button. Other radio buttons with the same name attribute are automatically deselected. |
| select | `(field_name: &str, value: &str) -> anyhow::Result<&mut Form>` | Selects an option in a select box. |
| submit | `(&self) -> anyhow::Result<HttpResponse>` | Submits the form and returns an HTTP response. |

#### fill Method Validation

The `fill` method automatically validates input values based on the `type` attribute of the field:

| type attribute | Validation |
|-----------|-------------------|
| email | Checks if it contains `@` |
| number | Checks if it can be parsed as a number |
| url | Checks if it starts with `http://` or `https://` |
| tel | Allows only digits, hyphens, spaces, parentheses, and `+` |
| date | Checks if it's in `YYYY-MM-DD` format |
| text, password, hidden, textarea, select, etc. | No validation |

```rust
// Valid cases
form.fill("email", "user@example.com")?;  // OK
form.fill("age", "25")?;                   // OK

// Error cases
form.fill("email", "invalid-email")?;     // Err: Invalid email format
form.fill("age", "not-a-number")?;         // Err: Invalid number format
form.fill("nonexistent", "value")?;        // Err: Field does not exist
```

#### is_exist Method Usage Example

```rust
// Check field existence
if form.is_exist("username") {
    form.fill("username", "alice")?;
}

// Conditional processing
if form.is_exist("email") && form.is_exist("phone") {
    // Fill only if both fields exist
    form.fill("email", "alice@example.com")?
        .fill("phone", "123-456-7890")?;
}
```

#### Checkbox, Radio Button, and Select Box Usage Examples

```rust
// Checkbox (multiple selection)
form.check("interests", "sports")?
    .check("interests", "music")?;

// Uncheck checkbox
form.uncheck("agree", "terms")?;

// Radio button (single selection)
form.choose("gender", "female")?;

// Select box
form.select("country", "japan")?;

// Combined usage example
form.fill("username", "alice")?
    .fill("email", "alice@example.com")?
    .check("notifications", "email")?
    .check("notifications", "sms")?
    .choose("plan", "premium")?
    .select("country", "jp")?
    .submit().await?;
```

### Button
`Button` represents an HTML button and provides methods for clicking.

| Method | Type | Description |
|----------|------|------------------------------------|
| click | `(&self) -> anyhow::Result<HttpResponse>` | Clicks the button and submits the associated form. Returns an HTTP response. |

#### Usage Example
```rust
let button = dom.button("#submit-btn")?;
let response = button.click().await?;
assert!(response.status.is_success());
```

### Link
`Link` represents an HTML link and provides methods for clicking.

| Method | Type | Description |
|----------|------|------------------------------------|
| click | `(&self) -> anyhow::Result<HttpResponse>` | Clicks the link and sends a GET request to the href destination. Returns an HTTP response. |

#### Usage Example
```rust
let link = dom.link("Home")?;
let response = link.click().await?;
assert_eq!(response.status.0, 200);
```

## Data Retrieval APIs

The data retrieval APIs provide functionality for extracting data from HTML content.

### Table
`Table` is an API for getting data from HTML tables (`<table>`).

| Method | Type | Description |
|----------|------|------------------------------------|
| headers | `() -> Vec<String>` | Gets the table headers (th elements). |
| rows | `() -> Vec<Row>` | Gets all rows of the table. |
| row | `(index: usize) -> anyhow::Result<Row>` | Gets the row at the specified index. |
| cell | `(row: usize, col: usize) -> anyhow::Result<String>` | Gets the text of the cell at the specified row and column. |
| find_row | `(column: &str, value: &str) -> anyhow::Result<Row>` | Searches for a row where the specified column value matches. |

#### Row
`Row` represents one row of a table.

| Method | Type | Description |
|----------|------|------------------------------------|
| cells | `() -> Vec<String>` | Gets the text of all cells in the row. |
| cell | `(index: usize) -> anyhow::Result<String>` | Gets the text of the cell at the specified index. |
| get | `(column: &str) -> anyhow::Result<String>` | Gets the text of a cell by specifying the header name. |

#### Usage Example
```rust
let table = dom.table("#users-table")?;

// Get headers
let headers = table.headers();
assert_eq!(headers, vec!["Name", "Email", "Status"]);

// Get all rows
for row in table.rows() {
    let cells = row.cells();
    println!("{:?}", cells);
}

// Access specific cell
let name = table.cell(0, 0)?; // Row 1, Column 1
assert_eq!(name, "Alice");

// Search row by column name
let row = table.find_row("Email", "alice@example.com")?;
let status = row.get("Status")?;
assert_eq!(status, "Active");
```

### List
`List` is an API for getting data from HTML lists (`<ul>`, `<ol>`).

| Method | Type | Description |
|----------|------|------------------------------------|
| items | `() -> Vec<String>` | Gets the text of all list items. |
| item | `(index: usize) -> anyhow::Result<String>` | Gets the text of the item at the specified index. |
| len | `() -> usize` | Returns the number of list items. |
| contains | `(text: &str) -> bool` | Checks if an item containing the specified text exists. |

#### Usage Example
```rust
let list = dom.list("#todo-list")?;

// Get all items
let items = list.items();
assert_eq!(items.len(), 3);

// Access specific item
let first = list.item(0)?;
assert_eq!(first, "Buy groceries");

// Check item existence
assert!(list.contains("Buy groceries"));
```

### Text
`Text` is an API for getting text content from HTML elements.

| Method | Type | Description |
|----------|------|------------------------------------|
| text | `(locator: &str) -> anyhow::Result<String>` | Gets the text of the element with the specified locator. |
| texts | `(locator: &str) -> Vec<String>` | Gets the text of all elements matching the specified locator. |
| inner_html | `(locator: &str) -> anyhow::Result<String>` | Gets the inner HTML of the element with the specified locator. |

Locator types that can be specified for `text`:

| Locator | Description |
|----------|------|
| @message | Identifies an element with `test-id` attribute `message`. |
| #message | Identifies an element with `id` attribute `message`. |
| .message | Identifies an element with `class` attribute `message`. |

#### Usage Example
```rust
let dom = Dom::new(transport).parse(html)?;

// Get text from single element
let message = dom.text("#welcome-message")?;
assert_eq!(message, "Welcome, Alice!");

// Get text from multiple elements
let errors = dom.texts(".error-message");
assert_eq!(errors, vec!["Invalid email", "Password too short"]);

// Get inner HTML
let content = dom.inner_html("#content")?;
assert!(content.contains("<p>"));
```

### Element
`Element` provides generic element retrieval and attribute access.

| Method | Type | Description |
|----------|------|------------------------------------|
| element | `(locator: &str) -> anyhow::Result<Element>` | Gets the element with the specified locator. |
| elements | `(locator: &str) -> Vec<Element>` | Gets all elements matching the specified locator. |

#### Element
`Element` represents a retrieved element.

| Method | Type | Description |
|----------|------|------------------------------------|
| text | `() -> String` | Gets the text content of the element. |
| attr | `(name: &str) -> Option<String>` | Gets the value of the specified attribute. |
| has_class | `(class: &str) -> bool` | Checks if the element has the specified class. |
| inner_html | `() -> String` | Gets the inner HTML of the element. |
| text_contains | `(text: &str) -> bool` | Checks if the element's text contains the specified string. |
| is_disabled | `() -> bool` | Checks if the element has the disabled attribute. |
| is_required | `() -> bool` | Checks if the element has the required attribute. |
| is_readonly | `() -> bool` | Checks if the element has the readonly attribute. |
| is_checked | `() -> bool` | Checks if the element has the checked attribute. |

#### Usage Example
```rust
let element = dom.element("#user-profile")?;

// Get text
let text = element.text();

// Get attribute
let user_id = element.attr("data-user-id");
assert_eq!(user_id, Some("123".to_string()));

// Check class
assert!(element.has_class("active"));

// Process multiple elements
for elem in dom.elements(".product-item") {
    let name = elem.attr("data-name").unwrap();
    let price = elem.text();
    println!("{}: {}", name, price);
}
```

### Meta Tags
`Dom` provides APIs for getting meta tags and title tags. Useful for SEO testing in SSR applications.

| Method | Type | Description |
|----------|------|------------------------------------|
| title | `() -> anyhow::Result<String>` | Gets the content of the `<title>` tag. |
| meta | `(name: &str) -> anyhow::Result<String>` | Gets the content attribute of `<meta name="...">` or `<meta property="...">`. |

#### Usage Example
```rust
let dom = Dom::new(transport).parse(html)?;

// Get title
let title = dom.title()?;
assert_eq!(title, "Welcome - My Site");

// Get meta tag
let description = dom.meta("description")?;
assert_eq!(description, "This is my website");

// Get OGP tag
let og_title = dom.meta("og:title")?;
assert_eq!(og_title, "Welcome");
```

### Exists Check
API for checking element existence.

| Method | Type | Description |
|----------|------|------------------------------------|
| exists | `(locator: &str) -> bool` | Checks if an element with the specified locator exists. |
| contains_text | `(text: &str) -> bool` | Checks if an element containing the specified text exists. |

#### Usage Example
```rust
// Check element existence
assert!(dom.exists("#error-message"));
assert!(!dom.exists("#success-message"));

// Check text existence
assert!(dom.contains_text("Welcome"));
assert!(!dom.contains_text("Error"));
```

### Select Element
`SelectElement` is an API for getting options from `<select>` elements.

| Method | Type | Description |
|----------|------|------------------------------------|
| select_element | `(locator: &str) -> Result<SelectElement>` | Gets the select element with the specified locator. |

#### SelectElement
| Method | Type | Description |
|----------|------|------------------------------------|
| options | `() -> Vec<SelectOption>` | Gets all options. |
| selected_option | `() -> Result<SelectOption>` | Gets the selected option. |

#### SelectOption
| Method | Type | Description |
|----------|------|------------------------------------|
| value | `() -> String` | Gets the value attribute of the option. |
| text | `() -> String` | Gets the display text of the option. |
| is_selected | `() -> bool` | Checks if the option is selected. |

#### Usage Example
```rust
let select = dom.select_element("#country")?;

// Get all options
let options = select.options();
assert_eq!(options.len(), 3);
assert_eq!(options[0].value(), "jp");
assert_eq!(options[0].text(), "Japan");

// Get selected option
let selected = select.selected_option()?;
assert_eq!(selected.value(), "us");
assert!(selected.is_selected());
```

### Image
`Dom` provides APIs for getting image elements.

| Method | Type | Description |
|----------|------|------------------------------------|
| image | `(locator: &str) -> Result<Image>` | Gets the image with the specified locator. |
| images | `(locator: &str) -> Vec<Image>` | Gets all images matching the specified locator. |

#### Image
| Method | Type | Description |
|----------|------|------------------------------------|
| src | `() -> String` | Gets the src attribute of the image. |
| alt | `() -> Option<String>` | Gets the alt attribute of the image. |
| width | `() -> Option<String>` | Gets the width attribute of the image. |
| height | `() -> Option<String>` | Gets the height attribute of the image. |

#### Usage Example
```rust
let img = dom.image("#logo")?;
assert_eq!(img.src(), "/logo.png");
assert_eq!(img.alt(), Some("Company Logo".to_string()));

// Get all images
let images = dom.images("img");
for img in images {
    println!("{}: {}", img.src(), img.alt().unwrap_or_default());
}
```

## Transport Layer

`lightdom-test` abstracts HTTP sending operations into the `HttpTransport` trait. This allows you to use it with any HTTP client or framework.

### HttpTransport Trait
`HttpTransport` is a trait for sending HTTP requests. Use it when implementing your own HTTP client.

```rust
#[async_trait::async_trait]
pub trait HttpTransport: Send + Sync {
    async fn send(&self, req: HttpRequest) -> anyhow::Result<HttpResponse>;
}
```

#### HttpRequest
`HttpRequest` is a struct representing an HTTP request.
```rust
pub struct HttpRequest {
    pub method: Method,
    pub url: String,
    pub headers: HashMap<String, String>,
    pub body: Option<String>,
}
```

#### HttpResponse
`HttpResponse` is a struct representing an HTTP response.
```rust
pub struct HttpResponse {
    pub status: StatusCode,
    pub headers: HashMap<String, String>,
    pub body: String,
}
```

### Transport Implementation Examples

You can use MockTransport for testing and an actual HTTP client (e.g., reqwest) for production:

```rust
use std::sync::{Arc, Mutex};

// For testing: MockTransport that captures requests
#[derive(Clone)]
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

// For production: Implementation using reqwest
struct ReqwestTransport {
    client: reqwest::Client,
    base_url: String,
}

#[async_trait::async_trait]
impl HttpTransport for ReqwestTransport {
    async fn send(&self, req: HttpRequest) -> Result<HttpResponse> {
        let url = format!("{}{}", self.base_url, req.url);
        let method = match req.method {
            Method::Get => reqwest::Method::GET,
            Method::Post => reqwest::Method::POST,
        };

        let response = self.client
            .request(method, &url)
            .body(req.body.unwrap_or_default())
            .send()
            .await?;

        Ok(HttpResponse {
            status: StatusCode(response.status().as_u16()),
            headers: Default::default(),
            body: response.text().await?,
        })
    }
}
```

## Framework Integration

`lightdom-test` provides integration with major Rust web frameworks as optional features.

### Axum Integration

If you're using the Axum framework, you can use `AxumTransport` to test your Router directly without starting an HTTP server.

#### Enable

Add the `axum` feature to your `Cargo.toml`:

```toml
[dev-dependencies]
lightdom-test = { version = "0.1", features = ["axum"] }
axum = "0.7"
tokio = { version = "1", features = ["full"] }
```

#### Usage Example

```rust
use axum::{Router, routing::post, Form};
use lightdom_test::{Dom, transports::AxumTransport};
use serde::Deserialize;

#[derive(Deserialize)]
struct LoginForm {
    username: String,
    password: String,
}

async fn login_handler(Form(form): Form<LoginForm>) -> String {
    if form.username == "alice" && form.password == "secret" {
        "Welcome, alice".to_string()
    } else {
        "Invalid credentials".to_string()
    }
}

#[tokio::test]
async fn test_login() {
    // Create Axum Router
    let app = Router::new()
        .route("/login", post(login_handler));

    // Use AxumTransport
    let transport = AxumTransport::new(app);

    let html = r#"
        <form action="/login" method="post">
            <input name="username" type="text">
            <input name="password" type="password">
        </form>
    "#;

    let mut form = Dom::new(transport)
        .parse(html.to_string())
        .unwrap()
        .form("/login")
        .unwrap();

    form.fill("username", "alice").unwrap()
        .fill("password", "secret").unwrap();

    let response = form.submit().await.unwrap();
    assert!(response.body.contains("Welcome, alice"));
}
```

#### Benefits

- **Fast**: Tests run quickly without needing to start an HTTP server
- **No port management**: No worries about random port allocation or port conflicts
- **Simple**: Just pass your `Router` directly

### Rocket Integration

If you're using the Rocket framework, you can use `RocketTransport` to test your Rocket instance directly without starting an HTTP server.

#### Enable

Add the `rocket` feature to your `Cargo.toml`:

```toml
[dev-dependencies]
lightdom-test = { version = "0.1", features = ["rocket"] }
rocket = "0.5"
tokio = { version = "1", features = ["full"] }
```

#### Usage Example

```rust
use rocket::{routes, post, form::Form};
use lightdom_test::{Dom, transports::RocketTransport};

#[derive(rocket::form::FromForm)]
struct LoginForm {
    username: String,
    password: String,
}

#[post("/login", data = "<form>")]
async fn login_handler(form: Form<LoginForm>) -> String {
    if form.username == "alice" && form.password == "secret" {
        "Welcome, alice".to_string()
    } else {
        "Invalid credentials".to_string()
    }
}

#[tokio::test]
async fn test_login() {
    // Create Rocket instance
    let rocket = rocket::build()
        .mount("/", routes![login_handler]);

    // Use RocketTransport
    let transport = RocketTransport::new(rocket).await.unwrap();

    let html = r#"
        <form action="/login" method="post">
            <input name="username" type="text">
            <input name="password" type="password">
        </form>
    "#;

    let mut form = Dom::new(transport)
        .parse(html.to_string())
        .unwrap()
        .form("/login")
        .unwrap();

    form.fill("username", "alice").unwrap()
        .fill("password", "secret").unwrap();

    let response = form.submit().await.unwrap();
    assert!(response.body.contains("Welcome, alice"));
}
```

#### Benefits

- **Fast**: Tests run quickly without needing to start an HTTP server
- **No port management**: No worries about random port allocation or port conflicts
- **Full Rocket features**: Test all Rocket features including middleware, Fairings, and State


## Philosophy

- **Lightweight & Fast**: Enables simple and fast testing without using large browser automation tools.
- **Rust Native**: Designed to integrate seamlessly with the Rust ecosystem.
- **Simplicity**: Provides an intuitive and easy-to-use API with minimal learning curve.
- **Flexibility**: Designed to work with any HTTP client or framework.
