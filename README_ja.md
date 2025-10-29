# lightdom-test

Rust で生成したHTMLフォームやボタンの操作をテストするための軽量ライブラリ。

---


## クイックスタート

### 1) HTMLを用意する

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

### 2) MockTransport を実装する

```rust
use lightdom_test::{HttpTransport, HttpRequest, HttpResponse, StatusCode};
use anyhow::Result;

struct MockTransport;

#[async_trait::async_trait]
impl HttpTransport for MockTransport {
    async fn send(&self, req: HttpRequest) -> Result<HttpResponse> {
        // リクエストの内容に応じてレスポンスを返す
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

### 3) テストを書く

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
`Dom` は HTML ドキュメントをパースし、フォームやボタンを操作するためのエントリーポイントです。

| メソッド | 型 | 説明 |
|----------|------|------------------------------------|
| new | `(transport: impl HttpTransport) -> Dom` | 新しい `Dom` インスタンスを作成します。 |
| parse | `(html: String) -> anyhow::Result<Dom>` | HTML 文字列をパースし、`Dom` インスタンスを返します。 |
| form | `(locator: &str) -> anyhow::Result<Form>` | 指定されたロケータに基づいてフォームを取得します。 |
| button | `(locator: &str) -> anyhow::Result<Button>` | 指定されたロケータに基づいてボタンを取得します。 |
| link | `(locator: &str) -> anyhow::Result<Link>` | 指定されたロケータに基づいてリンクを取得します。 |
| element | `(locator: &str) -> anyhow::Result<Element>` | 指定されたロケータの要素を取得します。 |
| elements | `(locator: &str) -> Vec<Element>` | 指定されたロケータに一致する全要素を取得します。 |
| text | `(locator: &str) -> anyhow::Result<String>` | 指定されたロケータの要素のテキストを取得します。 |
| texts | `(locator: &str) -> Vec<String>` | 指定されたロケータに一致する全要素のテキストを取得します。 |
| inner_html | `(locator: &str) -> anyhow::Result<String>` | 指定されたロケータの要素の内部HTMLを取得します。 |
| table | `(locator: &str) -> anyhow::Result<Table>` | 指定されたロケータのテーブルを取得します。 |
| list | `(locator: &str) -> anyhow::Result<List>` | 指定されたロケータのリストを取得します。 |
| title | `() -> anyhow::Result<String>` | `<title>` タグの内容を取得します。 |
| meta | `(name: &str) -> anyhow::Result<String>` | メタタグの content 属性を取得します。 |
| exists | `(locator: &str) -> bool` | 指定されたロケータの要素が存在するかチェックします。 |
| contains_text | `(text: &str) -> bool` | 指定されたテキストを含む要素が存在するかチェックします。 |
| select_element | `(locator: &str) -> anyhow::Result<SelectElement>` | 指定されたロケータの select 要素を取得します。 |
| image | `(locator: &str) -> anyhow::Result<Image>` | 指定されたロケータの画像を取得します。 |
| images | `(locator: &str) -> Vec<Image>` | 指定されたロケータに一致する全画像を取得します。 |


`form`で指定できるロケータの種類は以下の通りです。

| ロケータ | 説明 |
|----------|------|
| @login-form | `test-id` 属性が `login-form` のフォームを特定します。 |
| #login-form | `id` 属性が `login-form` のフォームを特定します。 |
| /login | `action` 属性が `/login` のフォームを特定します。 |

`button`で指定できるロケータの種類は以下の通りです。

| ロケータ | 説明 |
|----------|------|
| @submit-btn | `test-id` 属性が `submit-btn` のボタンを特定します。 |
| #submit-btn | `id` 属性が `submit-btn` のボタンを特定します。 |
| Login | ボタンの表示テキストが `Login` のボタンを特定します。 |

`link`で指定できるロケータの種類は以下の通りです。

| ロケータ | 説明 |
|----------|------|
| @home-link | `test-id` 属性が `home-link` のリンクを特定します。 |
| #home-link | `id` 属性が `home-link` のリンクを特定します。 |
| Home | リンクの表示テキストが `Home` のリンクを特定します。 |

### Form

`Form` は HTML フォームを表し、フィールドの入力やフォームの送信を行うためのメソッドを提供します。

| メソッド | 型 | 説明 |
|----------|------|------------------------------------|
| is_exist | `(field_name: &str) -> bool` | 指定されたフィールドがフォーム内に存在するかチェックします。 |
| get_value | `(field_name: &str) -> anyhow::Result<String>` | 指定されたフィールドの現在値を取得します。 |
| fill | `(field_name: &str, value: &str) -> anyhow::Result<&mut Form>` | 指定されたフィールドに値を入力します。フィールドが存在しない場合や、入力値がフィールドの型に適合しない場合はエラーを返します。 |
| check | `(field_name: &str, value: &str) -> anyhow::Result<&mut Form>` | チェックボックスをチェックします。複数の値を持つチェックボックスの場合は、複数回呼び出すことで複数選択できます。 |
| uncheck | `(field_name: &str, value: &str) -> anyhow::Result<&mut Form>` | チェックボックスのチェックを外します。 |
| choose | `(field_name: &str, value: &str) -> anyhow::Result<&mut Form>` | ラジオボタンを選択します。同じname属性の他のラジオボタンの選択は自動的に解除されます。 |
| select | `(field_name: &str, value: &str) -> anyhow::Result<&mut Form>` | セレクトボックスのオプションを選択します。 |
| submit | `(&self) -> anyhow::Result<HttpResponse>` | フォームを送信し、HTTP レスポンスを返します。 |

#### fill メソッドのバリデーション

`fill` メソッドは、フィールドの `type` 属性に応じて入力値を自動的にバリデーションします：

| type 属性 | バリデーション内容 |
|-----------|-------------------|
| email | `@` を含むかチェック |
| number | 数値として解析可能かチェック |
| url | `http://` または `https://` で始まるかチェック |
| tel | 数字、ハイフン、スペース、括弧、`+` のみ許可 |
| date | `YYYY-MM-DD` 形式かチェック |
| text, password, hidden, textarea, select など | バリデーションなし |

