use anyhow::{anyhow, Result};
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
}

/// HTML フォームを表す構造体
#[derive(Debug)]
pub struct Form<T: HttpTransport> {
    action: String,
    method: String,
    fields: HashMap<String, String>,
    field_types: HashMap<String, String>,
    // checkbox/radio の情報
    checkboxes: HashMap<String, Vec<String>>,  // name -> [values]
    radios: HashMap<String, Vec<String>>,      // name -> [values]
    checked_checkboxes: HashSet<String>,       // "name=value" のセット
    selected_radios: HashMap<String, String>,  // name -> selected value
    transport: Arc<T>,
}

impl<T: HttpTransport> Form<T> {
    fn find(html: &str, locator: &str, transport: Arc<T>) -> Result<Self> {
        let document = Html::parse_document(html);

        // ロケータに基づいてセレクタを生成
        let selector_str = if locator.starts_with('@') {
            // test-id 属性
            format!("form[test-id=\"{}\"]", &locator[1..])
        } else if locator.starts_with('#') {
            // id 属性
            format!("form{}", locator)
        } else if locator.starts_with('/') {
            // action 属性
            format!("form[action=\"{}\"]", locator)
        } else {
            return Err(anyhow!("Invalid locator: {}", locator));
        };

        let form_selector = Selector::parse(&selector_str)
            .map_err(|e| anyhow!("Invalid selector: {:?}", e))?;

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
        let input_selector = Selector::parse("input")
            .map_err(|e| anyhow!("Invalid selector: {:?}", e))?;

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
                            checkboxes.entry(name.to_string())
                                .or_insert_with(Vec::new)
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
                            radios.entry(name.to_string())
                                .or_insert_with(Vec::new)
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
        let textarea_selector = Selector::parse("textarea")
            .map_err(|e| anyhow!("Invalid selector: {:?}", e))?;
        for textarea in form_element.select(&textarea_selector) {
            if let Some(name) = textarea.value().attr("name") {
                field_types.insert(name.to_string(), "textarea".to_string());
            }
        }

        let select_selector = Selector::parse("select")
            .map_err(|e| anyhow!("Invalid selector: {:?}", e))?;
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

    /// フィールドに値を入力
    pub fn fill(&mut self, field_name: &str, value: &str) -> Result<&mut Self> {
        // フィールドが存在するかチェック
        let field_type = self.field_types.get(field_name)
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
                if !value.starts_with("http://") && !value.starts_with("https://") && !value.is_empty() {
                    return Err(anyhow!("Invalid URL format for field '{}'. Must start with http:// or https://", field_name));
                }
            }
            "tel" => {
                // 電話番号は数字、ハイフン、スペース、括弧、+のみ許可
                if !value.chars().all(|c| c.is_numeric() || c == '-' || c == ' ' || c == '(' || c == ')' || c == '+') {
                    return Err(anyhow!("Invalid phone number format for field '{}'", field_name));
                }
            }
            "date" => {
                // YYYY-MM-DD形式をチェック
                let parts: Vec<&str> = value.split('-').collect();
                if parts.len() != 3 {
                    return Err(anyhow!("Invalid date format for field '{}'. Expected YYYY-MM-DD", field_name));
                }
                if parts[0].len() != 4 || parts[1].len() != 2 || parts[2].len() != 2 {
                    return Err(anyhow!("Invalid date format for field '{}'. Expected YYYY-MM-DD", field_name));
                }
                for part in &parts {
                    if part.parse::<u32>().is_err() {
                        return Err(anyhow!("Invalid date format for field '{}'. Expected YYYY-MM-DD", field_name));
                    }
                }
            }
            _ => {
                // text, password, hidden, textarea, select などはバリデーション不要
            }
        }

