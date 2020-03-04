# 测试

每个应用程序都应该经过良好的测试。`kayrx::web`提供了执行单元和集成测试的工具.

## 单元测试

对于单元测试，`kayrx::web`提供了请求构建器类型。 `TestRequest`实现类似构建器的模式, 您可以使用生成 `HttpRequest`实例带`to_http_request()`, 并使用处理函数调用它.

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use kayrx::web::test;

    #[kayrx::test]
    async fn test_index_ok() {
        let req = test::TestRequest::with_header("content-type", "text/plain").to_http_request();
        let resp = index(req).await;
        assert_eq!(resp.status(), http::StatusCode::OK);
    }

    #[kayrx::test]
    async fn test_index_not_ok() {
        let req = test::TestRequest::default().to_http_request();
        let resp = index(req).await;
        assert_eq!(resp.status(), http::StatusCode::BAD_REQUEST);
    }
}
```

## 集成测试

有几种方法可以测试您的应用程序。`kayrx::web`可用于在实际的`http`服务器中使用特定的处理程序来运行应用.

`TestRequest::get()`, `TestRequest::post()`以及其他方法可用于将请求发送到测试服务器。

要创建测试用的`Service`，请使用`test::init_service`接受常规`App`构建器方法。

查看[api文档](https://docs.rs/kayrx/0.7.5/kayrx/web/test/index.html)以获取更多信息. 

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use kayrx::web::{test, web, App};

    #[kayrx::test]
    async fn test_index_get() {
        let mut app = test::init_service(App::new().route("/", web::get().to(index))).await;
        let req = test::TestRequest::with_header("content-type", "text/plain").to_request();
        let resp = test::call_service(&mut app, req).await;
        assert!(resp.status().is_success());
    }

    #[kayrx::test]
    async fn test_index_post() {
        let mut app = test::init_service(App::new().route("/", web::get().to(index))).await;
        let req = test::TestRequest::post().uri("/").to_request();
        let resp = test::call_service(&mut app, req).await;
        assert!(resp.status().is_client_error());
    }
}
```

如果需要更复杂的应用程序，则配置测试应与创建普通应用程序非常相似。例如，您可能需要初始化应用程序状态。就像使用普通应用程序一样，使用`App`的 `data`方法创建一个 并附加状态

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use kayrx::web::{test, web, App};

    #[kayrx::test]
    async fn test_index_get() {
        let mut app = test::init_service(
            App::new()
                .data(AppState { count: 4 })
                .route("/", web::get().to(index)),
        ).await;
        let req = test::TestRequest::get().uri("/").to_request();
        let resp: AppState = test::read_response_json(&mut app, req).await;

        assert_eq!(resp.count, 4);
    }
}
```

## 流响应测试

如果您需要测试流，则只需调用`take_body()`并将结果`ResponseBody`转换 为`future`并执行它就足够了. 例如，测试服务器发送事件(SSE)

```rust
use std::task::Poll;
use bytes::Bytes;
use futures::stream::poll_fn;

use kayrx::http::{self, ContentEncoding, StatusCode};
use kayrx::web::{web, App, Error, HttpRequest, HttpResponse};

async fn sse(_req: HttpRequest) -> HttpResponse {
    let mut counter: usize = 5;

    // yields `data: N` where N in [5; 1]
    let server_events = poll_fn(move |_cx| -> Poll<Option<Result<Bytes, Error>>> {
        if counter == 0 {
            return Poll::Ready(None);
        }
        let payload = format!("data: {}\n\n", counter);
        counter -= 1;
        Poll::Ready(Some(Ok(Bytes::from(payload))))
    });

    HttpResponse::build(StatusCode::OK)
        .set_header(http::header::CONTENT_TYPE, "text/event-stream")
        .set_header(
            http::header::CONTENT_ENCODING,
            ContentEncoding::Identity.as_str(),
        )
        .streaming(server_events)
}

pub fn main() {
    App::new().route("/", web::get().to(sse));
}

#[cfg(test)]
mod tests {
    use super::*;
    use kayrx;

    use futures_util::stream::StreamExt;
    use futures_util::stream::TryStreamExt;

    use kayrx::web::{test, web, App};

    #[kayrx::test]
    async fn test_stream() {
        let mut app = test::init_service(App::new().route("/", web::get().to(sse))).await;
        let req = test::TestRequest::get().to_request();

        let mut resp = test::call_service(&mut app, req).await;
        assert!(resp.status().is_success());

        // first chunk
        let (bytes, mut resp) = resp.take_body().into_future().await;
        assert_eq!(bytes.unwrap().unwrap(), Bytes::from_static(b"data: 5\n\n"));

        // second chunk
        let (bytes, mut resp) = resp.take_body().into_future().await;
        assert_eq!(bytes.unwrap().unwrap(), Bytes::from_static(b"data: 4\n\n"));

        // remaining part
        let bytes = test::load_stream(resp.take_body().into_stream()).await;
        assert_eq!(bytes.unwrap(), Bytes::from_static(b"data: 3\n\ndata: 2\n\ndata: 1\n\n"));
    }
}
```