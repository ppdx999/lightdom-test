# lightdom-test

Rust で生成したHTMLフォームやボタンの操作をテストするための軽量ライブラリ。

---


## クイックスタート

### 1) HTMLを用意する

```rust
fn login_page() -> String {
    "<form id=\"login-form\" action=\"/login\" method=\"post\">
        <input type=\"hidden\" name=\"_csrf\" value=\"fixed-token\">
        <label for=\"u\">User</label>
        <input id=\"u\" type=\"text\" name=\"username\">
        <label for=\"p\">Pass</label>
        <input id=\"p\" type=\"password\" name=\"password\">
        <button type=\"submit\">Login</button>
    </form>".into()
}
```

### 2) ハンドラを用意する

```rust
use axum::{response::Html as HtmlResp, routing::post, Router, Form};
use serde::Deserialize;

#[derive(Deserialize)]
struct Login { username: String, password: String }

async fn login(Form(f): Form<Login>) -> HtmlResp<String> {
    if f.username == "alice" && f.password == "secret" {
        HtmlResp(format!("Welcome, {}", f.username))
    } else {
        HtmlResp("NG".into())
    }
}

fn app() -> Router {
    Router::new().route("/login", post(login))
}
```

### 3) テストを書く

```rust
use lightdom_test::{Dom, HttpTransport, HttpRequest, HttpResponse};

// Axum 用の Transport 実装（詳細略）
struct AxumTransport;
#[async_trait::async_trait]
impl HttpTransport for AxumTransport {
    async fn send(&self, req: HttpRequest) -> anyhow::Result<HttpResponse> {
        // axum Router を直叩き
        todo!()
    }
}

#[tokio::test]
async fn login_flow() -> anyhow::Result<()> {
    let app = app(); // axum::Router

    let mut form = Dom::new(transporter).parse(login_page().into_string()?)?
        .form("@login-form")?;  // test-id="login-form" のフォームを取得

    form.fill("username", "alice")?
        .fill("password", "secret")?;

    let (status, body) = form.submit(&app).await?;
    assert!(status.is_success());
    assert!(body.contains("Welcome, alice"));
    Ok(())
}
```


## API

### Dom
`Dom` は HTML ドキュメントをパースし、フォームやボタンを操作するためのエントリーポイントです。

| メソッド | 型 | 説明 |
|----------|------|------------------------------------|
| new | (transport: impl HttpTransport) -> Dom | 新しい `Dom` インスタンスを作成します。 |
| parse | (html: String) -> anyhow::Result<Dom> | HTML 文字列をパースし、`Dom` インスタンスを返します。 |
| form | (locator: &str) -> anyhow::Result<Form> | 指定されたロケータに基づいてフォームを取得します。 |
| button | (locator: &str) -> anyhow::Result<Button> | 指定されたロケータに基づいてボタンを取得します。 |
| link | (locator: &str) -> anyhow::Result<Link> | 指定されたロケータに基づいてリンクを取得します。 |


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
| is_exist | (field_name: &str) -> bool | 指定されたフィールドがフォーム内に存在するかチェックします。 |
| fill | (field_name: &str, value: &str) -> anyhow::Result<&mut Form> | 指定されたフィールドに値を入力します。フィールドが存在しない場合や、入力値がフィールドの型に適合しない場合はエラーを返します。 |
| check | (field_name: &str, value: &str) -> anyhow::Result<&mut Form> | チェックボックスをチェックします。複数の値を持つチェックボックスの場合は、複数回呼び出すことで複数選択できます。 |
| uncheck | (field_name: &str, value: &str) -> anyhow::Result<&mut Form> | チェックボックスのチェックを外します。 |
| choose | (field_name: &str, value: &str) -> anyhow::Result<&mut Form> | ラジオボタンを選択します。同じname属性の他のラジオボタンの選択は自動的に解除されます。 |
| select | (field_name: &str, value: &str) -> anyhow::Result<&mut Form> | セレクトボックスのオプションを選択します。 |
| submit | (&self) -> anyhow::Result<HttpResponse> | フォームを送信し、HTTP レスポンスを返します。 |

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
| click | (&self, app: &Router) -> anyhow::Result<HttpResponse> | | フォームを送信し、HTTP レスポンスを返します。 |


### Link
`Link` は HTML リンクを表し、クリック操作を行うためのメソッドを提供します。

