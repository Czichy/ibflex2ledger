#![deny(warnings)]

extern crate futures;
extern crate reqwest;
extern crate tokio;

//use std::io::{self, Cursor};
use futures::Future;
use regex::Regex;
use reqwest::r#async::Client; //, Response};
use serde::Deserialize;
//use serde_xml_rs::{from_str, to_string};

#[derive(Deserialize, Debug)]
struct Slideshow {
    title: String,
    author: String,
}

#[derive(Deserialize, Debug)]
struct SlideshowContainer {
    slideshow: Slideshow,
}

fn fetch() -> impl Future<Item = (), Error = ()> {
    let client = Client::new();
    let _url = format!(
        "https://gdcdyn.interactivebrokers.com/Universal/servlet/FlexStatementService.SendRequest?t={}&q={}&v={}",
        "168331604313807293250457", "381004", 3
    );
    let request_code =
client
        .get(&_url)
        .send()
        .and_then(|mut res| {
            println!("{}", res.status());
            res.text()
        })
        .map_err(|err| println!("request error: {}", err))
        .map(|body| {
            let re = Regex::new(r">(?P<ref>.*)</ReferenceCode>").unwrap();
            match re.captures(&body) {
                Some(m) => Ok(m.name("ref").map_or("".into(), |m| m.as_str().to_string())),
                _ => Err("kaputt"),
            }
        })
        .map(|ref_code| {
     let statement_url = format!("https://gdcdyn.interactivebrokers.com/Universal/servlet/FlexStatementService.GetStatement?q={}&t={}&v={}",
                                 ref_code.unwrap(), "168331604313807293250457", 3);
            println!("{:#?}",statement_url);
            statement_url
        });

    request_code.and_then(|url| {
        Client::new()
            .get(&url)
            .send()
            .and_then(|mut res| res.text())
            .map_err(|err| println!("request error: {}", err))
            .map(|body| {
                println!("{:#?}", body);
            })
    })
}

fn main() {
    tokio::run(fetch());
}
