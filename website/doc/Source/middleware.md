# 中间件

`kayrx::web`的中间件系统允许我们向请求/响应处理添加其他行为。中间件可以挂接到传入的请求流程中，使我们能够修改请求以及暂停请求处理以尽早返回响应.

中间件也可以加入响应处理. 通常，中间件涉及以下操作：

* 预处理请求
* 后处理响应
* 修改应用程序状态
* 访问外部服务(redis, 日志记录, 会话)

中间件被注入每个`App`,`scope`或`Resource`, 并按照相反的顺序执行.  通常，中间件是一种实现 `Service` trait和`Transform` trait的类型。特征中的每个方法都有一个默认实现。每个方法都可以立即返回结果或`future`的对象。

以下演示了如何创建一个简单的中间件：

```rust
use std::pin::Pin;
use std::task::{Context, Poll};

use kayrx::service::{Service, Transform};
use kayrx::web::{dev::ServiceRequest, dev::ServiceResponse, Error};
use futures::future::{ok, Ready};
use futures::Future;

// There are two steps in middleware processing.
// 1. Middleware initialization, middleware factory gets called with
//    next service in chain as parameter.
// 2. Middleware's call method gets called with normal request.
pub struct SayHi;

// Middleware factory is `Transform` trait from kayrx::service crate
// `S` - type of the next service
// `B` - type of response's body
impl<S, B> Transform<S> for SayHi
where
    S: Service<Request = ServiceRequest, Response = ServiceResponse<B>, Error = Error>,
    S::Future: 'static,
    B: 'static,
{
    type Request = ServiceRequest;
    type Response = ServiceResponse<B>;
    type Error = Error;
    type InitError = ();
    type Transform = SayHiMiddleware<S>;
    type Future = Ready<Result<Self::Transform, Self::InitError>>;

    fn new_transform(&self, service: S) -> Self::Future {
        ok(SayHiMiddleware { service })
    }
}

pub struct SayHiMiddleware<S> {
    service: S,
}

impl<S, B> Service for SayHiMiddleware<S>
where
    S: Service<Request = ServiceRequest, Response = ServiceResponse<B>, Error = Error>,
    S::Future: 'static,
    B: 'static,
{
    type Request = ServiceRequest;
    type Response = ServiceResponse<B>;
    type Error = Error;
    type Future = Pin<Box<dyn Future<Output = Result<Self::Response, Self::Error>>>>;

    fn poll_ready(&mut self, cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        self.service.poll_ready(cx)
    }

    fn call(&mut self, req: ServiceRequest) -> Self::Future {
        println!("Hi from start. You requested: {}", req.path());

        let fut = self.service.call(req);

        Box::pin(async move {
            let res = fut.await?;

            println!("Hi from response");
            Ok(res)
        })
    }
}
```

