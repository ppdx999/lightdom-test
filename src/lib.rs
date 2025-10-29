use anyhow::{Result, anyhow};
use async_trait::async_trait;
use scraper::{Html, Selector};
use std::collections::{HashMap, HashSet};
use std::sync::Arc;

/// HTTP メソッドを表す列挙型
#[derive(Debug, Clone, PartialEq)]
pub enum Method {
    Get,
    Post,
}

/// HTTP リクエストを表す構造体
#[derive(Debug, Clone)]
pub struct HttpRequest {
    pub method: Method,
    pub url: String,
    pub headers: HashMap<String, String>,
    pub body: Option<String>,
}

/// HTTP ステータスコード
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct StatusCode(pub u16);

impl StatusCode {
    pub fn is_success(&self) -> bool {
        self.0 >= 200 && self.0 < 300
    }
}

/// HTTP レスポンスを表す構造体
#[derive(Debug, Clone)]
pub struct HttpResponse {
    pub status: StatusCode,
    pub headers: HashMap<String, String>,
    pub body: String,
}

/// HTTP トランスポート層のトレイト
#[async_trait]
pub trait HttpTransport: Send + Sync {
    async fn send(&self, req: HttpRequest) -> Result<HttpResponse>;
}

/// HTML ドキュメントを操作するための構造体
pub struct Dom<T: HttpTransport> {
    transport: Arc<T>,
    html: String,
}

impl<T: HttpTransport> Dom<T> {
    /// 新しい Dom インスタンスを作成
    pub fn new(transport: T) -> Self {
        Self {
            transport: Arc::new(transport),
            html: String::new(),
        }
    }

    /// HTML をパースして Dom インスタンスを返す
    pub fn parse(mut self, html: String) -> Result<Self> {
        self.html = html;
        Ok(self)
    }

    /// フォームを取得
    pub fn form(&self, locator: &str) -> Result<Form<T>> {
        Form::find(&self.html, locator, Arc::clone(&self.transport))
    }

    /// ボタンを取得
    pub fn button(&self, locator: &str) -> Result<Button<T>> {
        Button::find(&self.html, locator, Arc::clone(&self.transport))
    }

    /// リンクを取得
    pub fn link(&self, locator: &str) -> Result<Link<T>> {
        Link::find(&self.html, locator, Arc::clone(&self.transport))
    }

    /// 要素を取得
    pub fn element(&self, locator: &str) -> Result<Element> {
        Element::find(&self.html, locator)
    }

    /// 複数の要素を取得
    pub fn elements(&self, locator: &str) -> Vec<Element> {
        Element::find_all(&self.html, locator)
    }

    /// 要素のテキストを取得
    pub fn text(&self, locator: &str) -> Result<String> {
        let element = self.element(locator)?;
        Ok(element.text())
    }

    /// 複数要素のテキストを取得
    pub fn texts(&self, locator: &str) -> Vec<String> {
        self.elements(locator).iter().map(|e| e.text()).collect()
    }

    /// 要素の内部HTMLを取得
    pub fn inner_html(&self, locator: &str) -> Result<String> {
        let element = self.element(locator)?;
        Ok(element.inner_html())
    }

    /// テーブルを取得
    pub fn table(&self, locator: &str) -> Result<Table> {
        Table::find(&self.html, locator)
    }

    /// リストを取得
    pub fn list(&self, locator: &str) -> Result<List> {
        List::find(&self.html, locator)
    }

    /// 要素が存在するかチェック
    pub fn exists(&self, locator: &str) -> bool {
        self.element(locator).is_ok()
    }

    /// 指定されたテキストを含む要素が存在するかチェック
    pub fn contains_text(&self, text: &str) -> bool {
        let document = Html::parse_document(&self.html);
        let body_text: String = document.root_element().text().collect();
        body_text.contains(text)
    }

    /// title タグの内容を取得
    pub fn title(&self) -> Result<String> {
        let document = Html::parse_document(&self.html);
        let title_selector = Selector::parse("title").unwrap();
        let title_element = document
            .select(&title_selector)
            .next()
            .ok_or_else(|| anyhow!("Title tag not found"))?;
        Ok(title_element.text().collect::<String>().trim().to_string())
    }

