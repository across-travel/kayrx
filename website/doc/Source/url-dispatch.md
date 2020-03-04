# 路由分发

URL分发提供了一种使用简单的模式匹配将URL映射到处理程序代码的方法. 如果其中一种模式与与请求关联的路径信息匹配，则调用特定的处理程序对象。

> 请求处理程序是一个函数，该函数接受零个或多个可以从请求中提取的参数（即`impl FromRequest`），并返回可以转换为`HttpResponse`的类型(即`impl Responder`). 有关更多信息，请参见[处理程序](../basic/handler.html)部分。

## 资源配置

资源配置是向应用程序添加新资源的动作。资源具有名称，该名称用于URL生成的标识符。该名称还允许开发人员向现有资源添加路由。资源也有一个模式，这意味着匹配一个`URL`的`PATH`部分(该`scheme`和端口/port 后面的部分，例如在URL`http://localhost:8080/foo/bar?q=value` 的`/foo/bar`). 它不匹配针对`QUERY`部分(`？`后面的部分，例如`http://localhost:8080/foo/bar?q=value`中的`q=value`).

的 [App::route()](https://docs.rs/kayrx/0.7.5/kayrx/web/struct.App.html#method.route)方法提供注册路由的简单方法。此方法将单个路由添加到应用程序路由表。该方法接受 **path pattern**,**http method** 和处理函数。对于同一路径，可以多次调用该`route()`方法，在这种情况下，多个路由注册同一资源路径。

```rust
use kayrx::web::{web, App, HttpResponse, HttpServer};

async fn index() -> HttpResponse {
    HttpResponse::Ok().body("Hello")
}

#[kayrx::main]
async fn main() -> std::io::Result<()> {
    HttpServer::new(|| {
        App::new()
            .route("/", web::get().to(index))
            .route("/user", web::post().to(index))
    })
    .bind("127.0.0.1:8088")?
    .run()
    .await
}
```

虽然`App :: route()`提供了注册路由的简单方法，但要访问完整的资源配置，则必须使用其他方法。该[*App::service()]()方法将单个的资源添加到应用路由表。此方法接受 路径模式(path pattern)，守卫(guards)和一条或多条路由

```rust
use kayrx::web::{guard, web, App, HttpResponse};

fn index() -> HttpResponse {
    HttpResponse::Ok().body("Hello")
}

pub fn main() {
    App::new()
        .service(web::resource("/prefix").to(index))
        .service(
            web::resource("/user/{name}")
                .name("user_detail")
                .guard(guard::Header("content-type", "application/json"))
                .route(web::get().to(|| HttpResponse::Ok()))
                .route(web::put().to(|| HttpResponse::Ok())),
        );
}
```

如果资源不包含任何路由或不具有任何匹配的路由，则它将返回`NOT FOUND` http响应。

## 配置路由

资源包含一组路由。每个路由依次具有一组守卫(guards)和一个处理程序。可以使用`Resource::route()`方法创建新路由，该方法返回对新`Route`实例的引用。默认情况下，该路由不包含任何守卫，因此会匹配所有请求，并且默认处理函数为`HttpNotFound`。

该应用程序根据在资源注册和路由注册期间定义的路由标准来路由传入的请求。资源按照通过路由注册(`Resource::route()`)的顺序匹配它包含的所有路由。

> 一个路由可以包含任意数量的守卫(guards)，但只能包含一个处理程序

```rust
App::new().service(
    web::resource("/path").route(
        web::route()
            .guard(guard::Get())
            .guard(guard::Header("content-type", "text/plain"))
            .to(|| HttpResponse::Ok()),
    ),
)
```

在此示例中，如果请求包含`Content-Type`标头，此标头的值为`text / plain`且路径等于`/path`，`GET`请求则返回为`HttpResponse::Ok()`。

如果资源不能匹配任何路由，则返回`NOT FOUND`响应。

[ResourceHandler::route()]([resourcehandler](https://docs.rs/kayrx/0.7.5/kayrx/web/struct.Resource.html#method.route)) 返回 [Route](https://docs.rs/kayrx/0.7.5/kayrx/web//struct.Route.html)对象。可以使用类似构建器的模式来配置`Route`路由。提供以下配置方法：

* `Route::guard()` 注册一个新的守卫, 每个路线可以注册任意数量的守卫.
* `Route::method()` 注册方法守卫, 每个路线可以注册任意数量的守卫.
* `Route::to()` 注册此路由的处理函数, 只能注册一个处理程序, 通常处理程序注册是最后的配置操作.
* `Route::to_async()` 为此路由注册一个异步处理函数, 只能注册一个处理程序, 通常处理程序注册是最后的配置操作

## 路由匹配

路由配置的主要目的是根据URL`path`路径 模式匹配或不匹配请求, `path`代表所请求URL的路径部分.

当请求进入系统时，对于系统中存在的每个资源配置声明，`kayrx::web`都会根据声明的模式检查请求的路径。该检查按照通过`App::service()`方法声明路由的顺序进行。如果找不到资源，则将默认资源用作匹配的资源。

声明路由配置时，它可能包含路由守卫参数。与路由声明关联的所有路由守卫必须true用于在检查期间将路由配置用于给定的请求。如果false在检查期间返回给路由配置的一组路由守卫参数中的任何守卫返回，则该路由将被跳过，并且路由匹配将继续通过有序路由集合。

When a route configuration is declared, it may contain route guard arguments. All route
guards associated with a route declaration must be `true` for the route configuration to be used for a given request during a check. If any guard in the set of route guard
arguments provided to a route configuration returns `false` during a check, that route is skipped and route matching continues through the ordered set of routes.

如果有任何路由匹配，则路由匹配过程停止，并调用与该路由关联的处理程序。如果在所有路由模式都用尽之后没有路由匹配，则 返回`NOT FOUND`响应

## 资源模式语法

在参数中使用的模式匹配的语法很简单。

路由配置中使用的模式可以以斜杠字符开头。如果该模式不以斜杠字符开头，则在匹配时将在其前面添加一个隐式斜杠。例如，以下模式是等效的 : 

```rust
{foo}/bar/baz
```

```rust
/{foo}/bar/baz
```

可变部分(替换标记)用`{标识符}`的形式指定，其中，该配置 "接受任何字符到下一个斜线字符并使用它作为`HttpRequest.match_info()`对象中的名称". 

模式中的替换标记匹配正则表达式: `[^{}/]+`

`match_info`是代表根据路由模式从URL提取的`Params`对象的动态部分 。它可以作`request.match_info`使用。例如，以下模式定义一个文字段`（foo）`和两个替换标记（`baz`和`bar`）：

```rust
foo/{baz}/{bar}
```

上面的模式将匹配这些URL，生成以下匹配信息：

```rust
foo/1/2        -> Params {'baz':'1', 'bar':'2'}
foo/abc/def    -> Params {'baz':'abc', 'bar':'def'}
```

但是，它将不符合以下模式：

```rust
foo/1/2/        -> No match (trailing slash)
bar/abc/def     -> First segment literal mismatch
```

段中段替换标记的匹配将仅进行到模式中段中第一个非字母数字字符为止。因此，例如，如果使用此路由模式：

The match for a segment replacement marker in a segment will be done only up to
the first non-alphanumeric character in the segment in the pattern. So, for instance,
if this route pattern was used:

```rust
foo/{name}.html
```

字符串路径`/foo/biz.html`将匹配上述路由模式，匹配结果将为`Params{'name': 'biz'}`。然而，字符串路径`/foo/biz`将不匹配，因为在所代表的段的端点`{name}.html`, 它不包含一个字面的`.html`（它仅包含`biz`，而不是`biz.html`）。

要捕获两个片段，可以使用两个替换标记：

```rust
foo/{name}.{ext}
```

字面路径`/foo/biz.html`将匹配上述路由模式，并且匹配结果将为`Params {'name'：'biz'，'ext'：'html'}`。发生这种情况是因为的字面部分(句点)在两个替换标记`{name}`和`{ext}`之间。

替换标记可以选择指定正则表达式，该正则表达式将用于确定路径段是否应与标记匹配。若要指定替换标记仅应匹配正则表达式定义的一组特定字符，则必须使用稍微扩展形式的替换标记语法。在大括号内，替换标记名必须后跟一个冒号，然后是正则表达式。与替换标记`[^ /] +`关联的默认正则表达式匹配是一个或多个非斜杠字符。例如，在内部，替换标记`{foo}`可以更详细地拼写为`{foo：[^ /] +}`。您可以将其更改为任意正则表达式以匹配任意字符序列，例如`{foo：\ d +}`以仅匹配数字。

段必须至少包含一个字符才能匹配段替换标记。例如，对于URL`/ abc /`：

* */abc/{foo}*将不匹配
* */{foo}/* 将匹配

> **注意**：在匹配模式之前，将对路径进行`URL-unquoted`并将其解码为有效的`unicode`字符串，并且表示匹配路径段的值也将被`URL-unquoted`

```rust
foo/{bar}
```

匹配以下网址时：

```rust
http://example.com/foo/La%20Pe%C3%B1a
```

`match dict`看起来像这样（值是URL解码的）：

```rust
Params{'bar': 'La Pe\xf1a'}
```

路径段中的文字字符串应代表路径的解码值。您应在模式中使用URL编码的值。例如：

```rust
/Foo%20Bar/{baz}
```

您应使用以下内容：

```rust
/Foo Bar/{baz}
```

可以得到 "尾部匹配" , 必须使用自定义正则表达式:

```rust
foo/{bar}/{tail:.*}
```

上面的模式将匹配这些URL，生成以下匹配信息：

```rust
foo/1/2/           -> Params{'bar':'1', 'tail': '2/'}
foo/abc/def/a/b/c  -> Params{'bar':u'abc', 'tail': 'def/a/b/c'}
```

## 路由域

范围界定可以帮助您组织共享公共根路径的路由。您可以将范围嵌套在范围内。

假设您要组织用于查看"用户"的端点的路径。这些路径可能包括：

- /users
- /users/show
- /users/show/{id}

这些路径的作​​用域布局如下所示

```rust
async fn show_users() -> HttpResponse {
    HttpResponse::Ok().body("Show users")
}

async fn user_detail(path: web::Path<(u32,)>) -> HttpResponse {
    HttpResponse::Ok().body(format!("User detail: {}", path.0))
}

#[kayrx::main]
async fn main() -> std::io::Result<()> {
    HttpServer::new(|| {
        App::new().service(
            web::scope("/users")
                .route("/show", web::get().to(show_users))
                .route("/show/{id}", web::get().to(user_detail)),
        )
    })
    .bind("127.0.0.1:8088")?
    .run()
    .await
}
```

作用域(scope)路径可以包含可变路径段作为资源。与无作用域的路径一致。

您可以从`HttpRequest::match_info()`获得可变路径段, `Path`提取器还能够提取作用域范围级别的变量段.

## 匹配信息

代表匹配路径段的所有值都可在`HttpRequest::match_info`中找到。可以使用`Path::get()`检索特定值。

```rust
use kayrx::web::{HttpRequest, HttpResponse, Result};

async fn index(req: HttpRequest) -> Result<String> {
    let v1: u8 = req.match_info().get("v1").unwrap().parse().unwrap();
    let v2: u8 = req.match_info().query("v2").parse().unwrap();
    let (v3, v4): (u8, u8) = req.match_info().load().unwrap();
    Ok(format!("Values {} {} {} {}", v1, v2, v3, v4))
}

#[kayrx::main]
async fn main() -> std::io::Result<()> {
    use kayrx::web::{web, App, HttpServer};

    HttpServer::new(|| {
        App::new()
            .route("/a/{v1}/{v2}/", web::get().to(index))
            .route("", web::get().to(|| HttpResponse::Ok()))
    })
    .bind("127.0.0.1:8088")?
    .run()
    .await
}
```

对于此示例的路径`/a/1/2/`，值`v1`和`v2`将解析为“ 1”和“ 2”

可以从尾部路径参数创建一个`PathBuf`。返回的`PathBuf`内容已`percent-decoded`。如果一个段等于“ ..”，则跳过前一个段（如果有）。

为了安全起见，如果段满足以下任一条件，则返回`Err`：

* 解码的段以以下任意一项开头：`.` ( `..`除外), `*`
* 解码的段以以下任意一项结尾：`:`, `>`, `<`
* 解码的段包含以下任何内容： /
* 在Windows上，解码的段包含以下任何内容：`\`
* Percent-encoding 结果是无效的UTF8

这些条件的结果是，从请求路径参数解析的`PathBuf`结果可以安全地插在路径中或用作路径的后缀，而无需其他检查。

```rust
use kayrx::web::{HttpRequest, Result};
use std::path::PathBuf;

async fn index(req: HttpRequest) -> Result<String> {
    let path: PathBuf = req.match_info().query("tail").parse().unwrap();
    Ok(format!("Path {:?}", path))
}

#[kayrx::main]
async fn main() -> std::io::Result<()> {
    use kayrx::web::{web, App, HttpServer};

    HttpServer::new(|| App::new().route(r"/a/{tail:.*}", web::get().to(index)))
        .bind("127.0.0.1:8088")?
        .run()
        .await
}
```

## 路径信息提取器

`kayrx::web`提供了用于类型安全路径信息提取的功能。 路径 提取信息，目标类型可以几种不同的形式定义。最简单的方法是使用`tuple`类型。元组中的每个元素必须对应于路径模式中的一个元素。即您可以将路径模式`/{id}/{username}/`与 `Path<(u32, String)>`类型进行匹配，但`Path<(String, String, String)>`类型将始终失败

```rust
use kayrx::web::{web, Result};

async fn index(info: web::Path<(String, u32)>) -> Result<String> {
    Ok(format!("Welcome {}! id: {}", info.0, info.1))
}

#[kayrx::main]
async fn main() -> std::io::Result<()> {
    use kayrx::web::{App, HttpServer};

    HttpServer::new(|| {
        App::new().route(
            "/{username}/{id}/index.html", // <- define path parameters
            web::get().to(index),
        )
    })
    .bind("127.0.0.1:8088")?
    .run()
    .await
}
```

还可以将路径模式信息提取到结构。在这种情况下，此结构必须实现`serde`的` Deserialize`特征。

```rust
use kayrx::web::{web, Result};
use serde::Deserialize;

#[derive(Deserialize)]
struct Info {
    username: String,
}

// extract path info using serde
async fn index(info: web::Path<Info>) -> Result<String> {
    Ok(format!("Welcome {}!", info.username))
}

#[kayrx::main]
async fn main() -> std::io::Result<()> {
    use kayrx::web::{App, HttpServer};

    HttpServer::new(|| {
        App::new().route(
            "/{username}/index.html", // <- define path parameters
            web::get().to(index),
        )
    })
    .bind("127.0.0.1:8088")?
    .run()
    .await
}
```

`Query`为请求查询参数提供了类似的功能.

## 生成资源URLs

使用`HttpRequest.url_for()`方法可基于资源模式生成`URL`。例如，如果您使用名称 " foo" 和模式 " {a} / {b} / {c}" 配置了资源，则可以这样做：

```rust
use kayrx::web::{guard, http::header, HttpRequest, HttpResponse, Result};

async fn index(req: HttpRequest) -> Result<HttpResponse> {
    let url = req.url_for("foo", &["1", "2", "3"])?; // <- generate url for "foo" resource

    Ok(HttpResponse::Found()
        .header(header::LOCATION, url.as_str())
        .finish())
}

#[kayrx::main]
async fn main() -> std::io::Result<()> {
    use kayrx::web::{web, App, HttpServer};

    HttpServer::new(|| {
        App::new()
            .service(
                web::resource("/test/{a}/{b}/{c}")
                    .name("foo") // <- set resource name, then it could be used in `url_for`
                    .guard(guard::Get())
                    .to(|| HttpResponse::Ok()),
            )
            .route("/test/", web::get().to(index))
    })
    .bind("127.0.0.1:8088")?
    .run()
    .await
}
```

这将返回类似于`http://example.com/test/1/2/3`的字符串信息（至少在当前协议和主机名暗含`http://example.com的情况下`）。 `url_for()`方法返回Url对象，因此您可以修改此`url` (添加查询参数，锚点等)。 `url_for()`只能为命名资源调用，否则将返回错误

## 外部资源

可以将有效URL的资源注册为外部资源。它们仅用于生成URL，在请求时从不考虑匹配。

```rust
use kayrx::web::{HttpRequest, Responder};

async fn index(req: HttpRequest) -> impl Responder {
    let url = req.url_for("youtube", &["oHg5SJYRHA0"]).unwrap();
    assert_eq!(url.as_str(), "https://youtube.com/watch/oHg5SJYRHA0");

    url.into_string()
}

#[kayrx::main]
async fn main() -> std::io::Result<()> {
    use kayrx::web::{web, App, HttpServer};

    HttpServer::new(|| {
        App::new()
            .route("/", web::get().to(index))
            .external_resource("youtube", "https://youtube.com/watch/{video_id}")
            .route("/", web::get().to(index))
    })
    .bind("127.0.0.1:8088")?
    .run()
    .await
}
```

## 路径规范化并重定向到斜杠附加的路由

通过规范化意味着：

* 向路径添加尾部斜杠
* 用一个代替多个斜杠

处理程序找到正确解析的路径后立即返回。如果全部启用，标准化条件的顺序为1）合并，2）合并和附加 3）附加。如果路径至少满足上述条件之一，则它将重定向到新路径

```rust
use kayrx::web::{middleware, HttpResponse};

async fn index() -> HttpResponse {
    HttpResponse::Ok().body("Hello")
}

#[kayrx::main]
async fn main() -> std::io::Result<()> {
    use kayrx::web::{web, App, HttpServer};

    HttpServer::new(|| {
        App::new()
            .wrap(middleware::NormalizePath)
            .route("/resource/", web::to(index))
    })
    .bind("127.0.0.1:8088")?
    .run()
    .await
}
```

在此示例中，`//resource///`将重定向到`/resource/`

在此示例中，所有方法都注册了路径规范化处理程序，但是您不应依赖此机制来重定向`POST`请求。带有斜杠的`Not Found`重定向会将`POST`请求转换为`GET`，从而丢失原始请求中的所有 `POST`数据。

可以仅针对`GET`请求注册路径规范化：

```rust
use kayrx::web::{http::Method, middleware, web, App, HttpServer};

#[kayrx::main]
async fn main() -> std::io::Result<()> {
    HttpServer::new(|| {
        App::new()
            .wrap(middleware::NormalizePath)
            .route("/resource/", web::get().to(index))
            .default_service(web::route().method(Method::GET))
    })
    .bind("127.0.0.1:8088")?
    .run()
    .await
}
```

## 使用应用程序前缀编写应用

该`web::scope()`方法允许设置特定的应用范围。此范围表示资源前缀，该前缀将附加到资源配置添加的所有资源模式中。这可用于帮助将一组路由安装在与所包含的可调用者不同的位置，同时仍保持相同的资源名称。

例如：

```rust
async fn show_users() -> HttpResponse {
    HttpResponse::Ok().body("Show users")
}

async fn user_detail(path: web::Path<(u32,)>) -> HttpResponse {
    HttpResponse::Ok().body(format!("User detail: {}", path.0))
}

#[kayrx::main]
async fn main() -> std::io::Result<()> {
    HttpServer::new(|| {
        App::new().service(
            web::scope("/users")
                .route("/show", web::get().to(show_users))
                .route("/show/{id}", web::get().to(user_detail)),
        )
    })
    .bind("127.0.0.1:8088")?
    .run()
    .await
}
```

在上面的示例中，`show_users`路由将具有`/ users / show`而不是`/ show`的有效路由模式， 因为应用的作用域将位于该模式之前。然后，仅当URL路径为`/ users / show`时，路由才会匹配，并且使用路由`show_users`名称调用该`HttpRequest.url_for()`函数时，它将生成具有相同的UR路径L

## 自定义路由守卫

您可以将[Guard](https://docs.rs/kayrx/0.7.5/kayrx/web/guard/trait.Guard.html)看作一个简单的函数，它接受`request`对象引用并返回`true`或`false`。形式上，守卫是实现该`Guard`特征的任何对象 。`kayrx::web`提供了多个谓词，您可以检查 api文档的[functions部分](https://docs.rs/kayrx/0.7.5/kayrx/web/guard/index.html#functions)。

这是一个简单的守卫，用于检查请求是否包含特定的标头：

```rust
use kayrx::web::{dev::RequestHead, guard::Guard, http, HttpResponse};

struct ContentTypeHeader;

impl Guard for ContentTypeHeader {
    fn check(&self, req: &RequestHead) -> bool {
        req.headers().contains_key(http::header::CONTENT_TYPE)
    }
}

#[kayrx::main]
async fn main() -> std::io::Result<()> {
    use kayrx::web::{web, App, HttpServer};

    HttpServer::new(|| {
        App::new().route(
            "/",
            web::route()
                .guard(ContentTypeHeader)
                .to(|| HttpResponse::Ok()),
        )
    })
    .bind("127.0.0.1:8088")?
    .run()
    .await
}
```

在此示例中，仅当请求包含`CONTENT-TYPE`标头时才调用`index`处理程序。

守卫无法访问或修改请求对象，但是可以在[请求扩展](https://docs.rs/kayrx/0.7.5/kayrx/web/struct.HttpRequest.html#method.extensions)中存储额外的信息

## 修改守卫值

您可以通过将任何谓词值包装在`Not`谓词中来反转其含义。例如，如果要为除 `GET`以外的所有方法返回 `METHOD NOT ALLOWED`响应，请执行以下操作：

```rust
use kayrx::web::{guard, web, App, HttpResponse, HttpServer};

#[kayrx::main]
async fn main() -> std::io::Result<()> {
    HttpServer::new(|| {
        App::new().route(
            "/",
            web::route()
                .guard(guard::Not(guard::Get()))
                .to(|| HttpResponse::MethodNotAllowed()),
        )
    })
    .bind("127.0.0.1:8088")?
    .run()
    .await
}
```

该`Any`守卫接受 守卫列表, 如果提供的任何守卫匹配，则匹配 :

```rust
guard::Any(guard::Get()).or(guard::Post())
```

该`All`守卫接受守卫列表,和匹配的列表，如果提供的所有守卫匹配, 则匹配 :

```rust
guard::All(guard::Get()).and(guard::Header("content-type", "plain/text"))
```

## 更改默认的`Not Found`响应

如果在路由表中找不到路径模式或资源找不到匹配的路由，则使用默认资源。默认响应是`NOT FOUND`。可以使用`App::default_service()`来覆盖`NOT FOUND`响应。此方法接受与使用`App::service()`进行常规资源配置方法相同的配置功函数.

```rust
#[kayrx::main]
async fn main() -> std::io::Result<()> {
    HttpServer::new(|| {
        App::new()
            .service(web::resource("/").route(web::get().to(index)))
            .default_service(
                web::route()
                    .guard(guard::Not(guard::Get()))
                    .to(|| HttpResponse::MethodNotAllowed()),
            )
    })
    .bind("127.0.0.1:8088")?
    .run()
    .await
}
```