```rust
// 正常なケース
form.fill("email", "user@example.com")?;  // OK
form.fill("age", "25")?;                   // OK

// エラーケース
form.fill("email", "invalid-email")?;     // Err: Invalid email format
form.fill("age", "not-a-number")?;         // Err: Invalid number format
form.fill("nonexistent", "value")?;        // Err: Field does not exist
```

#### is_exist メソッドの使用例

```rust
// フィールドの存在チェック
if form.is_exist("username") {
    form.fill("username", "alice")?;
}

// 条件付き処理
if form.is_exist("email") && form.is_exist("phone") {
    // 両方のフィールドが存在する場合のみ入力
    form.fill("email", "alice@example.com")?
        .fill("phone", "123-456-7890")?;
}
```

#### チェックボックス・ラジオボタン・セレクトボックスの使用例

```rust
// チェックボックス（複数選択可）
form.check("interests", "sports")?
    .check("interests", "music")?;

// チェックボックスのチェックを外す
form.uncheck("agree", "terms")?;

// ラジオボタン（単一選択）
form.choose("gender", "female")?;

// セレクトボックス
form.select("country", "japan")?;

// 複合的な使用例
form.fill("username", "alice")?
    .fill("email", "alice@example.com")?
    .check("notifications", "email")?
    .check("notifications", "sms")?
    .choose("plan", "premium")?
    .select("country", "jp")?
    .submit().await?;
```

### Button
`Button` は HTML ボタンを表し、クリック操作を行うためのメソッドを提供します。

| メソッド | 型 | 説明 |
|----------|------|------------------------------------|
| click | `(&self) -> anyhow::Result<HttpResponse>` | ボタンをクリックし、関連するフォームを送信します。HTTP レスポンスを返します。 |