    /// メタタグの content 属性を取得
    pub fn meta(&self, name: &str) -> Result<String> {
        let document = Html::parse_document(&self.html);

        // name 属性で検索
        let name_selector = Selector::parse(&format!("meta[name=\"{}\"]", name)).unwrap();
        if let Some(meta_element) = document.select(&name_selector).next() {
            return meta_element
                .value()
                .attr("content")
                .ok_or_else(|| anyhow!("Meta tag '{}' has no content attribute", name))
                .map(|s| s.to_string());
        }

        // property 属性で検索（OGP タグ用）
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

    /// 画像を取得
    pub fn image(&self, locator: &str) -> Result<Image> {
        Image::find(&self.html, locator)
    }

    /// 複数の画像を取得
    pub fn images(&self, locator: &str) -> Vec<Image> {
        Image::find_all(&self.html, locator)
    }

    /// select 要素を取得
    pub fn select_element(&self, locator: &str) -> Result<SelectElement> {
        SelectElement::find(&self.html, locator)
    }
}

/// HTML フォームを表す構造体
#[derive(Debug)]
pub struct Form<T: HttpTransport> {
    action: String,
    method: String,
    fields: HashMap<String, String>,
    field_types: HashMap<String, String>,
    // checkbox/radio の情報
    checkboxes: HashMap<String, Vec<String>>, // name -> [values]
    radios: HashMap<String, Vec<String>>,     // name -> [values]
    checked_checkboxes: HashSet<String>,      // "name=value" のセット
    selected_radios: HashMap<String, String>, // name -> selected value
    transport: Arc<T>,
}

impl<T: HttpTransport> Form<T> {
    fn find(html: &str, locator: &str, transport: Arc<T>) -> Result<Self> {
        let document = Html::parse_document(html);

        // ロケータに基づいてセレクタを生成
        let selector_str = if let Some(test_id) = locator.strip_prefix('@') {
            // test-id 属性
            format!("form[test-id=\"{}\"]", test_id)
        } else if locator.starts_with('#') {
            // id 属性
            format!("form{}", locator)
        } else if locator.starts_with('/') {
            // action 属性
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

        // action 属性を取得
        let action = form_element
            .value()
            .attr("action")
            .unwrap_or("")
            .to_string();

        // method 属性を取得
        let method = form_element
            .value()
            .attr("method")
            .unwrap_or("get")
            .to_string();

        // フォーム内の hidden フィールドを事前に収集
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
                        // hidden フィールドの値は事前に設定
                        if let Some(value) = input.value().attr("value") {
                            fields.insert(name.to_string(), value.to_string());
                        }
                    }
                    "checkbox" => {
                        // checkbox の情報を収集
                        if let Some(value) = input.value().attr("value") {
                            checkboxes
                                .entry(name.to_string())
                                .or_default()
                                .push(value.to_string());

                            // checkedがtrueならチェック済みとして記録
                            if input.value().attr("checked").is_some() {
                                checked_checkboxes.insert(format!("{}={}", name, value));
                            }
                        }
                    }
                    "radio" => {
                        // radio の情報を収集
                        if let Some(value) = input.value().attr("value") {
                            radios
                                .entry(name.to_string())
                                .or_default()
                                .push(value.to_string());

                            // checkedがtrueなら選択済みとして記録
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

        // textareaとselectも収集
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

    /// フィールドがフォーム内に存在するかチェック
    pub fn is_exist(&self, field_name: &str) -> bool {
        self.field_types.contains_key(field_name)
    }

    /// フィールドの現在値を取得
    pub fn get_value(&self, field_name: &str) -> Result<String> {
        self.fields
            .get(field_name)
            .cloned()
            .ok_or_else(|| anyhow!("Field '{}' not found or has no value", field_name))
    }

    /// フィールドに値を入力
    pub fn fill(&mut self, field_name: &str, value: &str) -> Result<&mut Self> {
        // フィールドが存在するかチェック
        let field_type = self
            .field_types
            .get(field_name)
            .ok_or_else(|| anyhow!("Field '{}' does not exist in the form", field_name))?;

        // type に応じたバリデーション
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
                // 電話番号は数字、ハイフン、スペース、括弧、+のみ許可
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
                // YYYY-MM-DD形式をチェック
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
                // text, password, hidden, textarea, select などはバリデーション不要
            }
        }

        self.fields
            .insert(field_name.to_string(), value.to_string());
        Ok(self)
    }

    /// チェックボックスをチェックする
    pub fn check(&mut self, field_name: &str, value: &str) -> Result<&mut Self> {
        // チェックボックスが存在するかチェック
        let checkbox_values = self
            .checkboxes
            .get(field_name)
            .ok_or_else(|| anyhow!("Checkbox '{}' does not exist in the form", field_name))?;

        // 指定された値が存在するかチェック
        if !checkbox_values.contains(&value.to_string()) {
            return Err(anyhow!(
                "Checkbox '{}' does not have value '{}'",
                field_name,
                value
            ));
        }

        // チェック状態に設定
        self.checked_checkboxes
            .insert(format!("{}={}", field_name, value));
        Ok(self)
    }

    /// チェックボックスのチェックを外す
    pub fn uncheck(&mut self, field_name: &str, value: &str) -> Result<&mut Self> {
        // チェックボックスが存在するかチェック
        let checkbox_values = self
            .checkboxes
            .get(field_name)
            .ok_or_else(|| anyhow!("Checkbox '{}' does not exist in the form", field_name))?;

        // 指定された値が存在するかチェック
        if !checkbox_values.contains(&value.to_string()) {
            return Err(anyhow!(
                "Checkbox '{}' does not have value '{}'",
                field_name,
                value
            ));
        }

        // チェックを外す
        self.checked_checkboxes
            .remove(&format!("{}={}", field_name, value));
        Ok(self)
    }

    /// ラジオボタンを選択する
    pub fn choose(&mut self, field_name: &str, value: &str) -> Result<&mut Self> {
        // ラジオボタンが存在するかチェック
        let radio_values = self
            .radios
            .get(field_name)
            .ok_or_else(|| anyhow!("Radio button '{}' does not exist in the form", field_name))?;

        // 指定された値が存在するかチェック
        if !radio_values.contains(&value.to_string()) {
            return Err(anyhow!(
                "Radio button '{}' does not have value '{}'",
                field_name,
                value
            ));
        }

        // ラジオボタンを選択
        self.selected_radios
            .insert(field_name.to_string(), value.to_string());
        Ok(self)
    }

    /// セレクトボックスのオプションを選択する
    pub fn select(&mut self, field_name: &str, value: &str) -> Result<&mut Self> {
        // セレクトボックスが存在するかチェック
        let field_type = self
            .field_types
            .get(field_name)
            .ok_or_else(|| anyhow!("Select '{}' does not exist in the form", field_name))?;

        if field_type != "select" {
            return Err(anyhow!("Field '{}' is not a select element", field_name));
        }

        // 値を設定（オプションの存在チェックは実際のHTMLから行うのが理想だが、簡略化のため省略）
        self.fields
            .insert(field_name.to_string(), value.to_string());
        Ok(self)
    }

    /// フォームを送信
    pub async fn submit(&self) -> Result<HttpResponse> {
        let mut params = Vec::new();

        // 通常のフィールド
        for (k, v) in &self.fields {
            params.push(format!("{}={}", k, v));
        }

        // チェックされたチェックボックス
        for checked in &self.checked_checkboxes {
            params.push(checked.clone());
        }

        // 選択されたラジオボタン
        for (name, value) in &self.selected_radios {
            params.push(format!("{}={}", name, value));
        }

        let body = params.join("&");

        let req = HttpRequest {
            method: if self.method.to_lowercase() == "get" {
                Method::Get
            } else {
                Method::Post
            },
            url: self.action.clone(),
            headers: HashMap::new(),
            body: Some(body),
        };

        self.transport.send(req).await
    }
}

/// HTML ボタンを表す構造体
#[derive(Debug)]
pub struct Button<T: HttpTransport> {
    form_action: Option<String>,
    form_method: Option<String>,
    transport: Arc<T>,
}

impl<T: HttpTransport> Button<T> {
    fn find(html: &str, locator: &str, transport: Arc<T>) -> Result<Self> {
        let document = Html::parse_document(html);

        // ロケータに基づいてセレクタを生成
        let selector_str = if let Some(test_id) = locator.strip_prefix('@') {
            // test-id 属性
            format!("button[test-id=\"{}\"]", test_id)
        } else if locator.starts_with('#') {
            // id 属性
            format!("button{}", locator)
        } else {
            // テキストで検索（contains）は後で処理
            "button".to_string()
        };

        let button_selector =
            Selector::parse(&selector_str).map_err(|e| anyhow!("Invalid selector: {:?}", e))?;

        let button_element = if locator.starts_with('@') || locator.starts_with('#') {
            // 属性ベースの検索
            document
                .select(&button_selector)
                .next()
                .ok_or_else(|| anyhow!("Button not found: {}", locator))?
        } else {
            // テキストベースの検索
            document
                .select(&button_selector)
                .find(|el| {
                    let text = el.text().collect::<String>();
                    text.trim() == locator
                })
                .ok_or_else(|| anyhow!("Button not found: {}", locator))?
        };

        // ボタンが属するフォームを探す
        let mut form_action = None;
        let mut form_method = None;

        // 親要素を辿ってformを探す
        for ancestor in button_element.ancestors() {
            if let Some(element) = ancestor.value().as_element()
                && element.name() == "form"
            {
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

        Ok(Self {
            form_action,
            form_method,
            transport,
        })
    }

    /// ボタンをクリック
    pub async fn click(&self) -> Result<HttpResponse> {
        let action = self
            .form_action
            .as_ref()
            .ok_or_else(|| anyhow!("Button is not associated with a form"))?;

        let req = HttpRequest {
            method: if self.form_method.as_deref().unwrap_or("get").to_lowercase() == "post" {
                Method::Post
            } else {
                Method::Get
            },
            url: action.clone(),
            headers: HashMap::new(),
            body: None,
        };

        self.transport.send(req).await
    }
}

/// HTML リンクを表す構造体
#[derive(Debug)]
pub struct Link<T: HttpTransport> {
    href: String,
    transport: Arc<T>,
}

impl<T: HttpTransport> Link<T> {
    fn find(html: &str, locator: &str, transport: Arc<T>) -> Result<Self> {
        let document = Html::parse_document(html);

        // ロケータに基づいてセレクタを生成
        let selector_str = if let Some(test_id) = locator.strip_prefix('@') {
            // test-id 属性
            format!("a[test-id=\"{}\"]", test_id)
        } else if locator.starts_with('#') {
            // id 属性
            format!("a{}", locator)
        } else {
            // テキストで検索
            "a".to_string()
        };

        let link_selector =
            Selector::parse(&selector_str).map_err(|e| anyhow!("Invalid selector: {:?}", e))?;

        let link_element = if locator.starts_with('@') || locator.starts_with('#') {
            // 属性ベースの検索
            document
                .select(&link_selector)
                .next()
                .ok_or_else(|| anyhow!("Link not found: {}", locator))?
        } else {
            // テキストベースの検索
            document
                .select(&link_selector)
                .find(|el| {
                    let text = el.text().collect::<String>();
                    text.trim() == locator
                })
                .ok_or_else(|| anyhow!("Link not found: {}", locator))?
        };

        // href 属性を取得
        let href = link_element
            .value()
            .attr("href")
            .ok_or_else(|| anyhow!("Link has no href attribute"))?
            .to_string();

        Ok(Self { href, transport })
    }

    /// リンクをクリック
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

/// HTML要素を表す構造体
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

    /// 要素のテキストコンテンツを取得
    pub fn text(&self) -> String {
        self.text_content.clone()
    }

    /// 指定された属性の値を取得
    pub fn attr(&self, name: &str) -> Option<String> {
        self.attributes.get(name).cloned()
    }

    /// 指定されたクラスを持っているかチェック
    pub fn has_class(&self, class: &str) -> bool {
        if let Some(classes) = self.attributes.get("class") {
            classes.split_whitespace().any(|c| c == class)
        } else {
            false
        }
    }

    /// 要素の内部HTMLを取得
    pub fn inner_html(&self) -> String {
        self.inner_html.clone()
    }

    /// テキストが指定された文字列を含むかチェック
    pub fn text_contains(&self, text: &str) -> bool {
        self.text_content.contains(text)
    }

    /// disabled 属性を持っているかチェック
    pub fn is_disabled(&self) -> bool {
        self.attributes.contains_key("disabled")
    }

    /// required 属性を持っているかチェック
    pub fn is_required(&self) -> bool {
        self.attributes.contains_key("required")
    }

    /// readonly 属性を持っているかチェック
    pub fn is_readonly(&self) -> bool {
        self.attributes.contains_key("readonly")
    }

    /// checked 属性を持っているかチェック
    pub fn is_checked(&self) -> bool {
        self.attributes.contains_key("checked")
    }
}

/// テーブルの行を表す構造体
#[derive(Debug, Clone)]
pub struct Row {
    cells: Vec<String>,
    headers: Vec<String>,
}

impl Row {
    /// 行内の全セルのテキストを取得
    pub fn cells(&self) -> Vec<String> {
        self.cells.clone()
    }

    /// 指定されたインデックスのセルのテキストを取得
    pub fn cell(&self, index: usize) -> Result<String> {
        self.cells
            .get(index)
            .cloned()
            .ok_or_else(|| anyhow!("Cell index {} out of bounds", index))
    }

    /// ヘッダー名を指定してセルのテキストを取得
    pub fn get(&self, column: &str) -> Result<String> {
        let index = self
            .headers
            .iter()
            .position(|h| h == column)
            .ok_or_else(|| anyhow!("Column '{}' not found", column))?;
        self.cell(index)
    }
}

/// HTMLテーブルを表す構造体
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

        // ヘッダーの取得
        let th_selector = Selector::parse("thead th, tr th").unwrap();
        let headers: Vec<String> = table_element
            .select(&th_selector)
            .map(|th| th.text().collect::<String>().trim().to_string())
            .collect();

        // 行の取得
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

    /// テーブルのヘッダーを取得
    pub fn headers(&self) -> Vec<String> {
        self.headers.clone()
    }

    /// テーブルの全行を取得
    pub fn rows(&self) -> Vec<Row> {
        self.rows.clone()
    }

    /// 指定されたインデックスの行を取得
    pub fn row(&self, index: usize) -> Result<Row> {
        self.rows
            .get(index)
            .cloned()
            .ok_or_else(|| anyhow!("Row index {} out of bounds", index))
    }

    /// 指定された行・列のセルのテキストを取得
    pub fn cell(&self, row: usize, col: usize) -> Result<String> {
        let row_data = self.row(row)?;
        row_data.cell(col)
    }

    /// 指定された列の値が一致する行を検索
    pub fn find_row(&self, column: &str, value: &str) -> Result<Row> {
        self.rows
            .iter()
            .find(|row| row.get(column).map(|v| v == value).unwrap_or(false))
            .cloned()
            .ok_or_else(|| anyhow!("Row with {}='{}' not found", column, value))
    }
}

/// HTMLリストを表す構造体
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

