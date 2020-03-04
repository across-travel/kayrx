# 请求

## 内容编码

`kayrx::web` 自动解压/缩内容负载。支持以下编/解码器：

* Brotli
* Chunked
* Compress
* Gzip
* Deflate
* Identity
* Trailers
* EncodingExt

如果请求标头包含`Content-Encoding`标头，则根据标头值对请求有效负载进行解压缩。不支持多个编/解码器，即：`Content-Encoding: br, gzip`.

## JSON请求

`json`正文反序列化有几种选择 :

第一种选择是使用`Json`提取器。首先，定义一个处理函数接受`Json<T>`作为参数，然后，使用该`.to()`方法注册该处理函数。也可以通过使用`serde_json::Valuetype` 作为类型`T`, 来接受任意有效的`json`对象.

```rust
use kayrx::web::{web, App, HttpServer, Result};
use serde::Deserialize;

#[derive(Deserialize)]
struct Info {
    username: String,
}

/// extract `Info` using serde
async fn index(info: web::Json<Info>) -> Result<String> {
    Ok(format!("Welcome {}!", info.username))
}

#[kayrx::main]
async fn main() -> std::io::Result<()> {
    HttpServer::new(|| App::new().route("/", web::post().to(index)))
        .bind("127.0.0.1:8088")?
        .run()
        .await
}
```

您也可以将有效负载(payload)手动加载到内存中，然后反序列化。

在下面的示例中，我们将反序列化`MyObj`结构。我们需要先加载请求主体，然后将`json`反序列化为一个对象。

```rust
use kayrx::web::{error, web, App, Error, HttpResponse};
use bytes::BytesMut;
use futures::StreamExt;
use serde::{Deserialize, Serialize};
use serde_json;

#[derive(Serialize, Deserialize)]
struct MyObj {
    name: String,
    number: i32,
}

const MAX_SIZE: usize = 262_144; // max payload size is 256k

async fn index_manual(mut payload: web::Payload) -> Result<HttpResponse, Error> {
    // payload is a stream of Bytes objects
    let mut body = BytesMut::new();
    while let Some(chunk) = payload.next().await {
        let chunk = chunk?;
        // limit max size of in-memory payload
        if (body.len() + chunk.len()) > MAX_SIZE {
            return Err(error::ErrorBadRequest("overflow"));
        }
        body.extend_from_slice(&chunk);
    }

    // body is loaded, now we can deserialize serde-json
    let obj = serde_json::from_slice::<MyObj>(&body)?;
    Ok(HttpResponse::Ok().json(obj)) // <- send response
}
```

> [示例](https://github.com/kayrx/awesome/tree/master/examples/json)中提供了这两个选项的完整示例.

## 分块传输编码

`kayrx::web`自动解码分块编码。该`web::Payload` 提取已经包含了解码的字节流。如果使用支持的压缩编解码器(br, gzip, deflate)之一压缩请求负载，则将字节流解压缩

## Multipart body

`kayrx::web`使用[keclc_multipart](https://github.com/kayrx/keclc/tree/master/keclc-multipart)提供`Multipart body`支持.

> 目录中提供了完整的[示例](https://github.com/kayrx/awesome/tree/master/examples/multipart)

## Urlencoded body

 `kayrx::web`为`application / x-www-form-urlencoded`编码内容提供支持, 通过`web::Form`提取器解析为反序列化实例. 实例的类型必须实现`serde`的`Deserialize`特征

该url编码将若干情况下可以解析成错误：

* 内容类型不是` application/x-www-form-urlencoded`
* 传输编码为`chunked`
* 内容长度大于256k
* 负载(payload)因错误而终止。

```rust
use kayrx::web::{web, HttpResponse};
use serde::Deserialize;

#[derive(Deserialize)]
struct FormData {
    username: String,
}

async fn index(form: web::Form<FormData>) -> HttpResponse {
    HttpResponse::Ok().body(format!("username: {}", form.username))
}
```

## 流请求

`HttpRequest`是Bytes对象流。它可用于读取请求主体负载.

在以下示例中，我们逐块读取并打印请求负载：

```rust
use kayrx::web::{web, Error, HttpResponse};
use futures::StreamExt;

async fn index(mut body: web::Payload) -> Result<HttpResponse, Error> {
    let mut bytes = web::BytesMut::new();
    while let Some(item) = body.next().await {
        bytes.extend_from_slice(&item?);
    }

    println!("Chunk: {:?}", bytes);
    Ok(HttpResponse::Ok().finish())
}
```