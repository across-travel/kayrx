# 单个文件

可以使用自定义路径模式和`NamedFile`来提供静态文件。为了匹配路径尾部，我们可以使用`[.*]`正则表达式

```rust
use keclc_files::NamedFile;
use kayrx::web::{HttpRequest, Result};
use std::path::PathBuf;

async fn index(req: HttpRequest) -> Result<NamedFile> {
    let path: PathBuf = req.match_info().query("filename").parse().unwrap();
    Ok(NamedFile::open(path)?)
}

#[kayrx::main]
async fn main() -> std::io::Result<()> {
    use kayrx::web::{web, App, HttpServer};

    HttpServer::new(|| App::new().route("/{filename:.*}", web::get().to(index)))
        .bind("127.0.0.1:8088")?
        .run()
        .await
}
```

## 目录

`Files`可以支持来自特定目录和子目录的文件,  `Files`必须使用`App::service()`方法注册，否则它将无法支持子路径

```rust
use keclc_files as fs;
use kayrx::web::{App, HttpServer};

#[kayrx::main]
async fn main() -> std::io::Result<()> {
    HttpServer::new(|| {
        App::new().service(fs::Files::new("/static", ".").show_files_listing())
    })
    .bind("127.0.0.1:8088")?
    .run()
    .await
}
```

默认情况下，禁用子目录的文件列表。尝试加载目录列表将返回`404 Not Found`响应。要启用文件列表，请使用 `Files :: show_files_listing()` 方法.

除了显示目录的文件列表外，还可以重定向到特定的索引文件。使用`Files :: index_file()`方法配置此重定向.

## 配置

`NamedFiles` 可以指定用于提供文件的各种选项：

- `set_content_dispostion` - 用于将文件的`mime`映射到相应`Content-Disposition`类型的函数
- `use_etag` - 指定是否`ETag`应计算并包含在标头中
- `use_last_modifier` - 指定是否应使用文件修改的时间戳并将其添加到`Last-Modified`标头 

以上所有方法都是可选的，并提供了最佳默认值，但是可以自定义其中任何一种.

```rust
use keclc_files as fs;
use kayrx::http::header::{ContentDisposition, DispositionType};
use kayrx::web::{web, App, Error, HttpRequest, HttpServer};

async fn index(req: HttpRequest) -> Result<fs::NamedFile, Error> {
    let path: std::path::PathBuf = req.match_info().query("filename").parse().unwrap();
    let file = fs::NamedFile::open(path)?;
    Ok(file
        .use_last_modified(true)
        .set_content_disposition(ContentDisposition {
            disposition: DispositionType::Attachment,
            parameters: vec![],
        }))
}

#[kayrx::main]
async fn main() -> std::io::Result<()> {
    HttpServer::new(|| App::new().route("/{filename:.*}", web::get().to(index)))
        .bind("127.0.0.1:8088")?
        .run()
        .await
}
```

该配置也可以应用于目录服务：

```rust
use keclc_files as fs;
use kayrx::web::{App, HttpServer};

#[kayrx::main]
async fn main() -> std::io::Result<()> {
    HttpServer::new(|| {
        App::new().service(
            fs::Files::new("/static", ".")
                .show_files_listing()
                .use_last_modified(true),
        )
    })
    .bind("127.0.0.1:8088")?
    .run()
    .await
}
```
