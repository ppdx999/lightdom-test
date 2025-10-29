use anyhow::{anyhow, Result};
use async_trait::async_trait;
use scraper::{Html, Selector};
use std::collections::{HashMap, HashSet};
use std::sync::Arc;

pub mod transports;

/// HTTP method enumeration
#[derive(Debug, Clone, PartialEq)]
pub enum Method {
    Get,
    Post,
}

/// HTTP request structure
#[derive(Debug, Clone)]
pub struct HttpRequest {
    pub method: Method,
    pub url: String,
    pub headers: HashMap<String, String>,
    pub body: Option<String>,
}

/// HTTP status code
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct StatusCode(pub u16);

impl StatusCode {
    pub fn is_success(&self) -> bool {
        self.0 >= 200 && self.0 < 300
    }
}

/// HTTP response structure
#[derive(Debug, Clone)]
pub struct HttpResponse {
    pub status: StatusCode,
    pub headers: HashMap<String, String>,
    pub body: String,
}

/// HTTP transport layer trait
#[async_trait]
pub trait HttpTransport: Send + Sync {
    async fn send(&self, req: HttpRequest) -> Result<HttpResponse>;
}

/// Structure for manipulating HTML documents
pub struct Dom<T: HttpTransport> {
    transport: Arc<T>,
    html: String,
}

impl<T: HttpTransport> Dom<T> {
    /// Create a new Dom instance
    pub fn new(transport: T) -> Self {
        Self {
            transport: Arc::new(transport),
            html: String::new(),
        }
    }

    /// Parse HTML and return a Dom instance
    pub fn parse(mut self, html: String) -> Result<Self> {
        self.html = html;
        Ok(self)
    }

    /// Get a form
    pub fn form(&self, locator: &str) -> Result<Form<T>> {
        Form::find(&self.html, locator, Arc::clone(&self.transport))
    }

    /// Get a button
    pub fn button(&self, locator: &str) -> Result<Button<T>> {
        Button::find(&self.html, locator, Arc::clone(&self.transport))
    }

    /// Get a link
    pub fn link(&self, locator: &str) -> Result<Link<T>> {
        Link::find(&self.html, locator, Arc::clone(&self.transport))
    }

    /// Get an element
    pub fn element(&self, locator: &str) -> Result<Element> {
        Element::find(&self.html, locator)
    }

    /// Get multiple elements
    pub fn elements(&self, locator: &str) -> Vec<Element> {
        Element::find_all(&self.html, locator)
    }

    /// Get the text of an element
    pub fn text(&self, locator: &str) -> Result<String> {
        let element = self.element(locator)?;
        Ok(element.text())
    }

    /// Get the text of multiple elements
    pub fn texts(&self, locator: &str) -> Vec<String> {
        self.elements(locator).iter().map(|e| e.text()).collect()
    }

    /// Get the inner HTML of an element
    pub fn inner_html(&self, locator: &str) -> Result<String> {
        let element = self.element(locator)?;
        Ok(element.inner_html())
    }

    /// Get a table
    pub fn table(&self, locator: &str) -> Result<Table> {
        Table::find(&self.html, locator)
    }

    /// Get a list
    pub fn list(&self, locator: &str) -> Result<List> {
        List::find(&self.html, locator)
    }

    /// Check if an element exists
    pub fn exists(&self, locator: &str) -> bool {
        self.element(locator).is_ok()
    }

    /// Check if an element containing the specified text exists
    pub fn contains_text(&self, text: &str) -> bool {
        let document = Html::parse_document(&self.html);
        let body_text: String = document.root_element().text().collect();
        body_text.contains(text)
    }

    /// Get the content of the title tag
    pub fn title(&self) -> Result<String> {
        let document = Html::parse_document(&self.html);
        let title_selector = Selector::parse("title").unwrap();
        let title_element = document
            .select(&title_selector)
            .next()
            .ok_or_else(|| anyhow!("Title tag not found"))?;
        Ok(title_element.text().collect::<String>().trim().to_string())
    }

