# 响应

用类似构建器的模式构建`HttpResponse`的实例.  `HttpResponse` 提供了几种返回[HttpResponseBuilder](https://docs.rs/kayrx/0.7.5/kayrx/web/dev/struct.HttpResponseBuilder.html)实例的方法，该方法实现了用于构建响应的各种便捷方法。

方法`.body`，`.finish`和`.json`创建完成响应，并返回一个`HttpResponse`的实例构造。如果在同一构建器实例上多次调用此方法，则构建器将出现恐慌

```rust
use kayrx::web::HttpResponse;

async fn index() -> HttpResponse {
    HttpResponse::Ok()
        .content_type("plain/text")
        .header("X-Hdr", "sample")
        .body("data")
}
```

## 内容编码

`kayrx::web`可以使用[Compress](https://docs.rs/kayrx/0.7.5/kayrx/web/middleware/struct.Compress.html)中间件自动压缩内容负载, 支持以下编解码器：

* Brotli
* Gzip
* Deflate
* Identity

```rust
use kayrx::web::{middleware, HttpResponse};

async fn index_br() -> HttpResponse {
    HttpResponse::Ok().body("data")
}

#[kayrx::main]
async fn main() -> std::io::Result<()> {
    use kayrx::web::{web, App, HttpServer};

    HttpServer::new(|| {
        App::new()
            .wrap(middleware::Compress::default())
            .route("/", web::get().to(index_br))
    })
    .bind("127.0.0.1:8088")?
    .run()
    .await
}
```

基于`middleware::BodyEncoding`特征的编码参数压缩响应负载, 默认情况下使用`ContentEncoding::Auto`, 如果选择 `ContentEncoding::Auto`，则压缩取决于请求的 `Accept-Encoding`标头.

> `ContentEncoding::Identity`可用于禁用压缩。如果选择了其他内容编码，则将对该编解码器禁用该压缩。

例如，要对某个处理程序启用`brotli`，请使用`ContentEncoding::Br`：

```rust
use kayrx::web::{http::ContentEncoding, dev::BodyEncoding, HttpResponse};

async fn index_br() -> HttpResponse {
    HttpResponse::Ok()
        .encoding(ContentEncoding::Br)
        .body("data")
}

#[kayrx::main]
async fn main() -> std::io::Result<()> {
    use kayrx::web::{middleware, web, App, HttpServer};

    HttpServer::new(|| {
        App::new()
            .wrap(middleware::Compress::default())
            .route("/", web::get().to(index_br))
    })
    .bind("127.0.0.1:8088")?
    .run()
    .await
}
```

或对于整个应用程序:

```rust
use kayrx::web::{http::ContentEncoding, dev::BodyEncoding, HttpResponse};

async fn index_br() -> HttpResponse {
    HttpResponse::Ok().body("data")
}

#[kayrx::main]
async fn main() -> std::io::Result<()> {
    use kayrx::web::{middleware, web, App, HttpServer};

    HttpServer::new(|| {
        App::new()
            .wrap(middleware::Compress::new(ContentEncoding::Br))
            .route("/", web::get().to(index_br))
    })
    .bind("127.0.0.1:8088")?
    .run()
    .await
}
```

在这种情况下，我们通过将内容编码设置为一个`Identity`值来显式禁用内容压缩：

```rust
use kayrx::web::{
    http::ContentEncoding, middleware, dev::BodyEncoding, HttpResponse,
};

async fn index() -> HttpResponse {
    HttpResponse::Ok()
        // v- disable compression
        .encoding(ContentEncoding::Identity)
        .body("data")
}

#[kayrx::main]
async fn main() -> std::io::Result<()> {
    use kayrx::web::{web, App, HttpServer};

    HttpServer::new(|| {
        App::new()
            .wrap(middleware::Compress::default())
            .route("/", web::get().to(index))
    })
    .bind("127.0.0.1:8088")?
    .run()
    .await
}
```

处理已压缩的主体时（例如，在提供`assets`服务时），请将内容编码设置`Identity`为避免压缩已压缩的数据，并手动设置`content-encoding`标头：

```rust
use kayrx::web::{
    http::ContentEncoding, middleware, dev::BodyEncoding, HttpResponse,
};

static HELLO_WORLD: &[u8] = &[
    0x1f, 0x8b, 0x08, 0x00, 0xa2, 0x30, 0x10, 0x5c, 0x00, 0x03, 0xcb, 0x48, 0xcd, 0xc9,
    0xc9, 0x57, 0x28, 0xcf, 0x2f, 0xca, 0x49, 0xe1, 0x02, 0x00, 0x2d, 0x3b, 0x08, 0xaf,
    0x0c, 0x00, 0x00, 0x00,
];

async fn index() -> HttpResponse {
    HttpResponse::Ok()
        .encoding(ContentEncoding::Identity)
        .header("content-encoding", "gzip")
        .body(HELLO_WORLD)
}
```

也可以在应用程序级别设置默认的内容编码，默认情况下使用`ContentEncoding::Auto`，这意味着自动进行内容压缩协商

```rust
use kayrx::web::{http::ContentEncoding, middleware, HttpResponse};

async fn index() -> HttpResponse {
    HttpResponse::Ok().body("data")
}

#[kayrx::main]
async fn main() -> std::io::Result<()> {
    use kayrx::web::{web, App, HttpServer};

    HttpServer::new(|| {
        App::new()
            .wrap(middleware::Compress::new(ContentEncoding::Br))
            .route("/", web::get().to(index))
    })
    .bind("127.0.0.1:8088")?
    .run()
    .await
}
```

## JSON响应

该`Json`类型允许使用格式正确的`JSON`数据进行响应：只需返回`Json<T>`类型的值，其中`T`是要序列化为`JSON`结构的类型。该类型`T`必须实现`serde`的`Serialize`特征

```rust
use kayrx::web::{web, HttpResponse, Result};
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize)]
struct MyObj {
    name: String,
}

async fn index(obj: web::Path<MyObj>) -> Result<HttpResponse> {
    Ok(HttpResponse::Ok().json(MyObj {
        name: obj.name.to_string(),
    }))
}

#[kayrx::main]
async fn main() -> std::io::Result<()> {
    use kayrx::web::{App, HttpServer};

    HttpServer::new(|| App::new().route(r"/a/{name}", web::get().to(index)))
        .bind("127.0.0.1:8088")?
        .run()
        .await
}
```