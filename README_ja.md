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

    let form = Dom::new(transporter).parse(login_page().into_string()?)?
        .form("@login-form")?  // test-id="login-form" のフォームを取得
        .fill("username", "alice")
        .fill("password", "secret");

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
| fill | (field_name: &str, value: &str) -> &mut Form | 指定されたフィールドに値を入力します。 |
| submit | (&self, app: &Router) -> anyhow::Result<HttpResponse> | フォームを送信し、HTTP レスポンスを返します。 |

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