    /// Get the content attribute of a meta tag
    pub fn meta(&self, name: &str) -> Result<String> {
        let document = Html::parse_document(&self.html);

        // Search by name attribute
        let name_selector = Selector::parse(&format!("meta[name=\"{}\"]", name)).unwrap();
        if let Some(meta_element) = document.select(&name_selector).next() {
            return meta_element
                .value()
                .attr("content")
                .ok_or_else(|| anyhow!("Meta tag '{}' has no content attribute", name))
                .map(|s| s.to_string());
        }

        // Search by property attribute (for OGP tags)
        let property_selector = Selector::parse(&format!("meta[property=\"{}\"]", name)).unwrap();
        if let Some(meta_element) = document.select(&property_selector).next() {
            return meta_element
                .value()
                .attr("content")
                .ok_or_else(|| anyhow!("Meta tag '{}' has no content attribute", name))
                .map(|s| s.to_string());
        }

        Err(anyhow!("Meta tag '{}' not found", name))
    }

    /// Get an image
    pub fn image(&self, locator: &str) -> Result<Image> {
        Image::find(&self.html, locator)
    }

    /// Get multiple images
    pub fn images(&self, locator: &str) -> Vec<Image> {
        Image::find_all(&self.html, locator)
    }

    /// Get a select element
    pub fn select_element(&self, locator: &str) -> Result<SelectElement> {
        SelectElement::find(&self.html, locator)
    }
}

/// HTML form structure
#[derive(Debug)]
pub struct Form<T: HttpTransport> {
    action: String,
    method: String,
    fields: HashMap<String, String>,
    field_types: HashMap<String, String>,
    // Checkbox/radio information
    checkboxes: HashMap<String, Vec<String>>, // name -> [values]
    radios: HashMap<String, Vec<String>>,     // name -> [values]
    checked_checkboxes: HashSet<String>,      // Set of "name=value"
    selected_radios: HashMap<String, String>, // name -> selected value
    transport: Arc<T>,
}

impl<T: HttpTransport> Form<T> {
    fn find(html: &str, locator: &str, transport: Arc<T>) -> Result<Self> {
        let document = Html::parse_document(html);

        // Generate selector based on locator
        let selector_str = if let Some(test_id) = locator.strip_prefix('@') {
            // test-id attribute
            format!("form[test-id=\"{}\"]", test_id)
        } else if locator.starts_with('#') {
            // id attribute
            format!("form{}", locator)
        } else if locator.starts_with('/') {
            // action attribute
            format!("form[action=\"{}\"]", locator)
        } else {
            return Err(anyhow!("Invalid locator: {}", locator));
        };

        let form_selector =
            Selector::parse(&selector_str).map_err(|e| anyhow!("Invalid selector: {:?}", e))?;

        let form_element = document
            .select(&form_selector)
            .next()
            .ok_or_else(|| anyhow!("Form not found: {}", locator))?;

        // Get action attribute
        let action = form_element
            .value()
            .attr("action")
            .unwrap_or("")
            .to_string();

        // Get method attribute
        let method = form_element
            .value()
            .attr("method")
            .unwrap_or("get")
            .to_string();

        // Collect hidden fields in the form in advance
        let input_selector =
            Selector::parse("input").map_err(|e| anyhow!("Invalid selector: {:?}", e))?;

        let mut fields = HashMap::new();
        let mut field_types = HashMap::new();
        let mut checkboxes: HashMap<String, Vec<String>> = HashMap::new();
        let mut radios: HashMap<String, Vec<String>> = HashMap::new();
        let mut checked_checkboxes = HashSet::new();
        let mut selected_radios = HashMap::new();

        for input in form_element.select(&input_selector) {
            if let Some(name) = input.value().attr("name") {
                let input_type = input.value().attr("type").unwrap_or("text");
                field_types.insert(name.to_string(), input_type.to_string());

                match input_type {
                    "hidden" => {
                        // Set hidden field value in advance
                        if let Some(value) = input.value().attr("value") {
                            fields.insert(name.to_string(), value.to_string());
                        }
                    }
                    "checkbox" => {
                        // Collect checkbox information
                        if let Some(value) = input.value().attr("value") {
                            checkboxes
                                .entry(name.to_string())
                                .or_default()
                                .push(value.to_string());

                            // Record as checked if checked attribute is true
                            if input.value().attr("checked").is_some() {
                                checked_checkboxes.insert(format!("{}={}", name, value));
                            }
                        }
                    }
                    "radio" => {
                        // Collect radio information
                        if let Some(value) = input.value().attr("value") {
                            radios
                                .entry(name.to_string())
                                .or_default()
                                .push(value.to_string());

                            // Record as selected if checked attribute is true
                            if input.value().attr("checked").is_some() {
                                selected_radios.insert(name.to_string(), value.to_string());
                            }
                        }
                    }
                    _ => {
                        // text, email, password, number, etc.
                    }
                }
            }
        }

        // Also collect textarea and select elements
        let textarea_selector =
            Selector::parse("textarea").map_err(|e| anyhow!("Invalid selector: {:?}", e))?;
        for textarea in form_element.select(&textarea_selector) {
            if let Some(name) = textarea.value().attr("name") {
                field_types.insert(name.to_string(), "textarea".to_string());
            }
        }

        let select_selector =
            Selector::parse("select").map_err(|e| anyhow!("Invalid selector: {:?}", e))?;
        for select in form_element.select(&select_selector) {
            if let Some(name) = select.value().attr("name") {
                field_types.insert(name.to_string(), "select".to_string());
            }
        }

        Ok(Self {
            action,
            method,
            fields,
            field_types,
            checkboxes,
            radios,
            checked_checkboxes,
            selected_radios,
            transport,
        })
    }