| メソッド | 型 | 説明 |
|----------|------|------------------------------------|
| click | (&self, app: &Router) -> anyhow::Result<HttpResponse> | | フォームを送信し、HTTP レスポンスを返します。 |

## 取得系API（計画中）

取得系APIは、HTMLコンテンツからデータを抽出するための機能を提供します。

### Table
`Table` は HTML テーブル (`<table>`) からデータを取得するためのAPIです。

| メソッド | 型 | 説明 |
|----------|------|------------------------------------|
| headers | () -> Vec<String> | テーブルのヘッダー（th要素）を取得します。 |
| rows | () -> Vec<Row> | テーブルの全行を取得します。 |
| row | (index: usize) -> anyhow::Result<Row> | 指定されたインデックスの行を取得します。 |
| cell | (row: usize, col: usize) -> anyhow::Result<String> | 指定された行・列のセルのテキストを取得します。 |
| find_row | (column: &str, value: &str) -> anyhow::Result<Row> | 指定された列の値が一致する行を検索します。 |

#### Row
`Row` はテーブルの1行を表します。

| メソッド | 型 | 説明 |
|----------|------|------------------------------------|
| cells | () -> Vec<String> | 行内の全セルのテキストを取得します。 |
| cell | (index: usize) -> anyhow::Result<String> | 指定されたインデックスのセルのテキストを取得します。 |
| get | (column: &str) -> anyhow::Result<String> | ヘッダー名を指定してセルのテキストを取得します。 |

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
| items | () -> Vec<String> | リストの全アイテムのテキストを取得します。 |
| item | (index: usize) -> anyhow::Result<String> | 指定されたインデックスのアイテムのテキストを取得します。 |
| len | () -> usize | リストアイテムの数を返します。 |
| contains | (text: &str) -> bool | 指定されたテキストを含むアイテムが存在するかチェックします。 |

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
| text | (locator: &str) -> anyhow::Result<String> | 指定されたロケータの要素のテキストを取得します。 |
| texts | (locator: &str) -> Vec<String> | 指定されたロケータに一致する全要素のテキストを取得します。 |
| inner_html | (locator: &str) -> anyhow::Result<String> | 指定されたロケータの要素の内部HTMLを取得します。 |

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
| element | (locator: &str) -> anyhow::Result<Element> | 指定されたロケータの要素を取得します。 |
| elements | (locator: &str) -> Vec<Element> | 指定されたロケータに一致する全要素を取得します。 |

#### Element
`Element` は取得した要素を表します。

| メソッド | 型 | 説明 |
|----------|------|------------------------------------|
| text | () -> String | 要素のテキストコンテンツを取得します。 |
| attr | (name: &str) -> Option<String> | 指定された属性の値を取得します。 |
| has_class | (class: &str) -> bool | 指定されたクラスを持っているかチェックします。 |
| inner_html | () -> String | 要素の内部HTMLを取得します。 |

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

## Transport層

`lightdom-test` は HTTP 送信処理を`HttpTransport`トレイトに抽象化しています。これにより、任意の HTTP クライアントやフレームワークと組み合わせて使用することができます。

### HttpTransport トレイト
`HttpTransport` は HTTP リクエストを送信するためのトレイトです。独自の HTTP クライアントを実装する場合に使用します。

```rust
#[async_trait::async_trait]
pub trait HttpTransport {
    async fn send(&self, req: HttpRequest) -> anyhow::Result<HttpResponse>;
}
```

#### HttpRequest
`HttpRequest` は HTTP リクエストを表す構造体です。
```rust
pub struct HttpRequest {
    method: Method,
    url: String,
    headers: HashMap<String, String>,
    body: Option<String>,
}
```

#### HttpResponse
`HttpResponse` は HTTP レスポンスを表す構造体です。
```rust
pub struct HttpResponse {
    status: StatusCode,
    headers: HashMap<String, String>,
    body: String,
}
```


## 目指す哲学

- **軽量・高速**: 大規模なブラウザ自動化ツールを使用せず、シンプルで高速なテストを可能にします。
- **Rust ネイティブ**: Rust のエコシステムとシームレスに統合できるよう設計されています。
- **シンプルさ**: 直感的で使いやすい API を提供し、学習コストを最小限に抑えます。
- **柔軟性**: 任意の HTTP クライアントやフレームワークと組み合わせて使用できるように設計されています。
