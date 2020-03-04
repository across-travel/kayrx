# 请求处理

请求处理是一个异步函数，该函数接受零个或多个可以从请求中提取的参数（即[impl FromRequest](https://docs.rs/kayrx/0.7.5/kayrx/web/trait.FromRequest.html)），并返回可以转换为`HttpResponse`的类型（即[impl Responder](https://docs.rs/kayrx/0.7.5/kayrx/web/trait.Responder.html)）。

请求处理分为两个阶段。首先，调用处理程序对象，返回实现`Responder特质`的任何对象。然后，在返回的对象上调用`respond_to()`，将其自身转换为`HttpResponseor` 或 `Error`。

默认情况下`kayrx::web`提供了一些标准类型的`Responder`实现，诸如`&'static str`，`String`等

> 有关实现的完整列表，请参阅[Responder]()文档。

有效的处理程序示例：

```rust
async fn index(_req: HttpRequest) -> &'static str {
    "Hello world!"
}

async fn index2(_req: HttpRequest) -> String {
    "Hello world!".to_owned()
}
```

您还可以将返回签名更改为 `impl Responder`，如果涉及到更复杂的类型，效果很好

```rust
async fn index(_req: HttpRequest) -> impl Responder {
    Bytes::from_static(b"Hello world!")
}

async fn index(req: HttpRequest) -> Box<Future<Item=HttpResponse, Error=Error>> {
    ...
}
```

## 自定义响应的类型

要直接从处理函数返回自定义类型，该类型需要实现`Responder`特质.

让我们为序列化为`application/json`的响应创建一个自定义类型：

```rust
use kayrx::web::{Error, HttpRequest, HttpResponse, Responder};
use serde::Serialize;
use futures::future::{ready, Ready};

#[derive(Serialize)]
struct MyObj {
    name: &'static str,
}

// Responder
impl Responder for MyObj {
    type Error = Error;
    type Future = Ready<Result<HttpResponse, Error>>;

    fn respond_to(self, _req: &HttpRequest) -> Self::Future {
        let body = serde_json::to_string(&self).unwrap();

        // Create response and set content type
        ready(Ok(HttpResponse::Ok()
            .content_type("application/json")
            .body(body)))
    }
}

async fn index() -> impl Responder {
    MyObj { name: "user" }
}
```

## 流式响应主体

响应主体(Response body)可以异步生成。在这种情况下，`body`必须实现`stream` 特质 即： `Stream<Item=Bytes, Error=Error>`.

```rust
use kayrx::web::{web, App, HttpServer, Error, HttpResponse};
use bytes::Bytes;
use futures::stream::once;
use futures::future::ok;

async fn index() -> HttpResponse {
    let body = once(ok::<_, Error>(Bytes::from_static(b"test")));

    HttpResponse::Ok()
        .content_type("application/json")
        .streaming(body)
}

#[kayrx::main]
async fn main() -> std::io::Result<()> {
    HttpServer::new(|| App::new().route("/async", web::to(index)))
        .bind("127.0.0.1:8088")?
        .run()
        .await
}
```

## 返回不同类型 (Either)

有时，您需要返回不同类型的响应。例如，您可以进行错误检查并返回错误，返回异步响应或需要两种不同类型的任何结果。

在这种情况下，可以使用[Either](https://docs.rs/kayrx/0.7.5/kayrx/web/enum.Either.html)类型。 `Either`允许将两种不同的响应者类型组合为一个类型.

```rust
use kayrx::web::{Either, Error, HttpResponse};

type RegisterResult = Either<HttpResponse, Result<&'static str, Error>>;

fn index() -> RegisterResult {
    if is_a_variant() {
        // <- choose variant A
        Either::A(HttpResponse::BadRequest().body("Bad data"))
    } else {
        // <- variant B
        Either::B(Ok("Hello!"))
    }
}
```