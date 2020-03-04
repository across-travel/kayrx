# 入门

让我们来编写我们的第一个`kayrx` `app`！

## Hello world

首先创建一个新的基于二进制的Cargo项目，然后切换到新目录：

```bash
cargo new hello-world
cd hello-world
```

现在, `kayrx`通过确保`Cargo.toml` 包含以下内容，将其添加为项目的依赖项：

```ini
[dependencies]
kayrx = "0.7"
```

为了实现Web服务器，我们首先需要创建一个请求处理程序.

请求处理程序是一个异步函数，它接受零个或多个可以从请求中提取的参数（即 `impl FromRequest`），并返回可以转换为`HttpResponse`的类型（即`impl Responder`）：

```rust
use kayrx::web::{web, App, HttpResponse, HttpServer, Responder};

async fn index() -> impl Responder {
    HttpResponse::Ok().body("Hello world!")
}

async fn index2() -> impl Responder {
    HttpResponse::Ok().body("Hello world again!")
}
```

接下来，创建一个`App`实例，并`path`路径上使用`app`的`route`和特定的`HTTP`方法注册请求处理程序 。之后`HttpServer`可以将`app`实例用于侦听传入的连接。服务器接受 函数 返回`app`工厂.

```rust
#[kayrx::main]
async fn main() -> std::io::Result<()> {
    HttpServer::new(|| {
        App::new()
            .route("/", web::get().to(index))
            .route("/again", web::get().to(index2))
    })
    .bind("127.0.0.1:8088")?
    .run()
    .await
}
```

现在，使用`cargo run`编译并运行程.。去`http://localhost:8088/` 看看结果。

**注意**：您可能会注意到`#[kayrx::main]`属性宏。该宏在`kayrx`运行时中执行异步标记的函数。任何异步函数都可以通过此宏进行标记和执行。

### 使用属性宏定义路由

您可以使用宏属性定义路由，这些宏属性允许您在函数上方指定路由，如下所示：

```rust
use kayrx::web::get;

#[get("/hello")]
async fn index3() -> impl Responder {
    HttpResponse::Ok().body("Hey there!")
}
```

然后，您可以使用来`service()`注册路由：

```rust
App::new()
    .service(index3)
```

出于一致性的原因，本文档仅使用本页开头显示的显式语法。但是，如果您更喜欢这种语法，那么在声明路由时就可以随意使用它，因为它只是语法糖.

要了解更多信息，请参见[kayrx-macro](https://docs.rs/kayrx-macro)

### 自动重载

如果需要，您可以在开发过程中拥有一个自动重载服务器，该服务器可以按需重新编译。这不是必需的，但是它使快速原型制作更加方便，因为您可以在保存时立即看到更改。要了解如何完成此任务，请看一下[autoreload模式](./autoreload.md).