另外，对于简单的用例，可以使用[wrap_fn](https://docs.rs/kayrx/0.7.5/kayrx/web/struct.App.html#method.wrap_fn)创建小型的临时中间件：

```rust
use kayrx::service::Service;
use kayrx::web::{web, App};
use futures::future::FutureExt;

#[kayrx::main]
async fn main() {
    let app = App::new()
        .wrap_fn(|req, srv| {
            println!("Hi from start. You requested: {}", req.path());
            srv.call(req).map(|res| {
                println!("Hi from response");
                res
            })
        })
        .route(
            "/index.html",
            web::get().to(|| async {
                "Hello, middleware!"
            }),
        );
}
```

> `kayrx::web`提供了几种有用的中间件，例如 *logging*, *cors*, *compress* .

## 日志

日志记录是作为中间件实现的。通常将日志记录中间件注册为该应用程序的第一个中间件。记录中间件必须为每个应用注册.

该`Logger`中间件使用标准`log` crate 记录信息。您应该为`kayrx`软件包启用记录器以查看访问日志(`env_logger`或类似的)

Create `Logger` middleware with the specified `format`.  Default `Logger` can be created
with `default` method, it uses the default format:

`Logger`使用指定的格式创建中间件, 可以使用`defaul`方法 创建`Logger`默认值，它使用默认格式：

```ignore
  %a %t "%r" %s %b "%{Referer}i" "%{User-Agent}i" %T
```

```rust
use kayrx::web::middleware::Logger;
use env_logger;

#[kayrx::main]
async fn main() -> std::io::Result<()> {
    use kayrx::web::{App, HttpServer};

    std::env::set_var("RUST_LOG", "kayrx::web=info");
    env_logger::init();

    HttpServer::new(|| {
        App::new()
            .wrap(Logger::default())
            .wrap(Logger::new("%a %{User-Agent}i"))
    })
    .bind("127.0.0.1:8088")?
    .run()
    .await
}
```

以下是默认日志记录格式的示例：

```rust
INFO:kayrx::web::middleware::logger: 127.0.0.1:59934 [02/Dec/2017:00:21:43 -0800] "GET / HTTP/1.1" 302 0 "-" "curl/7.54.0" 0.000397
INFO:kayrx::web::middleware::logger: 127.0.0.1:59947 [02/Dec/2017:00:22:40 -0800] "GET /index.html HTTP/1.1" 200 0 "-" "Mozilla/5.0 
(Macintosh; Intel Mac OS X 10.13; rv:57.0) Gecko/20100101 Firefox/57.0" 0.000646
```

## 格式

- `%%`   百分号
- `%a`  远程IP地址（如果使用反向代理，则为代理的IP地址）
- `%t`  请求开始处理的时间
- `%P`  为请求提供服务的子进程ID
- `%r`  第一行要求
- `%s`  响应状态码
- `%b`  响应大小（以字节为单位），包括HTTP标头
- `%T`  服务请求所用的时间，以秒为单位，浮动分数为.06f格式
- `%D`  服务请求所花费的时间（以毫秒为单位）
- `%{FOO}i`  请求头['FOO']
- `%{FOO}o`  响应头['FOO']
- `%{FOO}e`  os.environ['FOO']

## 默认标头

要设置默认响应头，可以使用`DefaultHeaders`中间件. 如果响应头已经包含指定的报头,所述 `DefaultHeaders`中间件不设置标题

```rust
use kayrx::web::{http, middleware, HttpResponse};

#[kayrx::main]
async fn main() -> std::io::Result<()> {
    use kayrx::web::{web, App, HttpServer};

    HttpServer::new(|| {
        App::new()
            .wrap(middleware::DefaultHeaders::new().header("X-Version", "0.2"))
            .service(
                web::resource("/test")
                    .route(web::get().to(|| HttpResponse::Ok()))
                    .route(
                        web::method(http::Method::HEAD)
                            .to(|| HttpResponse::MethodNotAllowed()),
                    ),
            )
    })
    .bind("127.0.0.1:8088")?
    .run()
    .await
}
```

## 错误处理

`ErrorHandlers` 中间件使我们能够提供自定义的响应处理.

您可以使用该`ErrorHandlers::handler()`方法为特定状态代码注册自定义错误处理, 您可以修改现有响应或创建一个全新的响应, 错误处理程序可以立即返回响应，也可以返回解析为`Future`的响应. 

```rust
use kayrx::web::middleware::errhandlers::{ErrorHandlerResponse, ErrorHandlers};
use kayrx::web::{dev, http, HttpResponse, Result};

fn render_500<B>(mut res: dev::ServiceResponse<B>) -> Result<ErrorHandlerResponse<B>> {
    res.response_mut().headers_mut().insert(
        http::header::CONTENT_TYPE,
        http::HeaderValue::from_static("Error"),
    );
    Ok(ErrorHandlerResponse::Response(res))
}

#[kayrx::main]
async fn main() -> std::io::Result<()> {
    use kayrx::web::{web, App, HttpServer};

    HttpServer::new(|| {
        App::new()
            .wrap(
                ErrorHandlers::new()
                    .handler(http::StatusCode::INTERNAL_SERVER_ERROR, render_500),
            )
            .service(
                web::resource("/test")
                    .route(web::get().to(|| HttpResponse::Ok()))
                    .route(web::head().to(|| HttpResponse::MethodNotAllowed())),
            )
    })
    .bind("127.0.0.1:8088")?
    .run()
    .await
}
```