    /// Check if field exists in form
    pub fn is_exist(&self, field_name: &str) -> bool {
        self.field_types.contains_key(field_name)
    }

    /// Get current field value
    pub fn get_value(&self, field_name: &str) -> Result<String> {
        self.fields
            .get(field_name)
            .cloned()
            .ok_or_else(|| anyhow!("Field '{}' not found or has no value", field_name))
    }

    /// Fill field with value
    pub fn fill(&mut self, field_name: &str, value: &str) -> Result<&mut Self> {
        // Check if field exists
        let field_type = self
            .field_types
            .get(field_name)
            .ok_or_else(|| anyhow!("Field '{}' does not exist in the form", field_name))?;

        // Validation based on type
        match field_type.as_str() {
            "email" => {
                if !value.contains('@') {
                    return Err(anyhow!("Invalid email format for field '{}'", field_name));
                }
            }
            "number" => {
                if value.parse::<f64>().is_err() {
                    return Err(anyhow!("Invalid number format for field '{}'", field_name));
                }
            }
            "url" => {
                if !value.starts_with("http://")
                    && !value.starts_with("https://")
                    && !value.is_empty()
                {
                    return Err(anyhow!(
                        "Invalid URL format for field '{}'. Must start with http:// or https://",
                        field_name
                    ));
                }
            }
            "tel" => {
                // Phone numbers allow only digits, hyphens, spaces, parentheses, and +
                if !value.chars().all(|c| {
                    c.is_numeric() || c == '-' || c == ' ' || c == '(' || c == ')' || c == '+'
                }) {
                    return Err(anyhow!(
                        "Invalid phone number format for field '{}'",
                        field_name
                    ));
                }
            }
            "date" => {
                // Check YYYY-MM-DD format
                let parts: Vec<&str> = value.split('-').collect();
                if parts.len() != 3 {
                    return Err(anyhow!(
                        "Invalid date format for field '{}'. Expected YYYY-MM-DD",
                        field_name
                    ));
                }
                if parts[0].len() != 4 || parts[1].len() != 2 || parts[2].len() != 2 {
                    return Err(anyhow!(
                        "Invalid date format for field '{}'. Expected YYYY-MM-DD",
                        field_name
                    ));
                }
                for part in &parts {
                    if part.parse::<u32>().is_err() {
                        return Err(anyhow!(
                            "Invalid date format for field '{}'. Expected YYYY-MM-DD",
                            field_name
                        ));
                    }
                }
            }
            _ => {
                // No validation for text, password, hidden, textarea, select, etc.
            }
        }

        self.fields
            .insert(field_name.to_string(), value.to_string());
        Ok(self)
    }

    /// Check a checkbox
    pub fn check(&mut self, field_name: &str, value: &str) -> Result<&mut Self> {
        // Check if checkbox exists
        let checkbox_values = self
            .checkboxes
            .get(field_name)
            .ok_or_else(|| anyhow!("Checkbox '{}' does not exist in the form", field_name))?;

        // Check if specified value exists
        if !checkbox_values.contains(&value.to_string()) {
            return Err(anyhow!(
                "Checkbox '{}' does not have value '{}'",
                field_name,
                value
            ));
        }

        // Set to checked state
        self.checked_checkboxes
            .insert(format!("{}={}", field_name, value));
        Ok(self)
    }

