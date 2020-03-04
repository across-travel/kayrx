# 安装Rust

由于`kayrx`是Rust框架，因此您将需要Rust来使用. 我们建议您使用它`rustup`来管理您的Rust安装。该 [官员Rust入门](https://doc.rust-lang.org/book/ch01-01-installation.html)精彩的部分。

当前，我们至少需要Rust  1.39，因此请确保您 `rustup update`拥有最新和最好的Rust版本。特别是，本指南将假定您实际运行Rust 1.39 或更高版本。

## 安装`kayrx`

感谢`Rust` 的`cargo`软件包管理器，您无需显式安装 `kayrx`。只需依靠它，您就可以开始了。对于不太可能要使用`kayrx`开发版本的情况，可以直接依赖git存储库.

发布版本：

```toml
[dependencies]
kayrx = "0.7"
```

开发版本：

```toml
[dependencies]
kayrx = { git = "https://github.com/kayrx/kayrx" }
```

## 深入

您可以在此处采取两种方法。您可以按照指南进行操作，如果您不耐烦，则可以查看我们广泛的[示例存储库](https://github.com/kayrx/awesome/tree/master/kayrx-examples) 并运行其中的示例。例如，这是您运行所包含`basic` 示例的方式：

```shell
git clone https://github.com/kayrx/awesome
cd kayrx-examples/basic
cargo run
```
