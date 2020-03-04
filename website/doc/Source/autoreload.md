# 自动重载开发服务器

在开发过程中，让`cargo`在更改时自动重新编译代码会非常方便。这可以通过使用[cargo-watch](https://github.com/passcod/cargo-watch)来完成。由于应用程序通常会绑定到端口以侦听传入的`HTTP`请求，因此将其与[listenfd](https://crates.io/crates/listenfd)和[systemfd](https://github.com/mitsuhiko/systemfd) 实用程序结合使用以确保套接字在应用程序编译和重新加载时保持打开状态是有意义的。

`systemfd`将打开一个套接字并将其传递给`cargo-watch`，以监视更改，然后调用编译器并运行您的应用。然后应用程序将`listenfd`用于拾取`systemfd`打开的套接字.

## 必要的二进制文件

要获得自动重装，您需要安装`cargo-watch`和 `安装systemfd`。两者都是用`Rust`编写的，`cargo install`可以安装：

```rust
cargo install systemfd cargo-watch
```

## 代码变更

您需要稍微修改一下应用程序，以便它可以拾取由`systemfd`打开的外部套接字。将`listenfd`依赖项添加到您的应用程序：

```toml
[dependencies]
listenfd = "0.3"
```

然后修改服务器代码，使其仅作为`bind`后备调用：

```rust
use kayrx::web::{web, App, HttpRequest, HttpServer, Responder};
use listenfd::ListenFd;

async fn index(_req: HttpRequest) -> impl Responder {
    "Hello World!"
}

#[kayrx::main]
async fn main() -> std::io::Result<()> {
    let mut listenfd = ListenFd::from_env();
    let mut server = HttpServer::new(|| App::new().route("/", web::get().to(index)));

    server = if let Some(l) = listenfd.take_tcp_listener(0).unwrap() {
        server.listen(l)?
    } else {
        server.bind("127.0.0.1:3000")?
    };

    server.run().await
}
```

## 运行服务器

现在要运行开发服务器，请用以下命令：

```shell
systemfd --no-pid -s http::3000 -- cargo watch -x run
```