#### 使用例
```rust
let button = dom.button("#submit-btn")?;
let response = button.click().await?;
assert!(response.status.is_success());
```

### Link
`Link` は HTML リンクを表し、クリック操作を行うためのメソッドを提供します。

| メソッド | 型 | 説明 |
|----------|------|------------------------------------|
| click | `(&self) -> anyhow::Result<HttpResponse>` | リンクをクリックし、href 先に GET リクエストを送信します。HTTP レスポンスを返します。 |

#### 使用例
```rust
let link = dom.link("Home")?;
let response = link.click().await?;
assert_eq!(response.status.0, 200);
```

## 取得系API

取得系APIは、HTMLコンテンツからデータを抽出するための機能を提供します。

### Table
`Table` は HTML テーブル (`<table>`) からデータを取得するためのAPIです。

| メソッド | 型 | 説明 |
|----------|------|------------------------------------|
| headers | `() -> Vec<String>` | テーブルのヘッダー（th要素）を取得します。 |
| rows | `() -> Vec<Row>` | テーブルの全行を取得します。 |
| row | `(index: usize) -> anyhow::Result<Row>` | 指定されたインデックスの行を取得します。 |
| cell | `(row: usize, col: usize) -> anyhow::Result<String>` | 指定された行・列のセルのテキストを取得します。 |
| find_row | `(column: &str, value: &str) -> anyhow::Result<Row>` | 指定された列の値が一致する行を検索します。 |

#### Row
`Row` はテーブルの1行を表します。

| メソッド | 型 | 説明 |
|----------|------|------------------------------------|
| cells | `() -> Vec<String>` | 行内の全セルのテキストを取得します。 |
| cell | `(index: usize) -> anyhow::Result<String>` | 指定されたインデックスのセルのテキストを取得します。 |
| get | `(column: &str) -> anyhow::Result<String>` | ヘッダー名を指定してセルのテキストを取得します。 |

#### 使用例
```rust
let table = dom.table("#users-table")?;

// ヘッダーの取得
let headers = table.headers();
assert_eq!(headers, vec!["Name", "Email", "Status"]);

// 全行の取得
for row in table.rows() {
    let cells = row.cells();
    println!("{:?}", cells);
}

// 特定のセルにアクセス
let name = table.cell(0, 0)?; // 1行目、1列目
assert_eq!(name, "Alice");

// 列名を使った行の検索
let row = table.find_row("Email", "alice@example.com")?;
let status = row.get("Status")?;
assert_eq!(status, "Active");
```

### List
`List` は HTML リスト (`<ul>`, `<ol>`) からデータを取得するためのAPIです。

| メソッド | 型 | 説明 |
|----------|------|------------------------------------|
| items | `() -> Vec<String>` | リストの全アイテムのテキストを取得します。 |
| item | `(index: usize) -> anyhow::Result<String>` | 指定されたインデックスのアイテムのテキストを取得します。 |
| len | `() -> usize` | リストアイテムの数を返します。 |
| contains | `(text: &str) -> bool` | 指定されたテキストを含むアイテムが存在するかチェックします。 |

#### 使用例
```rust
let list = dom.list("#todo-list")?;

// 全アイテムの取得
let items = list.items();
assert_eq!(items.len(), 3);

// 特定のアイテムにアクセス
let first = list.item(0)?;
assert_eq!(first, "Buy groceries");

// アイテムの存在確認
assert!(list.contains("Buy groceries"));
```

### Text
`Text` は HTML 要素のテキストコンテンツを取得するためのAPIです。

| メソッド | 型 | 説明 |
|----------|------|------------------------------------|
| text | `(locator: &str) -> anyhow::Result<String>` | 指定されたロケータの要素のテキストを取得します。 |
| texts | `(locator: &str) -> Vec<String>` | 指定されたロケータに一致する全要素のテキストを取得します。 |
| inner_html | `(locator: &str) -> anyhow::Result<String>` | 指定されたロケータの要素の内部HTMLを取得します。 |