        self.fields.insert(field_name.to_string(), value.to_string());
        Ok(self)
    }

    /// チェックボックスをチェックする
    pub fn check(&mut self, field_name: &str, value: &str) -> Result<&mut Self> {
        // チェックボックスが存在するかチェック
        let checkbox_values = self.checkboxes.get(field_name)
            .ok_or_else(|| anyhow!("Checkbox '{}' does not exist in the form", field_name))?;

        // 指定された値が存在するかチェック
        if !checkbox_values.contains(&value.to_string()) {
            return Err(anyhow!("Checkbox '{}' does not have value '{}'", field_name, value));
        }

        // チェック状態に設定
        self.checked_checkboxes.insert(format!("{}={}", field_name, value));
        Ok(self)
    }

    /// チェックボックスのチェックを外す
    pub fn uncheck(&mut self, field_name: &str, value: &str) -> Result<&mut Self> {
        // チェックボックスが存在するかチェック
        let checkbox_values = self.checkboxes.get(field_name)
            .ok_or_else(|| anyhow!("Checkbox '{}' does not exist in the form", field_name))?;

        // 指定された値が存在するかチェック
        if !checkbox_values.contains(&value.to_string()) {
            return Err(anyhow!("Checkbox '{}' does not have value '{}'", field_name, value));
        }

        // チェックを外す
        self.checked_checkboxes.remove(&format!("{}={}", field_name, value));
        Ok(self)
    }

    /// ラジオボタンを選択する
    pub fn choose(&mut self, field_name: &str, value: &str) -> Result<&mut Self> {
        // ラジオボタンが存在するかチェック
        let radio_values = self.radios.get(field_name)
            .ok_or_else(|| anyhow!("Radio button '{}' does not exist in the form", field_name))?;

        // 指定された値が存在するかチェック
        if !radio_values.contains(&value.to_string()) {
            return Err(anyhow!("Radio button '{}' does not have value '{}'", field_name, value));
        }

        // ラジオボタンを選択
        self.selected_radios.insert(field_name.to_string(), value.to_string());
        Ok(self)
    }

    /// セレクトボックスのオプションを選択する
    pub fn select(&mut self, field_name: &str, value: &str) -> Result<&mut Self> {
        // セレクトボックスが存在するかチェック
        let field_type = self.field_types.get(field_name)
            .ok_or_else(|| anyhow!("Select '{}' does not exist in the form", field_name))?;

        if field_type != "select" {
            return Err(anyhow!("Field '{}' is not a select element", field_name));
        }

        // 値を設定（オプションの存在チェックは実際のHTMLから行うのが理想だが、簡略化のため省略）
        self.fields.insert(field_name.to_string(), value.to_string());
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
        let selector_str = if locator.starts_with('@') {
            // test-id 属性
            format!("button[test-id=\"{}\"]", &locator[1..])
        } else if locator.starts_with('#') {
            // id 属性
            format!("button{}", locator)
        } else {
            // テキストで検索（contains）は後で処理
            "button".to_string()
        };

        let button_selector = Selector::parse(&selector_str)
            .map_err(|e| anyhow!("Invalid selector: {:?}", e))?;

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
            if let Some(element) = ancestor.value().as_element() {
                if element.name() == "form" {
                    form_action = ancestor.value().as_element()
                        .and_then(|e| e.attr("action"))
                        .map(|s| s.to_string());
                    form_method = ancestor.value().as_element()
                        .and_then(|e| e.attr("method"))
                        .map(|s| s.to_string());
                    break;
                }
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
        let action = self.form_action.as_ref()
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
        let selector_str = if locator.starts_with('@') {
            // test-id 属性
            format!("a[test-id=\"{}\"]", &locator[1..])
        } else if locator.starts_with('#') {
            // id 属性
            format!("a{}", locator)
        } else {
            // テキストで検索
            "a".to_string()
        };

        let link_selector = Selector::parse(&selector_str)
            .map_err(|e| anyhow!("Invalid selector: {:?}", e))?;

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

        Ok(Self {
            href,
            transport,
        })
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