    /// Uncheck a checkbox
    pub fn uncheck(&mut self, field_name: &str, value: &str) -> Result<&mut Self> {
        // Check if checkbox exists
        let checkbox_values = self
            .checkboxes
            .get(field_name)
            .ok_or_else(|| anyhow!("Checkbox '{}' does not exist in the form", field_name))?;

        // Check if specified value exists
        if !checkbox_values.contains(&value.to_string()) {
            return Err(anyhow!(
                "Checkbox '{}' does not have value '{}'",
                field_name,
                value
            ));
        }

        // Uncheck
        self.checked_checkboxes
            .remove(&format!("{}={}", field_name, value));
        Ok(self)
    }

    /// Select a radio button
    pub fn choose(&mut self, field_name: &str, value: &str) -> Result<&mut Self> {
        // Check if radio button exists
        let radio_values = self
            .radios
            .get(field_name)
            .ok_or_else(|| anyhow!("Radio button '{}' does not exist in the form", field_name))?;

        // Check if specified value exists
        if !radio_values.contains(&value.to_string()) {
            return Err(anyhow!(
                "Radio button '{}' does not have value '{}'",
                field_name,
                value
            ));
        }

        // Select radio button
        self.selected_radios
            .insert(field_name.to_string(), value.to_string());
        Ok(self)
    }

    /// Select an option in select box
    pub fn select(&mut self, field_name: &str, value: &str) -> Result<&mut Self> {
        // Check if select box exists
        let field_type = self
            .field_types
            .get(field_name)
            .ok_or_else(|| anyhow!("Select '{}' does not exist in the form", field_name))?;

        if field_type != "select" {
            return Err(anyhow!("Field '{}' is not a select element", field_name));
        }

        // Set value (option existence check ideally done from actual HTML, but omitted for simplification)
        self.fields
            .insert(field_name.to_string(), value.to_string());
        Ok(self)
    }

    /// Submit form
    pub async fn submit(&self) -> Result<HttpResponse> {
        let mut params = Vec::new();

        // Regular fields
        for (k, v) in &self.fields {
            params.push(format!("{}={}", k, v));
        }

        // Checked checkboxes
        for checked in &self.checked_checkboxes {
            params.push(checked.clone());
        }

        // Selected radio buttons
        for (name, value) in &self.selected_radios {
            params.push(format!("{}={}", name, value));
        }

        let body = params.join("&");

        let mut headers = HashMap::new();
        if self.method.to_lowercase() == "post" {
            headers.insert(
                "Content-Type".to_string(),
                "application/x-www-form-urlencoded".to_string(),
            );
        }

        let req = HttpRequest {
            method: if self.method.to_lowercase() == "get" {
                Method::Get
            } else {
                Method::Post
            },
            url: self.action.clone(),
            headers,
            body: Some(body),
        };

        self.transport.send(req).await
    }
}

/// HTML button structure
#[derive(Debug)]
pub struct Button<T: HttpTransport> {
    form_action: Option<String>,
    form_method: Option<String>,
    html: String,
    transport: Arc<T>,
}

impl<T: HttpTransport> Button<T> {
    fn find(html: &str, locator: &str, transport: Arc<T>) -> Result<Self> {
        let document = Html::parse_document(html);

        // Generate selector based on locator
        let selector_str = if let Some(test_id) = locator.strip_prefix('@') {
            // test-id attribute
            format!("button[test-id=\"{}\"]", test_id)
        } else if locator.starts_with('#') {
            // id attribute
            format!("button{}", locator)
        } else {
            // Text search (contains) processed later
            "button".to_string()
        };

        let button_selector =
            Selector::parse(&selector_str).map_err(|e| anyhow!("Invalid selector: {:?}", e))?;

        let button_element = if locator.starts_with('@') || locator.starts_with('#') {
            // Attribute-based search
            document
                .select(&button_selector)
                .next()
                .ok_or_else(|| anyhow!("Button not found: {}", locator))?
        } else {
            // Text-based search
            document
                .select(&button_selector)
                .find(|el| {
                    let text = el.text().collect::<String>();
                    text.trim() == locator
                })
                .ok_or_else(|| anyhow!("Button not found: {}", locator))?
        };

        // Find the form that contains the button
        let mut form_action = None;
        let mut form_method = None;

        // Traverse parent elements to find form
        for ancestor in button_element.ancestors() {
            if let Some(element) = ancestor.value().as_element() {
                if element.name() == "form" {
                    form_action = ancestor
                        .value()
                        .as_element()
                        .and_then(|e| e.attr("action"))
                        .map(|s| s.to_string());
                    form_method = ancestor
                        .value()
                        .as_element()
                        .and_then(|e| e.attr("method"))
                        .map(|s| s.to_string());
                    break;
                }
            }
        }

        Ok(Self {
            form_action,
            form_method,
            html: html.to_string(),
            transport,
        })
    }

