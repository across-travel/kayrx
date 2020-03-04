# 编写应用

`kayrx` 提供了用Rust构建Web服务器和应用程序的各种基础类型。它提供了 路由，中间件，`http`请求的预处理，响应的后处理，`websocket`协议处理，`multipart`流，等等.

所有`kayrx`服务器都是围绕该`App`实例构建的。它用于为资源和中间件,注册路由。它还存储同一应用程序中所有处理程序之间共享的应用程序状态.

应用程序充当所有路由的命名空间，即特定应用程序的所有路由具有相同的url路径前缀。应用程序前缀总是包含一个前导的“/”斜杠。如果提供的前缀不包含前导斜杠，则会自动插入。前缀应该由路径值组成.

对于具有前缀`/app`的应用程序，与任何请求路径中有`/app`，`/app/`或`/app/test`匹配;  然而,路径`/application`不匹配。

```rust
use kayrx::web::{web, App, Responder, HttpServer};

async fn index() -> impl Responder {
    "Hello world!"
}

#[kayrx::main]
async fn main() -> std::io::Result<()> {
    HttpServer::new(|| {
        App::new().service(
            web::scope("/app").route("/index.html", web::get().to(index)),
        )
    })
    .bind("127.0.0.1:8088")?
    .run()
    .await
}
```

在此示例中，将创建具有`/app`前缀和`index.html`资源的应用。该资源可通过`/app/index.html`路径获得。

>有关更多信息，请查看[URL Dispatch](../advance/url-dispatch)部分。

## 状态

应用程序状态在同一范围内的所有路由和资源共享。状态可以使用`web::Data<T>`提取器访问，其中T是状态的类型。状态也可用于中间件.

让我们编写一个简单的应用程序并将应用程序名称存储在状态中：

```rust
use kayrx::web::{web, App, HttpServer};
use std::sync::Mutex;

// This struct represents state
struct AppState {
    app_name: String,
}

async fn index(data: web::Data<AppState>) -> String {
    let app_name = &data.app_name; // <- get app_name

    format!("Hello {}!", app_name) // <- response with app_name
}
```

并在初始化应用程序时传递状态，然后启动应用程序：

```rust
#[kayrx::main]
async fn main() -> std::io::Result<()> {
    HttpServer::new(|| {
        App::new()
            .data(AppState {
                app_name: String::from("kayrx web"),
            })
            .route("/", web::get().to(index))
    })
    .bind("127.0.0.1:8088")?
    .run()
    .await
}
```

可以在应用程序中注册任意数量的状态类型

## 共享可变状态

`HttpServer`接受应用工厂而不是应用实例。Http服务器为每个线程构造一个应用实例，因此必须多次构造应用数据。如果要在不同线程之间共享数据，则应使用可共享对象，例如“Send + Sync”。

内部`web::Data`使用`Arc`。因此，为了避免产生双` Arc`，我们应该在使用`App::app_data()`进行注册之前创建数据.

在下面的示例中，我们将编写一个具有可变共享状态的应用程序。首先，我们定义状态并创建处理程序：

```rust
struct AppStateWithCounter {
    counter: Mutex<i32>, // <- Mutex is necessary to mutate safely across threads
}

async fn _index(data: web::Data<AppStateWithCounter>) -> String {
    let mut counter = data.counter.lock().unwrap(); // <- get counter's MutexGuard
    *counter += 1; // <- access counter inside MutexGuard

    format!("Request number: {}", counter) // <- response with count
}
```

并在应用中注册数据：

```rust
#[kayrx::main]
async fn _main() -> std::io::Result<()> {
    let counter = web::Data::new(AppStateWithCounter {
        counter: Mutex::new(0),
    });

    HttpServer::new(move || {
        // move counter into the closure
        App::new()
            .app_data(counter.clone()) // <- register the created data
            .route("/", web::get().to(_index))
    })
    .bind("127.0.0.1:8088")?
    .run()
    .await
}
```

## 使用应用`Scope`来编写应用

