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
// Element API Tests
// ============================================

#[tokio::test]
async fn test_element_by_id() -> Result<()> {
    let html = r#"
        <div id="content">Hello World</div>
    "#;

    let transport = MockTransport::new(default_response());
    let dom = Dom::new(transport).parse(html.to_string())?;
    let element = dom.element("#content")?;

    assert_eq!(element.text().trim(), "Hello World");
    Ok(())
}

#[tokio::test]
async fn test_element_by_test_id() -> Result<()> {
    let html = r#"
        <div test-id="main-content">Test Content</div>
    "#;

    let transport = MockTransport::new(default_response());
    let dom = Dom::new(transport).parse(html.to_string())?;
    let element = dom.element("@main-content")?;

    assert_eq!(element.text().trim(), "Test Content");
    Ok(())
}

#[tokio::test]
async fn test_element_by_class() -> Result<()> {
    let html = r#"
        <div class="container">Container Content</div>
    "#;

    let transport = MockTransport::new(default_response());
    let dom = Dom::new(transport).parse(html.to_string())?;
    let element = dom.element(".container")?;

    assert_eq!(element.text().trim(), "Container Content");
    Ok(())
}

#[tokio::test]
async fn test_elements_multiple() -> Result<()> {
    let html = r#"
        <div class="item">Item 1</div>
        <div class="item">Item 2</div>
        <div class="item">Item 3</div>
    "#;

    let transport = MockTransport::new(default_response());
    let dom = Dom::new(transport).parse(html.to_string())?;
    let elements = dom.elements(".item");

    assert_eq!(elements.len(), 3);
    assert_eq!(elements[0].text().trim(), "Item 1");
    assert_eq!(elements[1].text().trim(), "Item 2");
    assert_eq!(elements[2].text().trim(), "Item 3");
    Ok(())
}

#[tokio::test]
async fn test_element_attr() -> Result<()> {
    let html = r#"
        <a id="link" href="/page" title="Go to page">Link</a>
    "#;

    let transport = MockTransport::new(default_response());
    let dom = Dom::new(transport).parse(html.to_string())?;
    let element = dom.element("#link")?;

    assert_eq!(element.attr("href"), Some("/page".to_string()));
    assert_eq!(element.attr("title"), Some("Go to page".to_string()));
    assert_eq!(element.attr("nonexistent"), None);
    Ok(())
}

#[tokio::test]
async fn test_element_has_class() -> Result<()> {
    let html = r#"
        <div id="box" class="container primary active">Content</div>
    "#;

    let transport = MockTransport::new(default_response());
    let dom = Dom::new(transport).parse(html.to_string())?;
    let element = dom.element("#box")?;

    assert!(element.has_class("container"));
    assert!(element.has_class("primary"));
    assert!(element.has_class("active"));
    assert!(!element.has_class("hidden"));
    Ok(())
}

#[tokio::test]
async fn test_element_inner_html() -> Result<()> {
    let html = r#"
        <div id="wrapper"><span>Hello</span> <strong>World</strong></div>
    "#;

    let transport = MockTransport::new(default_response());
    let dom = Dom::new(transport).parse(html.to_string())?;
    let element = dom.element("#wrapper")?;

    let inner = element.inner_html();
    assert!(inner.contains("<span>Hello</span>"));
    assert!(inner.contains("<strong>World</strong>"));
    Ok(())
}

#[tokio::test]
async fn test_element_not_found() {
    let html = r#"
        <div id="content">Content</div>
    "#;

    let transport = MockTransport::new(default_response());
    let dom = Dom::new(transport).parse(html.to_string()).unwrap();
    let result = dom.element("#nonexistent");

    assert!(result.is_err());
    assert!(result.unwrap_err().to_string().contains("Element not found"));
}

// ============================================
// Text API Tests
// ============================================

#[tokio::test]
async fn test_text_single() -> Result<()> {
    let html = r#"
        <p id="message">Hello World</p>
    "#;

    let transport = MockTransport::new(default_response());
    let dom = Dom::new(transport).parse(html.to_string())?;
    let text = dom.text("#message")?;

    assert_eq!(text.trim(), "Hello World");
    Ok(())
}

