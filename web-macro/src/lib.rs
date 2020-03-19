extern crate proc_macro;

use syn::parse::{Parse, ParseStream, Result};
use proc_macro::TokenStream;
use syn::parse_macro_input;
use quote::quote;
use std::env;
use std::fs;
use std::fs::File;
use std::fs::OpenOptions;
use std::io::Write;
use std::path::Path;
use std::sync::Mutex;
use lazy_static::lazy_static;

use crate::parser::HtmlParser;
use crate::tag::Tag;

mod parser;
mod tag;
mod validation;

/// Used to generate VirtualNode's from a TokenStream.
///
/// html! { <div> Welcome to the html! procedural macro! </div> }
#[proc_macro]
pub fn html(input: proc_macro::TokenStream) -> proc_macro::TokenStream {
    let parsed = parse_macro_input!(input as Html);

    let mut html_parser = HtmlParser::new();

    let parsed_tags_len = parsed.tags.len();

    // Iterate over all of our parsed tags and push them into our HtmlParser one by one.
    //
    // As we go out HtmlParser will maintain some heuristics about what we've done so far
    // since that will sometimes inform how to parse the next token.
    for (idx, tag) in parsed.tags.iter().enumerate() {
        let mut next_tag = None;

        if parsed_tags_len - 1 > idx {
            next_tag = Some(&parsed.tags[idx + 1])
        }

        html_parser.push_tag(tag, next_tag);
    }

    html_parser.finish().into()
}

#[derive(Debug)]
struct Html {
    tags: Vec<Tag>,
}

impl Parse for Html {
    fn parse(input: ParseStream) -> Result<Self> {
        let mut tags = Vec::new();

        while !input.is_empty() {
            let tag: Tag = input.parse()?;
            tags.push(tag);
        }

        Ok(Html { tags })
    }
}


/// CSS macro is a procedural macro that allows you to write your CSS next to your Rust views.

lazy_static! {
    static ref CSS_COUNTER: Mutex<u32> = Mutex::new(0);
}

/// Parses the syntax for writing inline css. Every call to css! will have its class
/// name incremented by one.
///
/// So your first css! call is class "._css_rs_0", then "._css_rs_1", etc.
///
/// To write your css to a file use:
///
/// ```ignore
/// OUTPUT_CSS=/path/to/my/output.css cargo run my-app
/// ```
///
/// # Examples
///
/// ```ignore
/// #![feature(use_extern_macros)]
/// #![feature(proc_macro_hygiene)]
///
/// use kayrx::webui::css;
///
/// fn main () {
///     let class1 = css! {
///       "
///       :host {
///         background-color: red;
///       }
///
///       :host > div {
///         display: flex;
///         align-items: center;
///       }
///       "
///     };
///
///     let class2 = css! {r#"
///         :host { display: flex; }
///     "#};
///
///     assert_eq!(class1, "_css_rs_0".to_string());
///     assert_eq!(class2, "_css_rs_1".to_string());
/// }
/// ```
#[proc_macro]
pub fn css(input: TokenStream) -> TokenStream {
    let mut css_counter = CSS_COUNTER.lock().unwrap();

    let class = format!("_css_rs_{}", css_counter);

    let css_file = env::vars().find(|(key, _)| key == "OUTPUT_CSS");

    if css_file.is_some() {
        let css_file = css_file.unwrap().1;

        if *css_counter == 0 {
            if Path::new(&css_file).exists() {
                fs::remove_file(&css_file).unwrap();
            }

            let mut css_file = OpenOptions::new()
                .write(true)
                .create_new(true)
                .open(css_file)
                .unwrap();

            write_css_to_file(&mut css_file, &class, input);
        } else {
            let mut css_file = OpenOptions::new().append(true).open(css_file).unwrap();

            write_css_to_file(&mut css_file, &class, input);
        }
    }

    *css_counter += 1;

    let expanded = quote! {
    #class
    };

    expanded.into()
}

fn write_css_to_file(css_file: &mut File, class: &str, input: TokenStream) {
    for css in input.into_iter() {
        let mut css = css.to_string();

        // Remove the surrounding quotes from so that we can write only the
        // CSS to our file.
        //
        // Handles:
        //   css!{r#" :host { ... } "#}
        //     as well as
        //   css!{" :host { ... } "}
        let first_quote_mark = css.find(r#"""#).unwrap();
        let last_quote_mark = css.rfind(r#"""#).unwrap();
        css.truncate(last_quote_mark);
        let mut css = css.split_off(first_quote_mark + 1);

        // Replace :host selectors with the class name of the :host element
        // A fake shadow-dom implementation.. if you will..
        let css = css.replace(":host", &format!(".{}", class));

        css_file.write(&css.into_bytes()).unwrap();
        css_file.write("\n".as_bytes()).unwrap();
    }
}