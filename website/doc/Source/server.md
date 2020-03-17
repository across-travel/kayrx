# HTTP服务器

[**HttpServer**](https://docs.rs/kayrx/0.7.5/kayrx/web/struct.HttpServer.html)类型负责服务HTTP请求

`HttpServer`接受应用工厂(app factory)作为参数，并且应用工厂必须具有`Send+ Sync`约束。有关更多信息，请参见*多线程*部分

要绑定到特定的套接字地址，必须使用`bind()`，并且可以多次调用它。要绑定`SSL`套接字，应该使用[bind_rustls()](https://docs.rs/kayrx/0.7.5/kayrx/web/struct.HttpServer.html#method.bind_rustls)。要运行http服务器，请使用`HttpServer::run()` 方法

```rust
use kayrx::web::{web, App, HttpResponse, HttpServer};

#[kayrx::main]
async fn main() -> std::io::Result<()> {
    HttpServer::new(|| {
        App::new().route("/", web::get().to(|| HttpResponse::Ok()))
    })
    .bind("127.0.0.1:8088")?
    .run()
    .await
}
```

`run()`方法返回[Server](https://docs.rs/kayrx/0.7.5/kayrx/web/dev/struct.Server.html)类型的实例。Server类型的方法可用于管理http服务器

- `pause()` -暂停接受传入的连接
- `resume()` -恢复接受传入的连接
- `stop()` -停止传入连接处理，停止所有工作程序并退出
以下示例显示如何在单独的线程中启动http服务器。

```rust
use kayrx::fiber::System;
use kayrx::web::{web, App, HttpResponse, HttpServer};
use std::sync::mpsc;
use std::thread;

#[kayrx::main]
async fn main() {
    let (tx, rx) = mpsc::channel();

    thread::spawn(move || {
        let sys = System::new("http-server");

        let srv = HttpServer::new(|| {
            App::new().route("/", web::get().to(|| HttpResponse::Ok()))
        })
        .bind("127.0.0.1:8088")?
        .shutdown_timeout(60) // <- Set shutdown timeout to 60 seconds
        .run();

        let _ = tx.send(srv);
        sys.run()
    });

    let srv = rx.recv().unwrap();

    // pause accepting new connections
    srv.pause().await;
    // resume accepting new connections
    srv.resume().await;
    // stop server
    srv.stop(true).await;
}
```

## 多线程

`HttpServer`自动启动一些`http worker`，默认情况下，该数量等于系统中逻辑CPU的数量。该数字可以用该[HttpServer::workers()](https://docs.rs/kayrx/0.7.5/kayrx/web/struct.HttpServer.html#method.workers)方法覆盖

```rust
use kayrx::web::{web, App, HttpResponse, HttpServer};

#[kayrx::main]
async fn main() {
    HttpServer::new(|| {
        App::new().route("/", web::get().to(|| HttpResponse::Ok()))
    })
    .workers(4); // <- Start 4 workers
}
```

工作线程创建后，他们每个都会接受一个单独的应用实例来处理请求。线程之间不共享应用状态，并且处理程序可以自由地操作其状态副本，而无需担心并发问题。

应用状态不必为`Send或Sync`，但应用工厂必须为`Send+ Sync`。

要在工作线程之间共享状态，请使用`Arc`。引入共享和同步后，应格外小心。在许多情况下，由于锁定共享状态以进行修改而无意中引入了性能成本。

在某些情况下，可以使用更有效的锁定策略来减轻这些成本，例如，使用读/写锁而不是 互斥锁来实现非排他性锁定，但是性能最高的实现通常往往是根本不使用锁的实现。

由于每个工作线程都按顺序处理其请求，因此阻塞当前线程处理程序将导致当前工作线程停止处理新请求：

```rust
fn my_handler() -> impl Responder {
    std::thread::sleep(Duration::from_secs(5)); // <-- Bad practice! Will cause the current worker thread to hang!
    "response"
}
```

因此，任何长时间的，非cpu绑定的操作（例如，I / O，数据库操作等）都应表示为`Future`或异步函数。异步处理由工作线程并发执行，因此不会阻塞执行：

```rust
async fn my_handler() -> impl Responder {
    kayrx::timer::delay_for(Duration::from_secs(5)).await; // <-- Ok. Worker thread will handle other requests here
    "response"
}
```

同样的限制也适用于提取器。当处理函数收到实现`FromRequest`的参数，并且该实现阻塞当前线程时，工作线程将在运行处理程序时阻塞。为此，在实现提取器时必须特别注意，并且还应在需要时异步实现它们

## SSL

`SSL`服务器：`rustls` , 该 rustls 适用于 `rustls`集成

```toml
[dependencies]
kayrx = "0.7.5"
rustls = "0.16"
```

```rust
use std::fs::File;
use std::io::BufReader;
use kayrx::web::{web, App, HttpRequest, HttpServer, Responder};
use rustls::internal::pemfile::{certs, rsa_private_keys};
use rustls::{NoClientAuth, ServerConfig};

async fn index(_req: HttpRequest) -> impl Responder {
    "Welcome!"
}

#[kayrx::main]
async fn main() -> std::io::Result<()> {
    // load ssl keys

    let mut config = ServerConfig::new(NoClientAuth::new());
    let cert_file = &mut BufReader::new(File::open("cert.pem").unwrap());
    let key_file = &mut BufReader::new(File::open("key.pem").unwrap());
    let cert_chain = certs(cert_file).unwrap();
    let mut keys = rsa_private_keys(key_file).unwrap();
    config.set_single_cert(cert_chain, keys.remove(0)).unwrap();

    HttpServer::new(|| App::new().route("/", web::get().to(index)))
        .bind_rustls("127.0.0.1:8088", builder)?
        .run()
        .await
}
```

> 完整示例[rustls](https://github.com/kayrx/awesome/tree/master/kayrx-examples/rustls).

## Keep-Alive

kayrx::web 可以在保持活动连接上等待请求.

> *keep alive* 保持活动连接行为由服务器设置定义.

- `75`, `Some(75)`, `KeepAlive::Timeout(75)` - enable 75 second *keep alive* timer.
- `None` or `KeepAlive::Disabled` - disable *keep alive*.
- `KeepAlive::Tcp(75)` - use `SO_KEEPALIVE` socket option.

```rust
use kayrx::web::{web, App, HttpResponse, HttpServer};

#[kayrx::main]
async fn main() -> std::io::Result<()> {
    let one = HttpServer::new(|| {
        App::new().route("/", web::get().to(|| HttpResponse::Ok()))
    })
    .keep_alive(75); // <- Set keep-alive to 75 seconds

    // let _two = HttpServer::new(|| {
    //     App::new().route("/", web::get().to(|| HttpResponse::Ok()))
    // })
    // .keep_alive(); // <- Use `SO_KEEPALIVE` socket option.

    let _three = HttpServer::new(|| {
        App::new().route("/", web::get().to(|| HttpResponse::Ok()))
    })
    .keep_alive(None); // <- Disable keep-alive

    one.bind("127.0.0.1:8088")?.run().await
}
```

如果选择了上面的第一个选项，则根据响应的`connection-type`计算*keep alive*状态。默认情况下未定义。在这种情况下，*keep alive*是由请求的 http版本定义的

> *keep alive* is **off** for *HTTP/1.0* and is **on** for *HTTP/1.1* and *HTTP/2.0*.

可以通过`HttpResponseBuilder::connection_type()`方法更改*Connection type*.

```rust
use kayrx::web::{http, HttpRequest, HttpResponse};

async fn index(req: HttpRequest) -> HttpResponse {
    HttpResponse::Ok()
        .connection_type(http::ConnectionType::Close) // <- Close connection
        .force_close() // <- Alternative method
        .finish()
}
```

## 优雅关机

`HttpServer`支持优雅关机。收到停止信号后，工作线程将有特定的时间来完成服务请求。超时后仍然存活的所有工人(工作线程)都被强制撤职。默认情况下，关闭超时设置为30秒。您可以使用`HttpServer::shutdown_timeout()` 方法更改此参数。

您可以使用服务器地址向服务器发送停止消息，并指定是否要正常关机。该`start()`方法返回服务器的地址。

`HttpServer`处理几个操作系统信号。`CTRL-C`在所有操作系统上均可用，其他信号在`UNIX`系统上均可用。

- SIGINT-强制关机
- SIGTERM-正常关闭的工人
- SIGQUIT-强制关机工人

可以使用`HttpServer::disable_signals()`方法禁用信号处理