`text` で指定できるロケータの種類は以下の通りです。

| ロケータ | 説明 |
|----------|------|
| @message | `test-id` 属性が `message` の要素を特定します。 |
| #message | `id` 属性が `message` の要素を特定します。 |
| .message | `class` 属性が `message` の要素を特定します。 |

#### 使用例
```rust
let dom = Dom::new(transport).parse(html)?;

// 単一要素のテキスト取得
let message = dom.text("#welcome-message")?;
assert_eq!(message, "Welcome, Alice!");

// 複数要素のテキスト取得
let errors = dom.texts(".error-message");
assert_eq!(errors, vec!["Invalid email", "Password too short"]);

// 内部HTMLの取得
let content = dom.inner_html("#content")?;
assert!(content.contains("<p>"));
```

### Element
`Element` は汎用的な要素の取得と属性アクセスを提供します。

| メソッド | 型 | 説明 |
|----------|------|------------------------------------|
| element | `(locator: &str) -> anyhow::Result<Element>` | 指定されたロケータの要素を取得します。 |
| elements | `(locator: &str) -> Vec<Element>` | 指定されたロケータに一致する全要素を取得します。 |

#### Element
`Element` は取得した要素を表します。

| メソッド | 型 | 説明 |
|----------|------|------------------------------------|
| text | `() -> String` | 要素のテキストコンテンツを取得します。 |
| attr | `(name: &str) -> Option<String>` | 指定された属性の値を取得します。 |
| has_class | `(class: &str) -> bool` | 指定されたクラスを持っているかチェックします。 |
| inner_html | `() -> String` | 要素の内部HTMLを取得します。 |
| text_contains | `(text: &str) -> bool` | 要素のテキストが指定された文字列を含むかチェックします。 |
| is_disabled | `() -> bool` | disabled 属性を持っているかチェックします。 |
| is_required | `() -> bool` | required 属性を持っているかチェックします。 |
| is_readonly | `() -> bool` | readonly 属性を持っているかチェックします。 |
| is_checked | `() -> bool` | checked 属性を持っているかチェックします。 |

#### 使用例
```rust
let element = dom.element("#user-profile")?;

// テキストの取得
let text = element.text();

// 属性の取得
let user_id = element.attr("data-user-id");
assert_eq!(user_id, Some("123".to_string()));

// クラスの確認
assert!(element.has_class("active"));

// 複数要素の処理
for elem in dom.elements(".product-item") {
    let name = elem.attr("data-name").unwrap();
    let price = elem.text();
    println!("{}: {}", name, price);
}
```

### Meta Tags
`Dom` はメタタグや title タグを取得するための API を提供します。SSR アプリケーションの SEO テストに便利です。

| メソッド | 型 | 説明 |
|----------|------|------------------------------------|
| title | `() -> Result<String>` | `<title>` タグの内容を取得します。 |
| meta | `(name: &str) -> Result<String>` | `<meta name="...">` または `<meta property="...">` の content 属性を取得します。 |

#### 使用例
```rust
let dom = Dom::new(transport).parse(html)?;

// タイトルの取得
let title = dom.title()?;
assert_eq!(title, "Welcome - My Site");

// メタタグの取得
let description = dom.meta("description")?;
assert_eq!(description, "This is my website");

// OGP タグの取得
let og_title = dom.meta("og:title")?;
assert_eq!(og_title, "Welcome");
```

### Exists Check
要素の存在確認を行う API です。

| メソッド | 型 | 説明 |
|----------|------|------------------------------------|
| exists | `(locator: &str) -> bool` | 指定されたロケータの要素が存在するかチェックします。 |
| contains_text | `(text: &str) -> bool` | 指定されたテキストを含む要素が存在するかチェックします。 |

#### 使用例
```rust
// 要素の存在確認
assert!(dom.exists("#error-message"));
assert!(!dom.exists("#success-message"));

// テキストの存在確認
assert!(dom.contains_text("Welcome"));
assert!(!dom.contains_text("Error"));
```