        // li要素を取得
        let li_selector = Selector::parse("li").unwrap();
        let items: Vec<String> = list_element
            .select(&li_selector)
            .map(|li| li.text().collect::<String>().trim().to_string())
            .collect();

        Ok(Self { items })
    }

    /// リストの全アイテムのテキストを取得
    pub fn items(&self) -> Vec<String> {
        self.items.clone()
    }

    /// 指定されたインデックスのアイテムのテキストを取得
    pub fn item(&self, index: usize) -> Result<String> {
        self.items
            .get(index)
            .cloned()
            .ok_or_else(|| anyhow!("Item index {} out of bounds", index))
    }

    /// リストアイテムの数を返す
    pub fn len(&self) -> usize {
        self.items.len()
    }

    /// リストが空かどうかを返す
    pub fn is_empty(&self) -> bool {
        self.items.is_empty()
    }

    /// 指定されたテキストを含むアイテムが存在するかチェック
    pub fn contains(&self, text: &str) -> bool {
        self.items.iter().any(|item| item == text)
    }
}

/// HTML画像を表す構造体
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

    /// 画像の src 属性を取得
    pub fn src(&self) -> String {
        self.src.clone()
    }

    /// 画像の alt 属性を取得
    pub fn alt(&self) -> Option<String> {
        self.alt.clone()
    }

    /// 画像の width 属性を取得
    pub fn width(&self) -> Option<String> {
        self.width.clone()
    }

    /// 画像の height 属性を取得
    pub fn height(&self) -> Option<String> {
        self.height.clone()
    }
}

/// Selectのオプションを表す構造体
#[derive(Debug, Clone)]
pub struct SelectOption {
    value: String,
    text: String,
    selected: bool,
}

impl SelectOption {
    /// オプションの value 属性を取得
    pub fn value(&self) -> String {
        self.value.clone()
    }

    /// オプションの表示テキストを取得
    pub fn text(&self) -> String {
        self.text.clone()
    }

    /// オプションが選択されているかチェック
    pub fn is_selected(&self) -> bool {
        self.selected
    }
}

/// HTML select 要素を表す構造体
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

        // option 要素を取得
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

    /// 全てのオプションを取得
    pub fn options(&self) -> Vec<SelectOption> {
        self.options.clone()
    }

    /// 選択されているオプションを取得
    pub fn selected_option(&self) -> Result<SelectOption> {
        self.options
            .iter()
            .find(|opt| opt.selected)
            .cloned()
            .ok_or_else(|| anyhow!("No option is selected"))
    }
}
