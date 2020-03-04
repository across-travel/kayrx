# 提取器

`kayrx::web` 提供了一种用于类型安全的请求信息访问的工具, 称为提取器. (即 `impl FromRequest`). 默认情况下，`kayrx::web`提供了几种提取器实现.

提取器可以作为处理函数的参数进行访问。`kayrx::web` 每个处理函数最多支持10个提取器。参数位置无关紧要。

```rust
async fn index(
    path: web::Path<(String, String)>,
    json: web::Json<MyInfo>,
) -> impl Responder {
    format!("{} {} {} {}", path.0, path.1, json.id, json.username)
}
```

## Path

[Path](https://docs.rs/kayrx/0.7.5/kayrx/web/dev/struct.Path.html) (路径)提供可以从请求的路径中提取信息。您可以从路径反序列化任何变量段。

例如，对于在`/users/{userid}/{friend}`路径中注册的资源，可以反序列化两个段，`userid`和`friend`。这些片段可以提取到`tuple`，即 `Path<(u32, String)>`或者从`Serde` crate 中实现该`Deserialize`特征的任何结构

```rust
use kayrx::web::{web, Result};

/// extract path info from "/users/{userid}/{friend}" url
/// {userid} -  - deserializes to a u32
/// {friend} - deserializes to a String
async fn index(info: web::Path<(u32, String)>) -> Result<String> {
    Ok(format!("Welcome {}, userid {}!", info.1, info.0))
}

#[kayrx::main]
async fn main() -> std::io::Result<()> {
    use kayrx::web::{App, HttpServer};

    HttpServer::new(|| {
        App::new().route(
            "/users/{userid}/{friend}", // <- define path parameters
            web::get().to(index),
        )
    })
    .bind("127.0.0.1:8088")?
    .run()
    .await
}
```

还可以提取路径信息(实现`serde`的`Deserialize`特征)到特定类型 。这是一个使用`serde` 而不是`tuple`类型的等效示例

```rust
use kayrx::web::{web, Result};
use serde::Deserialize;

#[derive(Deserialize)]
struct Info {
    userid: u32,
    friend: String,
}

/// extract path info using serde
async fn index(info: web::Path<Info>) -> Result<String> {
    Ok(format!("Welcome {}, userid {}!", info.friend, info.userid))
}

#[kayrx::main]
async fn main() -> std::io::Result<()> {
    use kayrx::web::{App, HttpServer};

    HttpServer::new(|| {
        App::new().route(
            "/users/{userid}/{friend}", // <- define path parameters
            web::get().to(index),
        )
    })
    .bind("127.0.0.1:8088")?
    .run()
    .await
}
```

它也可以通过路径名`get`或`query`请求的参数：

```rust
async fn index(req: HttpRequest) -> Result<String> {
    let name: String =
        req.match_info().get("friend").unwrap().parse().unwrap();
    let userid: i32 = req.match_info().query("userid").parse().unwrap();

    Ok(format!("Welcome {}, userid {}!", name, userid))
}

#[kayrx::main]
async fn main() -> std::io::Result<()> {
    use kayrx::web::{App, HttpServer};

    HttpServer::new(|| {
        App::new().route(
            "/users/{userid}/{friend}", // <- define path parameters
            web::get().to(index),
        )
    })
    .bind("127.0.0.1:8088")?
    .run()
    .await
}
```

## Query

[Query](https://docs.rs/kayrx/0.7.5/kayrx/web/web/struct.Query.html)类型提供用于请求查询参数的提取功能。下面使用了`serde_urlencoded` crate.

```rust
use kayrx::web::web;
use serde::Deserialize;

#[derive(Deserialize)]
struct Info {
    username: String,
}

// this handler get called only if the request's query contains `username` field
async fn index(info: web::Query<Info>) -> String {
    format!("Welcome {}!", info.username)
}
```

## Json

[Json](https://docs.rs/kayrx/0.7.5/kayrx/web/web/struct.Json.html) 允许将请求主体反序列化为结构体。要从请求的正文中提取类型化的信息，类型T必须实现` serde`的`Deserialize` 特征

```rust
use kayrx::web::{web, Result};
use serde::Deserialize;

#[derive(Deserialize)]
struct Info {
    username: String,
}

/// deserialize `Info` from request's body
async fn index(info: web::Json<Info>) -> Result<String> {
    Ok(format!("Welcome {}!", info.username))
}
```

一些提取程序提供了一种配置提取过程的方法。Json提取器的 `JsonConfig`类型用于配置。要配置提取器，请将其配置对象传递给资源的`.data()`方法。如果是Json提取器，它将返回`JsonConfig`。您可以配置`json`有效负载的最大大小以及自定义错误处理函数.

```rust
use kayrx::web::{error, web, FromRequest, HttpResponse, Responder};
use serde::Deserialize;

#[derive(Deserialize)]
struct Info {
    username: String,
}

/// deserialize `Info` from request's body, max payload size is 4kb
async fn index(info: web::Json<Info>) -> impl Responder {
    format!("Welcome {}!", info.username)
}

#[kayrx::main]
async fn main() -> std::io::Result<()> {
    use kayrx::web::{App, HttpServer};

    HttpServer::new(|| {
        App::new().service(
            web::resource("/")
                // change json extractor configuration
                .app_data(web::Json::<Info>::configure(|cfg| {
                    cfg.limit(4096).error_handler(|err, _req| {
                        // create custom error response
                        error::InternalError::from_response(
                            err,
                            HttpResponse::Conflict().finish(),
                        )
                        .into()
                    })
                }))
                .route(web::post().to(index)),
        )
    })
    .bind("127.0.0.1:8088")?
    .run()
    .await
}
```

## Form

目前仅支持`url-encoded` forms. 可以将`url-encoded`的正文提取为特定类型。此类型必须实现`Serde` Crate的`Deserialize`特征.

[FormConfig](https://docs.rs/kayrx/0.7.5/kayrx/web/web/struct.FormConfig.html)允许配置提取过程

```rust
use kayrx::web::{web, Result};
use serde::Deserialize;

#[derive(Deserialize)]
struct FormData {
    username: String,
}

/// extract form data using serde
/// this handler gets called only if the content type is *x-www-form-urlencoded*
/// and the content of the request could be deserialized to a `FormData` struct
async fn index(form: web::Form<FormData>) -> Result<String> {
    Ok(format!("Welcome {}!", form.username))
}
```

## 其他

`kayrx::web` 还提供了其他几种提取器：

- [Data](https://docs.rs/kayrx/0.7.5/kayrx/web/web/struct.Data.html) -如果您需要访问应用程序状态。
- `HttpRequest` - HttpRequest本身是一个提取器，如果需要访问请求，它会返回`self`.
- `String` -您可以将请求的有效负载转换为字符串. [示例](https://docs.rs/kayrx/0.7.5/kayrx/web/web/trait.FromRequest.html#example-2)
- `bytes :: Bytes`-您可以将请求的有效负载转换为`Byte`.  [示例](https://docs.rs/kayrx/0.7.5/kayrx/web/web/trait.FromRequest.html#example-4)
- `Payload` -您可以访问请求的有效负载. [示例](https://docs.rs/kayrx/0.7.5/kayrx/web/web/struct.Payload.html)

## 应用状态

应用状态可通过`web::Data`提取器从处理程序访问；但是，状态作为只读引用进行访问。如果您需要对状态的可变访问，则必须实现它。

> 当心，`kayrx::web`将创建应用程序状态和处理程序的多个副本，这对于每个线程都是唯一的。如果您在多个线程中运行应用程序，`kayrx::web`将创建与线程数量相同的应用程序状态对象和处理程序对象。

这是存储处理请求数的处理程序示例：

```rust
use kayrx::web::{web, Responder};
use std::cell::Cell;

#[derive(Clone)]
struct AppState {
    count: Cell<i32>,
}

async fn show_count(data: web::Data<AppState>) -> impl Responder {
    format!("count: {}", data.count.get())
}

async fn add_one(data: web::Data<AppState>) -> impl Responder {
    let count = data.count.get();
    data.count.set(count + 1);

    format!("count: {}", data.count.get())
}

#[kayrx::main]
async fn main() -> std::io::Result<()> {
    use kayrx::web::{App, HttpServer};

    let data = AppState {
        count: Cell::new(0),
    };

    HttpServer::new(move || {
        App::new()
            .app_data(data.clone())
            .route("/", web::to(show_count))
            .route("/add", web::to(add_one))
    })
    .bind("127.0.0.1:8088")?
    .run()
    .await
}
```

尽管此处理程序可以工作, 但`self.0`根据线程数和每个线程处理的请求数而有所不同。适当的实现将使用`Arc`和`AtomicUsize`.

```rust
use kayrx::web::{web, Responder};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;

#[derive(Clone)]
struct AppState {
    count: Arc<AtomicUsize>,
}

async fn show_count(data: web::Data<AppState>) -> impl Responder {
    format!("count: {}", data.count.load(Ordering::Relaxed))
}

async fn add_one(data: web::Data<AppState>) -> impl Responder {
    data.count.fetch_add(1, Ordering::Relaxed);

    format!("count: {}", data.count.load(Ordering::Relaxed))
}

#[kayrx::main]
async fn main() -> std::io::Result<()> {
    use kayrx::web::{App, HttpServer};

    let data = AppState {
        count: Arc::new(AtomicUsize::new(0)),
    };

    HttpServer::new(move || {
        App::new()
            .app_data(data.clone())
            .route("/", web::to(show_count))
            .route("/add", web::to(add_one))
    })
    .bind("127.0.0.1:8088")?
    .run()
    .await
}
```

> 请注意同步原语，例如`Mutex`或`RwLock`。`kayrx::web`框架异步处理请求。通过阻止线程执行，所有并发请求处理过程都将被阻止。如果需要从多个线程共享或更新某些状态，请考虑使用`kayrx::krse::sync`同步原语.