#[tokio::test]
async fn test_texts_multiple() -> Result<()> {
    let html = r#"
        <p class="para">First paragraph</p>
        <p class="para">Second paragraph</p>
        <p class="para">Third paragraph</p>
    "#;

    let transport = MockTransport::new(default_response());
    let dom = Dom::new(transport).parse(html.to_string())?;
    let texts = dom.texts(".para");

    assert_eq!(texts.len(), 3);
    assert_eq!(texts[0].trim(), "First paragraph");
    assert_eq!(texts[1].trim(), "Second paragraph");
    assert_eq!(texts[2].trim(), "Third paragraph");
    Ok(())
}

#[tokio::test]
async fn test_inner_html_from_dom() -> Result<()> {
    let html = r#"
        <div id="content"><h1>Title</h1><p>Paragraph</p></div>
    "#;

    let transport = MockTransport::new(default_response());
    let dom = Dom::new(transport).parse(html.to_string())?;
    let inner = dom.inner_html("#content")?;

    assert!(inner.contains("<h1>Title</h1>"));
    assert!(inner.contains("<p>Paragraph</p>"));
    Ok(())
}

// ============================================
// Table API Tests
// ============================================

#[tokio::test]
async fn test_table_headers() -> Result<()> {
    let html = r#"
        <table id="users">
            <thead>
                <tr>
                    <th>Name</th>
                    <th>Email</th>
                    <th>Age</th>
                </tr>
            </thead>
            <tbody>
                <tr>
                    <td>Alice</td>
                    <td>alice@example.com</td>
                    <td>25</td>
                </tr>
            </tbody>
        </table>
    "#;

    let transport = MockTransport::new(default_response());
    let dom = Dom::new(transport).parse(html.to_string())?;
    let table = dom.table("#users")?;

    let headers = table.headers();
    assert_eq!(headers.len(), 3);
    assert_eq!(headers[0], "Name");
    assert_eq!(headers[1], "Email");
    assert_eq!(headers[2], "Age");
    Ok(())
}

#[tokio::test]
async fn test_table_rows() -> Result<()> {
    let html = r#"
        <table id="data">
            <tr>
                <th>A</th>
                <th>B</th>
            </tr>
            <tr>
                <td>1</td>
                <td>2</td>
            </tr>
            <tr>
                <td>3</td>
                <td>4</td>
            </tr>
        </table>
    "#;

    let transport = MockTransport::new(default_response());
    let dom = Dom::new(transport).parse(html.to_string())?;
    let table = dom.table("#data")?;

    let rows = table.rows();
    assert_eq!(rows.len(), 2);

    let row0_cells = rows[0].cells();
    assert_eq!(row0_cells[0], "1");
    assert_eq!(row0_cells[1], "2");

    let row1_cells = rows[1].cells();
    assert_eq!(row1_cells[0], "3");
    assert_eq!(row1_cells[1], "4");
    Ok(())
}

#[tokio::test]
async fn test_table_row() -> Result<()> {
    let html = r#"
        <table id="products">
            <tr>
                <th>Product</th>
                <th>Price</th>
            </tr>
            <tr>
                <td>Apple</td>
                <td>$1.00</td>
            </tr>
            <tr>
                <td>Banana</td>
                <td>$0.50</td>
            </tr>
        </table>
    "#;

    let transport = MockTransport::new(default_response());
    let dom = Dom::new(transport).parse(html.to_string())?;
    let table = dom.table("#products")?;

    let row = table.row(0)?;
    assert_eq!(row.cell(0)?, "Apple");
    assert_eq!(row.cell(1)?, "$1.00");
    Ok(())
}

#[tokio::test]
async fn test_table_cell() -> Result<()> {
    let html = r#"
        <table id="grid">
            <tr>
                <td>A1</td>
                <td>A2</td>
            </tr>
            <tr>
                <td>B1</td>
                <td>B2</td>
            </tr>
        </table>
    "#;

    let transport = MockTransport::new(default_response());
    let dom = Dom::new(transport).parse(html.to_string())?;
    let table = dom.table("#grid")?;

    assert_eq!(table.cell(0, 0)?, "A1");
    assert_eq!(table.cell(0, 1)?, "A2");
    assert_eq!(table.cell(1, 0)?, "B1");
    assert_eq!(table.cell(1, 1)?, "B2");
    Ok(())
}

