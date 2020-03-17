# Errors

`kayrx::web`使用其自身的`kayrx::web::error::Error`类型和 `kayrx::web::error::ResponseError`特征处理来自Web处理程序的错误. 

如果处理程序在返回`Result` 中的`Error`(指Rust `std::error::Error`特征) 也实现 `ResultResponseError`特征，`kayrx::web`将把该错误作为HTTP响应及其对应的`kayrx::web::http::StatusCode`呈现。默认情况下会生成内部服务器错误：

```rust
pub trait ResponseError {
    fn error_response(&self) -> HttpResponse;
    fn status_code(&self) -> StatusCode;
}
```

将兼容`Result`的`Responder`强制转换为HTTP响应：

```rust
impl<T: Responder, E: Into<Error>> Responder for Result<T, E>
```

上面的代码中的`Error`是`kayrx::web`的错误定义，可以将实现`ResponseError`的任何错误自动转换.。

kayrx-web提供了一些常见的非`kayrx::web`错误的`ResponseError`实现。例如，如果处理程序以`io::Error`响应，则该错误将转换为`HttpInternalServerError`：

```rust
use std::io;

fn index(_req: HttpRequest) -> io::Result<fs::NamedFile> {
    Ok(fs::NamedFile::open("static/index.html")?)
}
```

有关[`ResponseError`的外部实现](https://docs.rs/kayrx/0.7.5/kayrx/web/error/trait.ResponseError.html#foreign-impls)完整列表.

## 自定义错误响应

这是一个实现`ResponseError`示例：

```rust
use kayrx::web::{error, Result};
use failure::Fail;

#[derive(Fail, Debug)]
#[fail(display = "my error")]
struct MyError {
    name: &'static str,
}

// Use default implementation for `error_response()` method
impl error::ResponseError for MyError {}

async fn index() -> Result<&'static str, MyError> {
    Err(MyError { name: "test" })
}
```

`ResponseError`有一个`error_response()`的默认实现，它将呈现 500 (内部服务器错误)，这就是在上面执行`index`处理程序时将发生的情况 .

覆盖`error_response()`以产生更有用的结果：

```rust
use kayrx::http::ResponseBuilder;
use kayrx::web::{error, http::header, http::StatusCode, HttpResponse};
use failure::Fail;

#[derive(Fail, Debug)]
enum MyError {
    #[fail(display = "internal error")]
    InternalError,
    #[fail(display = "bad request")]
    BadClientData,
    #[fail(display = "timeout")]
    Timeout,
}

impl error::ResponseError for MyError {
    fn error_response(&self) -> HttpResponse {
        ResponseBuilder::new(self.status_code())
            .set_header(header::CONTENT_TYPE, "text/html; charset=utf-8")
            .body(self.to_string())
    }

    fn status_code(&self) -> StatusCode {
        match *self {
            MyError::InternalError => StatusCode::INTERNAL_SERVER_ERROR,
            MyError::BadClientData => StatusCode::BAD_REQUEST,
            MyError::Timeout => StatusCode::GATEWAY_TIMEOUT,
        }
    }
}

async fn index() -> Result<&'static str, MyError> {
    Err(MyError::BadClientData)
}
```

## 错误助手

`kayrx::web`提供了一组错误帮助函数，这些函数可用于从其他错误中生成特定的HTTP错误码。在这里，我们使用`map_err`将未实现`ResponseError`特征的`MyError`转换为 400（错误请求）：

```rust
use kayrx::web::{error, Result};

#[derive(Debug)]
struct MyError {
    name: &'static str,
}

async fn index() -> Result<&'static str> {
    let result: Result<&'static str, MyError> = Err(MyError { name: "test error" });

    Ok(result.map_err(|e| error::ErrorBadRequest(e.name))?)
}
```

有关 可用的错误帮助程序的完整列表，请参见`error`模块的[API文档](https://docs.rs/kayrx/0.7.5/kayrx/web/error/struct.Error.html).

## 错误日志

`kayrx::web`在`WARN`日志级别记录所有错误。如果将应用程序的日志级别设置为`DEBUG`并启用`RUST_BACKTRACE`，则还将记录回溯。这些可以使用环境变量进行配置：

```rust
>> RUST_BACKTRACE=1 RUST_LOG=kayrx::web=debug cargo run
```

该`Error`类型使用原因的错误回溯(如果可用). 如果故障根本不提供回溯，则构造一个新的回溯，指向发生转换的点(而不是错误的根源).

考虑将应用程序产生的错误分为两大类可能是有用的：一类是面向用户的，二类不是面向用户的.

这是使用的`middleware::Logger`基本示例：

```rust
use kayrx::web::{error, Result};
use failure::Fail;
use log::debug;

#[derive(Fail, Debug)]
#[fail(display = "my error")]
pub struct MyError {
    name: &'static str,
}

// Use default implementation for `error_response()` method
impl error::ResponseError for MyError {}

async fn index() -> Result<&'static str, MyError> {
    let err = MyError { name: "test error" };
    debug!("{}", err);
    Err(err)
}

#[kayrx::main]
async fn main() -> std::io::Result<()> {
    use kayrx::web::{middleware::Logger, web, App, HttpServer};

    std::env::set_var("RUST_LOG", "my_errors=debug,kayrx::web=info");
    std::env::set_var("RUST_BACKTRACE", "1");
    env_logger::init();

    HttpServer::new(|| {
        App::new()
            .wrap(Logger::default())
            .route("/", web::get().to(index))
    })
    .bind("127.0.0.1:8088")?
    .run()
    .await
}
```