该`web::scope()`方法允许设置特定的应用前缀。此范围(Scope)表示资源前缀，该前缀将附加到资源配置添加的所有资源模式中。这可用于帮助将一组路由安装在与所包含的可调用者不同的位置，同时仍保持相同的资源名称。

例如：

```rust
#[kayrx::main]
async fn main() {
    App::new()
        .service(
            web::scope("/users")
                .route("/show", web::get().to(show_users)));
}
```

在上面的示例中，`show_users`路由将具有`/ users / show`而不是`/ show`的有效路由模式， 因为应用的`scope`参数将被添加到该模式之前。然后，仅当URL路径为`/users /show`时，路由才会匹配，并且`HttpRequest.url_for()`使用`show_users`路由名称调用该函数时，它将生成具有相同路径的URL

## 应用防护和虚拟主机

您可以将`Guard`看作一个简单的函数，它接受请求对象引用并返回`true`或`false`。形式上，守卫是实现该`Guard`特征的任何对象 。`kayrx`提供了几种保护措施，您可以检查 `api`文档的[函数部分](https://docs.rs/kayrx/0.7.5/kayrx/web/guard/index.html#functions)。

提供的防护之一是[`Header`](https://docs.rs/kayrx/0.7.5/kayrx/web/guard/fn.Header.html)，它可以根据请求的标头信息用作应用的过滤器。

```rust
#[kayrx::main]
async fn main() -> std::io::Result<()> {
    HttpServer::new(|| {
        App::new()
            .service(
                web::scope("/")
                    .guard(guard::Header("Host", "www.rust-lang.org"))
                    .route("", web::to(|| HttpResponse::Ok().body("www"))),
            )
            .service(
                web::scope("/")
                    .guard(guard::Header("Host", "users.rust-lang.org"))
                    .route("", web::to(|| HttpResponse::Ok().body("user"))),
            )
            .route("/", web::to(|| HttpResponse::Ok()))
    })
    .bind("127.0.0.1:8088")?
    .run()
    .await
}
```

## 配置

为了简单和可重用性，`App`和`web::Scope`提供了该`configure`方法。此函数对于将配置的部分移动到其他模块甚至库中很有用。例如，某些资源的配置可以移到其他模块

```rust
use kayrx::web::{web, App, HttpResponse, HttpServer};

// this function could be located in different module
fn scoped_config(cfg: &mut web::ServiceConfig) {
    cfg.service(
        web::resource("/test")
            .route(web::get().to(|| HttpResponse::Ok().body("test")))
            .route(web::head().to(|| HttpResponse::MethodNotAllowed())),
    );
}

// this function could be located in different module
fn config(cfg: &mut web::ServiceConfig) {
    cfg.service(
        web::resource("/app")
            .route(web::get().to(|| HttpResponse::Ok().body("app")))
            .route(web::head().to(|| HttpResponse::MethodNotAllowed())),
    );
}

#[kayrx::main]
async fn main() -> std::io::Result<()> {
    HttpServer::new(|| {
        App::new()
            .configure(config)
            .service(web::scope("/api").configure(scoped_config))
            .route("/", web::get().to(|| HttpResponse::Ok().body("/")))
    })
    .bind("127.0.0.1:8088")?
    .run()
    .await
}
```

上面示例的结果将是：

```shell
/         -> "/"
/app      -> "app"
/api/test -> "test"
```

每个`ServiceConfig`可以有它自己的`data`，`routes`和`services`

也可以在单独的函数中创建应用对象。 `App`类型使用复杂的泛型，`result`类型必须使用`impl Trait`功能。这对于单元测试很有用

```rust
use kayrx::service::ServiceFactory;
use kayrx::web::dev::{MessageBody, ServiceRequest, ServiceResponse};
use kayrx::web::{web, App, Error, HttpResponse};

fn create_app() -> App<
    impl ServiceFactory<
        Config = (),
        Request = ServiceRequest,
        Response = ServiceResponse<impl MessageBody>,
        Error = Error,
    >,
    impl MessageBody,
> {
    App::new().service(
        web::scope("/app")
            .route("/index.html", web::get().to(|| HttpResponse::Ok())),
    )
}
```