    /// Click button
    pub async fn click(&self) -> Result<HttpResponse> {
        let action = self
            .form_action
            .as_ref()
            .ok_or_else(|| anyhow!("Button is not associated with a form"))?;

        // Collect default form values (hidden, text, email, etc.)
        let document = Html::parse_document(&self.html);
        let form_selector = Selector::parse(&format!("form[action=\"{}\"]", action))
            .or_else(|_| Selector::parse("form"))
            .map_err(|e| anyhow!("Invalid selector: {:?}", e))?;

        let mut params = Vec::new();

        if let Some(form_element) = document.select(&form_selector).next() {
            let input_selector =
                Selector::parse("input").map_err(|e| anyhow!("Invalid selector: {:?}", e))?;

            for input in form_element.select(&input_selector) {
                let input_type = input.value().attr("type").unwrap_or("text");
                let name = input.value().attr("name");
                let value = input.value().attr("value");

                if let (Some(n), Some(v)) = (name, value) {
                    // Collect default values for hidden, text, email, etc. fields
                    if !matches!(
                        input_type,
                        "checkbox" | "radio" | "submit" | "button" | "reset"
                    ) {
                        params.push(format!("{}={}", n, v));
                    }
                }
            }
        }

        let body = params.join("&");
        let method_str = self.form_method.as_deref().unwrap_or("get").to_lowercase();

        let mut headers = HashMap::new();
        if method_str == "post" {
            headers.insert(
                "Content-Type".to_string(),
                "application/x-www-form-urlencoded".to_string(),
            );
        }

        let req = HttpRequest {
            method: if method_str == "post" {
                Method::Post
            } else {
                Method::Get
            },
            url: action.clone(),
            headers,
            body: if method_str == "post" {
                Some(body)
            } else {
                None
            },
        };

        self.transport.send(req).await
    }
}

/// HTML link structure
#[derive(Debug)]
pub struct Link<T: HttpTransport> {
    href: String,
    transport: Arc<T>,
}

impl<T: HttpTransport> Link<T> {
    fn find(html: &str, locator: &str, transport: Arc<T>) -> Result<Self> {
        let document = Html::parse_document(html);

        // Generate selector based on locator
        let selector_str = if let Some(test_id) = locator.strip_prefix('@') {
            // test-id attribute
            format!("a[test-id=\"{}\"]", test_id)
        } else if locator.starts_with('#') {
            // id attribute
            format!("a{}", locator)
        } else {
            // Search by text
            "a".to_string()
        };

        let link_selector =
            Selector::parse(&selector_str).map_err(|e| anyhow!("Invalid selector: {:?}", e))?;

        let link_element = if locator.starts_with('@') || locator.starts_with('#') {
            // Attribute-based search
            document
                .select(&link_selector)
                .next()
                .ok_or_else(|| anyhow!("Link not found: {}", locator))?
        } else {
            // Text-based search
            document
                .select(&link_selector)
                .find(|el| {
                    let text = el.text().collect::<String>();
                    text.trim() == locator
                })
                .ok_or_else(|| anyhow!("Link not found: {}", locator))?
        };

        // Get href attribute
        let href = link_element
            .value()
            .attr("href")
            .ok_or_else(|| anyhow!("Link has no href attribute"))?
            .to_string();

        Ok(Self { href, transport })
    }

    /// Click link
    pub async fn click(&self) -> Result<HttpResponse> {
        let req = HttpRequest {
            method: Method::Get,
            url: self.href.clone(),
            headers: HashMap::new(),
            body: None,
        };

        self.transport.send(req).await
    }
}