### Select Element
`SelectElement` は `<select>` 要素のオプションを取得するための API です。

| メソッド | 型 | 説明 |
|----------|------|------------------------------------|
| select_element | `(locator: &str) -> Result<SelectElement>` | 指定されたロケータの select 要素を取得します。 |

#### SelectElement
| メソッド | 型 | 説明 |
|----------|------|------------------------------------|
| options | `() -> Vec<SelectOption>` | 全てのオプションを取得します。 |
| selected_option | `() -> Result<SelectOption>` | 選択されているオプションを取得します。 |

#### SelectOption
| メソッド | 型 | 説明 |
|----------|------|------------------------------------|
| value | `() -> String` | オプションの value 属性を取得します。 |
| text | `() -> String` | オプションの表示テキストを取得します。 |
| is_selected | `() -> bool` | オプションが選択されているかチェックします。 |

#### 使用例
```rust
let select = dom.select_element("#country")?;

// 全オプションの取得
let options = select.options();
assert_eq!(options.len(), 3);
assert_eq!(options[0].value(), "jp");
assert_eq!(options[0].text(), "Japan");

// 選択されているオプションの取得
let selected = select.selected_option()?;
assert_eq!(selected.value(), "us");
assert!(selected.is_selected());
```

### Element State
`Element` に要素の状態をチェックするメソッドが追加されます。

| メソッド | 型 | 説明 |
|----------|------|------------------------------------|
| is_disabled | `() -> bool` | disabled 属性を持っているかチェックします。 |
| is_required | `() -> bool` | required 属性を持っているかチェックします。 |
| is_readonly | `() -> bool` | readonly 属性を持っているかチェックします。 |
| is_checked | `() -> bool` | checked 属性を持っているかチェックします（checkbox/radio）。 |

#### 使用例
```rust
let button = dom.element("#submit-btn")?;
assert!(button.is_disabled());

let email = dom.element("#email")?;
assert!(email.is_required());
assert!(!email.is_readonly());

let checkbox = dom.element("#agree")?;
assert!(checkbox.is_checked());
```

### Image
`Dom` は画像要素を取得するための API を提供します。

| メソッド | 型 | 説明 |
|----------|------|------------------------------------|
| image | `(locator: &str) -> Result<Image>` | 指定されたロケータの画像を取得します。 |
| images | `(locator: &str) -> Vec<Image>` | 指定されたロケータに一致する全画像を取得します。 |

#### Image
| メソッド | 型 | 説明 |
|----------|------|------------------------------------|
| src | `() -> String` | 画像の src 属性を取得します。 |
| alt | `() -> Option<String>` | 画像の alt 属性を取得します。 |
| width | `() -> Option<String>` | 画像の width 属性を取得します。 |
| height | `() -> Option<String>` | 画像の height 属性を取得します。 |

#### 使用例
```rust
let img = dom.image("#logo")?;
assert_eq!(img.src(), "/logo.png");
assert_eq!(img.alt(), Some("Company Logo".to_string()));

// 全画像の取得
let images = dom.images("img");
for img in images {
    println!("{}: {}", img.src(), img.alt().unwrap_or_default());
}
```

### Text Search
`Element` にテキストの部分一致検索メソッドが追加されます。

| メソッド | 型 | 説明 |
|----------|------|------------------------------------|
| text_contains | `(text: &str) -> bool` | 要素のテキストが指定された文字列を含むかチェックします。 |

#### 使用例
```rust
let message = dom.element("#message")?;
assert!(message.text_contains("Success"));
assert!(!message.text_contains("Error"));
```

### Form Value
`Form` にフィールドの初期値を取得するメソッドが追加されます。

| メソッド | 型 | 説明 |
|----------|------|------------------------------------|
| get_value | `(field_name: &str) -> Result<String>` | 指定されたフィールドの現在値を取得します。 |

