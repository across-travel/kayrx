<div align="center">
 
 <span><img src="https://avatars0.githubusercontent.com/u/58360786?s=200&v=4" width="111" /></span>
  <p><h2>The Kayrx Framework</h2> </p>
  
  <p>

![linux](https://github.com/kayrx/kayrx/workflows/linux/badge.svg?branch=master)
[![Documentation](https://docs.rs/kayrx/badge.svg)](https://docs.rs/kayrx)
[![Download](https://img.shields.io/crates/d/kayrx.svg)](https://crates.io/crates/kayrx)
![License](https://img.shields.io/crates/l/kayrx.svg)

  </p>

  <h3>
    <a href="https://kayrx.github.io/kayrx">Website</a>
    <span> | </span>
    <a href="https://spectrum.chat/kayrx/kayrx">Chat</a>
    <span> | </span>
    <a href="https://github.com/kayrx/awesome/tree/master/examples">Examples</a>
  </h3>
</div>
<br>

**Kayrx** : Event-driven, Non-blocking I/O , Net, HTTP,  Web platform for writing **Asynchronous** apps with Rust.

## Info

> **Kayrx 诞生于 Actix-\* 和 Tokio** 可作为定制版(兼容Actix-web 2.0 - 查看 [Examples](https://github.com/kayrx/awesome/tree/master/examples))

## Features

* Async runtime with Fiber.
* Multi-thread Server.
* IO, FS, Net (Tcp, Udp, Uds)
* Sync primitive and Channel
* Timer :  timeouts, delays, and intervals.
* Codec : Decode, Encode, Framed
* Supported HTTP/1.x and HTTP/2.0 protocols
* Streaming and pipelining
* Keep-alive and slow requests handling
* Server/Client WebSockets support
* Transparent content compression/decompression (br, gzip, deflate)
* Configurable request router
* Multipart streams
* Static assets
* SSL support with  Rustls
* Middlewares (Logger, CORS, etc)
* Asynchronous HTTP client

## Ecosystem Component Librarys

- [keclc-file](https://github.com/kayrx/keclc/tree/master/keclc-file)
- [keclc-framed	](https://github.com/kayrx/keclc/tree/master/keclc-framed)
- [keclc-httpauth](https://github.com/kayrx/keclc/tree/master/keclc-httpauth)
- [keclc-ioframe](https://github.com/kayrx/keclc/tree/master/keclc-ioframe)
- [keclc-multipart](https://github.com/kayrx/keclc/tree/master/keclc-multipart)

[And More](https://github.com/kayrx/keclc)

## Example

Dependencies:

```toml
[dependencies]
kayrx = "0.7"
```

Code:

```rust
#[macro_use]
extern crate kayrx;

use kayrx::web::{web, App, HttpServer, Responder};

#[get("/{id}/{name}/index.html")]
async fn index(info: web::Path<(u32, String)>) -> impl Responder {
    format!("Hello {}! id:{}", info.1, info.0)
}

#[kayrx::main]
async fn main() -> std::io::Result<()> {
    HttpServer::new(|| App::new().service(index))
        .bind("127.0.0.1:8080")?
        .run()
        .await
}
```

查看更多 [**Examples**](https://github.com/kayrx/awesome/tree/master/examples)

## License

MIT license (LICENSE-MIT or http://opensource.org/licenses/MIT)