/// HTML element structure
#[derive(Debug, Clone)]
pub struct Element {
    text_content: String,
    inner_html: String,
    attributes: HashMap<String, String>,
}

impl Element {
    fn find(html: &str, locator: &str) -> Result<Self> {
        let document = Html::parse_document(html);
        let selector_str = Self::locator_to_selector(locator)?;
        let selector =
            Selector::parse(&selector_str).map_err(|e| anyhow!("Invalid selector: {:?}", e))?;

        let element = document
            .select(&selector)
            .next()
            .ok_or_else(|| anyhow!("Element not found: {}", locator))?;

        Ok(Self::from_element_ref(element))
    }

    fn find_all(html: &str, locator: &str) -> Vec<Self> {
        let document = Html::parse_document(html);
        let selector_str = match Self::locator_to_selector(locator) {
            Ok(s) => s,
            Err(_) => return Vec::new(),
        };
        let selector = match Selector::parse(&selector_str) {
            Ok(s) => s,
            Err(_) => return Vec::new(),
        };

        document
            .select(&selector)
            .map(Self::from_element_ref)
            .collect()
    }

    fn locator_to_selector(locator: &str) -> Result<String> {
        if let Some(test_id) = locator.strip_prefix('@') {
            Ok(format!("[test-id=\"{}\"]", test_id))
        } else if locator.starts_with('#') || locator.starts_with('.') {
            Ok(locator.to_string())
        } else {
            Err(anyhow!(
                "Invalid locator: {}. Must start with @, #, or .",
                locator
            ))
        }
    }

    fn from_element_ref(element: scraper::element_ref::ElementRef) -> Self {
        let text_content = element.text().collect::<String>();
        let inner_html = element.inner_html();
        let mut attributes = HashMap::new();

        for (name, value) in element.value().attrs() {
            attributes.insert(name.to_string(), value.to_string());
        }

        Self {
            text_content,
            inner_html,
            attributes,
        }
    }

    /// Get element text content
    pub fn text(&self) -> String {
        self.text_content.clone()
    }

    /// Get value of specified attribute
    pub fn attr(&self, name: &str) -> Option<String> {
        self.attributes.get(name).cloned()
    }

    /// Check if element has specified class
    pub fn has_class(&self, class: &str) -> bool {
        if let Some(classes) = self.attributes.get("class") {
            classes.split_whitespace().any(|c| c == class)
        } else {
            false
        }
    }

    /// Get element inner HTML
    pub fn inner_html(&self) -> String {
        self.inner_html.clone()
    }

    /// Check if text contains specified string
    pub fn text_contains(&self, text: &str) -> bool {
        self.text_content.contains(text)
    }

    /// Check if element has disabled attribute
    pub fn is_disabled(&self) -> bool {
        self.attributes.contains_key("disabled")
    }

    /// Check if element has required attribute
    pub fn is_required(&self) -> bool {
        self.attributes.contains_key("required")
    }

    /// Check if element has readonly attribute
    pub fn is_readonly(&self) -> bool {
        self.attributes.contains_key("readonly")
    }

    /// Check if element has checked attribute
    pub fn is_checked(&self) -> bool {
        self.attributes.contains_key("checked")
    }
}

/// Table row structure
#[derive(Debug, Clone)]
pub struct Row {
    cells: Vec<String>,
    headers: Vec<String>,
}

impl Row {
    /// Get text of all cells in row
    pub fn cells(&self) -> Vec<String> {
        self.cells.clone()
    }

    /// Get text of cell at specified index
    pub fn cell(&self, index: usize) -> Result<String> {
        self.cells
            .get(index)
            .cloned()
            .ok_or_else(|| anyhow!("Cell index {} out of bounds", index))
    }

    /// Get cell text by header name
    pub fn get(&self, column: &str) -> Result<String> {
        let index = self
            .headers
            .iter()
            .position(|h| h == column)
            .ok_or_else(|| anyhow!("Column '{}' not found", column))?;
        self.cell(index)
    }
}

/// HTML table structure
#[derive(Debug, Clone)]
pub struct Table {
    headers: Vec<String>,
    rows: Vec<Row>,
}