#[tokio::test]
async fn test_table_row_get_by_column_name() -> Result<()> {
    let html = r#"
        <table id="users">
            <thead>
                <tr>
                    <th>Name</th>
                    <th>Email</th>
                </tr>
            </thead>
            <tbody>
                <tr>
                    <td>Bob</td>
                    <td>bob@example.com</td>
                </tr>
            </tbody>
        </table>
    "#;

    let transport = MockTransport::new(default_response());
    let dom = Dom::new(transport).parse(html.to_string())?;
    let table = dom.table("#users")?;

    let row = table.row(0)?;
    assert_eq!(row.get("Name")?, "Bob");
    assert_eq!(row.get("Email")?, "bob@example.com");
    Ok(())
}

#[tokio::test]
async fn test_table_find_row() -> Result<()> {
    let html = r#"
        <table id="employees">
            <thead>
                <tr>
                    <th>ID</th>
                    <th>Name</th>
                    <th>Department</th>
                </tr>
            </thead>
            <tbody>
                <tr>
                    <td>001</td>
                    <td>Alice</td>
                    <td>Engineering</td>
                </tr>
                <tr>
                    <td>002</td>
                    <td>Bob</td>
                    <td>Sales</td>
                </tr>
                <tr>
                    <td>003</td>
                    <td>Charlie</td>
                    <td>Engineering</td>
                </tr>
            </tbody>
        </table>
    "#;

    let transport = MockTransport::new(default_response());
    let dom = Dom::new(transport).parse(html.to_string())?;
    let table = dom.table("#employees")?;

    let row = table.find_row("Name", "Bob")?;
    assert_eq!(row.get("ID")?, "002");
    assert_eq!(row.get("Department")?, "Sales");
    Ok(())
}

#[tokio::test]
async fn test_table_not_found() {
    let html = r#"
        <table id="data">
            <tr><td>Test</td></tr>
        </table>
    "#;

    let transport = MockTransport::new(default_response());
    let dom = Dom::new(transport).parse(html.to_string()).unwrap();
    let result = dom.table("#nonexistent");

    assert!(result.is_err());
    assert!(result.unwrap_err().to_string().contains("Table not found"));
}

#[tokio::test]
async fn test_table_row_out_of_bounds() {
    let html = r#"
        <table id="data">
            <tr><td>A</td></tr>
        </table>
    "#;

    let transport = MockTransport::new(default_response());
    let dom = Dom::new(transport).parse(html.to_string()).unwrap();
    let table = dom.table("#data").unwrap();
    let result = table.row(5);

    assert!(result.is_err());
    assert!(result.unwrap_err().to_string().contains("out of bounds"));
}

#[tokio::test]
async fn test_table_by_class() -> Result<()> {
    let html = r#"
        <table class="data-table">
            <tr>
                <th>Col1</th>
                <th>Col2</th>
            </tr>
            <tr>
                <td>Val1</td>
                <td>Val2</td>
            </tr>
        </table>
    "#;

    let transport = MockTransport::new(default_response());
    let dom = Dom::new(transport).parse(html.to_string())?;
    let table = dom.table(".data-table")?;

    let headers = table.headers();
    assert_eq!(headers.len(), 2);
    assert_eq!(headers[0], "Col1");
    assert_eq!(headers[1], "Col2");
    Ok(())
}

// ============================================
// List API Tests
// ============================================

#[tokio::test]
async fn test_list_items() -> Result<()> {
    let html = r#"
        <ul id="fruits">
            <li>Apple</li>
            <li>Banana</li>
            <li>Orange</li>
        </ul>
    "#;

    let transport = MockTransport::new(default_response());
    let dom = Dom::new(transport).parse(html.to_string())?;
    let list = dom.list("#fruits")?;

    let items = list.items();
    assert_eq!(items.len(), 3);
    assert_eq!(items[0], "Apple");
    assert_eq!(items[1], "Banana");
    assert_eq!(items[2], "Orange");
    Ok(())
}