#### 使用例
```rust
let form = dom.form("#edit-form")?;

// 初期値の確認
let username = form.get_value("username")?;
assert_eq!(username, "alice");

// 値を変更
form.fill("username", "bob")?;
let new_value = form.get_value("username")?;
assert_eq!(new_value, "bob");
```

## Transport層

`lightdom-test` は HTTP 送信処理を`HttpTransport`トレイトに抽象化しています。これにより、任意の HTTP クライアントやフレームワークと組み合わせて使用することができます。

### HttpTransport トレイト
`HttpTransport` は HTTP リクエストを送信するためのトレイトです。独自の HTTP クライアントを実装する場合に使用します。

```rust
#[async_trait::async_trait]
pub trait HttpTransport: Send + Sync {
    async fn send(&self, req: HttpRequest) -> anyhow::Result<HttpResponse>;
}
```

#### HttpRequest
`HttpRequest` は HTTP リクエストを表す構造体です。
```rust
pub struct HttpRequest {
    pub method: Method,
    pub url: String,
    pub headers: HashMap<String, String>,
    pub body: Option<String>,
}
```

#### HttpResponse
`HttpResponse` は HTTP レスポンスを表す構造体です。
```rust
pub struct HttpResponse {
    pub status: StatusCode,
    pub headers: HashMap<String, String>,
    pub body: String,
}
```

### Transport実装例

テストには MockTransport、本番環境では実際のHTTPクライアント（reqwest等）を使用できます：

```rust
use std::sync::{Arc, Mutex};

// テスト用: リクエストをキャプチャする MockTransport
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

// 本番用: reqwest を使った実装例
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

## フレームワーク統合

`lightdom-test` は主要な Rust Web フレームワークとの統合をオプション機能として提供しています。

### Axum 統合

Axum フレームワークを使用している場合、`AxumTransport` を使って HTTPサーバーを起動せずに直接 Router をテストできます。

#### 有効化

`Cargo.toml` に `axum` feature を追加します：

```toml
[dev-dependencies]
lightdom-test = { version = "0.1", features = ["axum"] }
axum = "0.7"
tokio = { version = "1", features = ["full"] }
```

#### 使用例

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
    // Axum Router を作成
    let app = Router::new()
        .route("/login", post(login_handler));

    // AxumTransport を使用
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

#### 利点

- **高速**: HTTP サーバーを起動する必要がないため、テストが高速に実行されます
- **ポート管理不要**: ランダムポートの割り当てやポート衝突の心配がありません
- **シンプル**: `Router` を直接渡すだけで使用できます

### Rocket 統合

Rocket フレームワークを使用している場合、`RocketTransport` を使って HTTPサーバーを起動せずに直接 Rocket インスタンスをテストできます。

#### 有効化

`Cargo.toml` に `rocket` feature を追加します：

```toml
[dev-dependencies]
lightdom-test = { version = "0.1", features = ["rocket"] }
rocket = "0.5"
tokio = { version = "1", features = ["full"] }
```

#### 使用例

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
    // Rocket インスタンスを作成
    let rocket = rocket::build()
        .mount("/", routes![login_handler]);

    // RocketTransport を使用
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

#### 利点

- **高速**: HTTP サーバーを起動する必要がないため、テストが高速に実行されます
- **ポート管理不要**: ランダムポートの割り当てやポート衝突の心配がありません
- **Rocket の機能をフル活用**: ミドルウェア、Fairings、State などの Rocket の全機能をテストできます


## 目指す哲学

- **軽量・高速**: 大規模なブラウザ自動化ツールを使用せず、シンプルで高速なテストを可能にします。
- **Rust ネイティブ**: Rust のエコシステムとシームレスに統合できるよう設計されています。
- **シンプルさ**: 直感的で使いやすい API を提供し、学習コストを最小限に抑えます。
- **柔軟性**: 任意の HTTP クライアントやフレームワークと組み合わせて使用できるように設計されています。