impl Table {
    fn find(html: &str, locator: &str) -> Result<Self> {
        let document = Html::parse_document(html);
        let selector_str = Element::locator_to_selector(locator)?;
        let table_selector =
            Selector::parse(&selector_str).map_err(|e| anyhow!("Invalid selector: {:?}", e))?;

        let table_element = document
            .select(&table_selector)
            .next()
            .ok_or_else(|| anyhow!("Table not found: {}", locator))?;

        // Get headers
        let th_selector = Selector::parse("thead th, tr th").unwrap();
        let headers: Vec<String> = table_element
            .select(&th_selector)
            .map(|th| th.text().collect::<String>().trim().to_string())
            .collect();

        // Get rows
        let tr_selector = Selector::parse("tbody tr, tr").unwrap();
        let td_selector = Selector::parse("td").unwrap();

        let rows: Vec<Row> = table_element
            .select(&tr_selector)
            .filter_map(|tr| {
                let cells: Vec<String> = tr
                    .select(&td_selector)
                    .map(|td| td.text().collect::<String>().trim().to_string())
                    .collect();

                if cells.is_empty() {
                    None
                } else {
                    Some(Row {
                        cells,
                        headers: headers.clone(),
                    })
                }
            })
            .collect();

        Ok(Self { headers, rows })
    }

    /// Get table headers
    pub fn headers(&self) -> Vec<String> {
        self.headers.clone()
    }

    /// Get all table rows
    pub fn rows(&self) -> Vec<Row> {
        self.rows.clone()
    }

    /// Get row at specified index
    pub fn row(&self, index: usize) -> Result<Row> {
        self.rows
            .get(index)
            .cloned()
            .ok_or_else(|| anyhow!("Row index {} out of bounds", index))
    }

    /// Get text of cell at specified row and column
    pub fn cell(&self, row: usize, col: usize) -> Result<String> {
        let row_data = self.row(row)?;
        row_data.cell(col)
    }

    /// Search for row where specified column value matches
    pub fn find_row(&self, column: &str, value: &str) -> Result<Row> {
        self.rows
            .iter()
            .find(|row| row.get(column).map(|v| v == value).unwrap_or(false))
            .cloned()
            .ok_or_else(|| anyhow!("Row with {}='{}' not found", column, value))
    }
}

/// HTML list structure
#[derive(Debug, Clone)]
pub struct List {
    items: Vec<String>,
}

impl List {
    fn find(html: &str, locator: &str) -> Result<Self> {
        let document = Html::parse_document(html);
        let selector_str = Element::locator_to_selector(locator)?;
        let list_selector =
            Selector::parse(&selector_str).map_err(|e| anyhow!("Invalid selector: {:?}", e))?;

        let list_element = document
            .select(&list_selector)
            .next()
            .ok_or_else(|| anyhow!("List not found: {}", locator))?;

        // Get li elements
        let li_selector = Selector::parse("li").unwrap();
        let items: Vec<String> = list_element
            .select(&li_selector)
            .map(|li| li.text().collect::<String>().trim().to_string())
            .collect();

        Ok(Self { items })
    }

    /// Get text of all list items
    pub fn items(&self) -> Vec<String> {
        self.items.clone()
    }

    /// Get text of item at specified index
    pub fn item(&self, index: usize) -> Result<String> {
        self.items
            .get(index)
            .cloned()
            .ok_or_else(|| anyhow!("Item index {} out of bounds", index))
    }

    /// Return number of list items
    pub fn len(&self) -> usize {
        self.items.len()
    }

    /// Return whether list is empty
    pub fn is_empty(&self) -> bool {
        self.items.is_empty()
    }

    /// Check if item containing specified text exists
    pub fn contains(&self, text: &str) -> bool {
        self.items.iter().any(|item| item == text)
    }
}

/// HTML image structure
#[derive(Debug, Clone)]
pub struct Image {
    src: String,
    alt: Option<String>,
    width: Option<String>,
    height: Option<String>,
}