#[tokio::test]
async fn test_list_item() -> Result<()> {
    let html = r#"
        <ol id="steps">
            <li>First</li>
            <li>Second</li>
            <li>Third</li>
        </ol>
    "#;

    let transport = MockTransport::new(default_response());
    let dom = Dom::new(transport).parse(html.to_string())?;
    let list = dom.list("#steps")?;

    assert_eq!(list.item(0)?, "First");
    assert_eq!(list.item(1)?, "Second");
    assert_eq!(list.item(2)?, "Third");
    Ok(())
}

#[tokio::test]
async fn test_list_len() -> Result<()> {
    let html = r#"
        <ul id="colors">
            <li>Red</li>
            <li>Green</li>
            <li>Blue</li>
            <li>Yellow</li>
        </ul>
    "#;

    let transport = MockTransport::new(default_response());
    let dom = Dom::new(transport).parse(html.to_string())?;
    let list = dom.list("#colors")?;

    assert_eq!(list.len(), 4);
    assert!(!list.is_empty());
    Ok(())
}

#[tokio::test]
async fn test_list_is_empty() -> Result<()> {
    let html = r#"
        <ul id="empty"></ul>
    "#;

    let transport = MockTransport::new(default_response());
    let dom = Dom::new(transport).parse(html.to_string())?;
    let list = dom.list("#empty")?;

    assert_eq!(list.len(), 0);
    assert!(list.is_empty());
    Ok(())
}

#[tokio::test]
async fn test_list_contains() -> Result<()> {
    let html = r#"
        <ul id="tasks">
            <li>Buy milk</li>
            <li>Walk the dog</li>
            <li>Read a book</li>
        </ul>
    "#;

    let transport = MockTransport::new(default_response());
    let dom = Dom::new(transport).parse(html.to_string())?;
    let list = dom.list("#tasks")?;

    assert!(list.contains("Buy milk"));
    assert!(list.contains("Walk the dog"));
    assert!(!list.contains("Write code"));
    Ok(())
}

#[tokio::test]
async fn test_list_not_found() {
    let html = r#"
        <ul id="items">
            <li>Item 1</li>
        </ul>
    "#;

    let transport = MockTransport::new(default_response());
    let dom = Dom::new(transport).parse(html.to_string()).unwrap();
    let result = dom.list("#nonexistent");

    assert!(result.is_err());
    assert!(result.unwrap_err().to_string().contains("List not found"));
}

#[tokio::test]
async fn test_list_item_out_of_bounds() {
    let html = r#"
        <ul id="items">
            <li>Item 1</li>
        </ul>
    "#;

    let transport = MockTransport::new(default_response());
    let dom = Dom::new(transport).parse(html.to_string()).unwrap();
    let list = dom.list("#items").unwrap();
    let result = list.item(5);

    assert!(result.is_err());
    assert!(result.unwrap_err().to_string().contains("out of bounds"));
}

#[tokio::test]
async fn test_list_by_class() -> Result<()> {
    let html = r#"
        <ul class="menu-items">
            <li>Home</li>
            <li>About</li>
            <li>Contact</li>
        </ul>
    "#;

    let transport = MockTransport::new(default_response());
    let dom = Dom::new(transport).parse(html.to_string())?;
    let list = dom.list(".menu-items")?;

    assert_eq!(list.len(), 3);
    assert!(list.contains("Home"));
    assert!(list.contains("About"));
    assert!(list.contains("Contact"));
    Ok(())
}

#[tokio::test]
async fn test_list_by_test_id() -> Result<()> {
    let html = r#"
        <ol test-id="ranking">
            <li>First Place</li>
            <li>Second Place</li>
            <li>Third Place</li>
        </ol>
    "#;

    let transport = MockTransport::new(default_response());
    let dom = Dom::new(transport).parse(html.to_string())?;
    let list = dom.list("@ranking")?;

    assert_eq!(list.len(), 3);
    assert_eq!(list.item(0)?, "First Place");
    Ok(())
}
