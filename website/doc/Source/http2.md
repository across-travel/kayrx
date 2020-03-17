# HTTP2

`kayrx::web` 如有可能, 自动将连接升级到HTTP / 2.0.

## TLS

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

> 请查看 [rustls](https://github.com/kayrx/awesome/tree/master/kayrx-examples/rustls)完整示例.