impl Image {
    fn find(html: &str, locator: &str) -> Result<Self> {
        let document = Html::parse_document(html);
        let selector_str = if let Some(test_id) = locator.strip_prefix('@') {
            format!("img[test-id=\"{}\"]", test_id)
        } else if locator.starts_with('#') || locator.starts_with('.') {
            format!("img{}", locator)
        } else if locator == "img" {
            "img".to_string()
        } else {
            return Err(anyhow!(
                "Invalid locator: {}. Must start with @, #, . or be 'img'",
                locator
            ));
        };

        let selector =
            Selector::parse(&selector_str).map_err(|e| anyhow!("Invalid selector: {:?}", e))?;

        let img_element = document
            .select(&selector)
            .next()
            .ok_or_else(|| anyhow!("Image not found: {}", locator))?;

        Ok(Self::from_element_ref(img_element))
    }

    fn find_all(html: &str, locator: &str) -> Vec<Self> {
        let document = Html::parse_document(html);
        let selector_str = if let Some(test_id) = locator.strip_prefix('@') {
            format!("img[test-id=\"{}\"]", test_id)
        } else if locator.starts_with('#') || locator.starts_with('.') {
            format!("img{}", locator)
        } else if locator == "img" {
            "img".to_string()
        } else {
            return Vec::new();
        };

        let selector = match Selector::parse(&selector_str) {
            Ok(s) => s,
            Err(_) => return Vec::new(),
        };

        document
            .select(&selector)
            .map(Self::from_element_ref)
            .collect()
    }

    fn from_element_ref(element: scraper::element_ref::ElementRef) -> Self {
        let src = element.value().attr("src").unwrap_or("").to_string();
        let alt = element.value().attr("alt").map(|s| s.to_string());
        let width = element.value().attr("width").map(|s| s.to_string());
        let height = element.value().attr("height").map(|s| s.to_string());

        Self {
            src,
            alt,
            width,
            height,
        }
    }

    /// Get image src attribute
    pub fn src(&self) -> String {
        self.src.clone()
    }

    /// Get image alt attribute
    pub fn alt(&self) -> Option<String> {
        self.alt.clone()
    }

    /// Get image width attribute
    pub fn width(&self) -> Option<String> {
        self.width.clone()
    }

    /// Get image height attribute
    pub fn height(&self) -> Option<String> {
        self.height.clone()
    }
}

/// Select option structure
#[derive(Debug, Clone)]
pub struct SelectOption {
    value: String,
    text: String,
    selected: bool,
}

impl SelectOption {
    /// Get option value attribute
    pub fn value(&self) -> String {
        self.value.clone()
    }

    /// Get option display text
    pub fn text(&self) -> String {
        self.text.clone()
    }

    /// Check if option is selected
    pub fn is_selected(&self) -> bool {
        self.selected
    }
}

/// HTML select element structure
#[derive(Debug, Clone)]
pub struct SelectElement {
    options: Vec<SelectOption>,
}

impl SelectElement {
    fn find(html: &str, locator: &str) -> Result<Self> {
        let document = Html::parse_document(html);
        let selector_str = if let Some(test_id) = locator.strip_prefix('@') {
            format!("select[test-id=\"{}\"]", test_id)
        } else if locator.starts_with('#') || locator.starts_with('.') {
            format!("select{}", locator)
        } else {
            return Err(anyhow!(
                "Invalid locator: {}. Must start with @, #, or .",
                locator
            ));
        };

        let selector =
            Selector::parse(&selector_str).map_err(|e| anyhow!("Invalid selector: {:?}", e))?;

        let select_element = document
            .select(&selector)
            .next()
            .ok_or_else(|| anyhow!("Select element not found: {}", locator))?;

        // Get option elements
        let option_selector = Selector::parse("option").unwrap();
        let options: Vec<SelectOption> = select_element
            .select(&option_selector)
            .map(|option| {
                let text_content = option.text().collect::<String>();
                let value = option
                    .value()
                    .attr("value")
                    .map(|s| s.to_string())
                    .unwrap_or_else(|| text_content.trim().to_string());
                let text = text_content.trim().to_string();
                let selected = option.value().attr("selected").is_some();

                SelectOption {
                    value,
                    text,
                    selected,
                }
            })
            .collect();

        Ok(Self { options })
    }

    /// Get all options
    pub fn options(&self) -> Vec<SelectOption> {
        self.options.clone()
    }

    /// Get selected option
    pub fn selected_option(&self) -> Result<SelectOption> {
        self.options
            .iter()
            .find(|opt| opt.selected)
            .cloned()
            .ok_or_else(|| anyhow!("No option is selected"))
    }
}
