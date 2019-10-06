#![deny(warnings)]

extern crate futures;
extern crate reqwest;
extern crate tokio;

//use std::io::{self, Cursor};
//use futures::Future;
use regex::Regex;
use reqwest::Client; //, Response};
use serde::Deserialize;
//use serde_xml_rs::{from_str, to_string};
use env_logger::Env;
use log::{error, info};
use reqwest::StatusCode;
use std::error::Error;
use std::fs::File;
use std::io::copy;
use std::time::Duration;
use tempfile::Builder;

#[derive(Deserialize, Debug)]
struct Slideshow {
    title: String,
    author: String,
}

#[derive(Deserialize, Debug)]
struct SlideshowContainer {
    slideshow: Slideshow,
}

fn get(client: &reqwest::Client, uri: &str) -> Result<reqwest::Response, Box<dyn Error>> {
    let mut retries = 3;
    let mut delay = 1;
    loop {
        match client.get(uri).send() {
            Ok(response) => {
                return Ok(response);
            }
            Err(ref error) if retries > 0 => {
                eprintln!("{:?}\n", error);
            }
            Err(error) => {
                return Err(error.into());
            }
        }
        std::thread::sleep(Duration::from_secs(delay));
        retries -= 1;
        delay *= 16;
    }
}

fn request_ref_code(client: &reqwest::Client) -> String {
    //let mut fail_count = 0;
    let _url = format!(
        "https://gdcdyn.interactivebrokers.com/Universal/servlet/FlexStatementService.SendRequest?t={}&q={}&v={}",
        "168331604313807293250457", "381004", 3
    );
    match get(client, &_url) {
        Err(error) => {
            error!("{}", error);
            panic!("Error fetching items");
        }
        Ok(mut response) => match response.status() {
            StatusCode::OK => {
                match response.text() {
                    Ok(text) => {
                        //info!("{:#?}", text);
                        let re = Regex::new(r">(?P<ref>.*)</ReferenceCode>").unwrap();
                        match re.captures(&text) {
                            Some(m) => {
                                return m.name("ref").map_or("".into(), |m| {
                                    info!("Got ReferenceCode: {}", m.as_str().to_string());
                                    m.as_str().to_string()
                                })
                            }
                            _ => panic!("Error parsing ReferenceCode"),
                        };
                    }

                    Err(error) => {
                        // error receiving full response, try again with same link
                        eprintln!("{}", error);
                        panic!("Partial response");
                    }
                }
            }
            status => {
                eprintln!(
                    "Response {:?} {}",
                    status,
                    status.canonical_reason().unwrap()
                );
            }
        },
    }
    panic!("Unexpected response");
}
//client
//        .get(&_url)
//        .send()
//        .and_then(|mut res| {
//            println!("{}", res.status());
//            res.text()
//        })
//        .map_err(|err| println!("request error: {}", err))
//        .map(|body| {
//            let re = Regex::new(r">(?P<ref>.*)</ReferenceCode>").unwrap();
//            match re.captures(&body) {
//                Some(m) => Ok(m.name("ref").map_or("".into(), |m| m.as_str().to_string())),
//                _ => Err("kaputt"),
//            }
//        });
//    request_code
//    .map(|ref_code| {
// let statement_url = format!("https://gdcdyn.interactivebrokers.com/Universal/servlet/FlexStatementService.GetStatement?q={}&t={}&v={}",
//                             ref_code.unwrap(), "168331604313807293250457", 3);
//        println!("{:#?}",statement_url);
//        statement_url
//    });
//}
//fn fetch() -> impl Future<Item = (), Error = ()> {
//    let client = Client::new();
//    let _url = format!(
//        "https://gdcdyn.interactivebrokers.com/Universal/servlet/FlexStatementService.SendRequest?t={}&q={}&v={}",
//        "168331604313807293250457", "381004", 3
//    );
//    let request_code =
//client
//        .get(&_url)
//        .send()
//        .and_then(|mut res| {
//            println!("{}", res.status());
//            res.text()
//        })
//        .map_err(|err| println!("request error: {}", err))
//        .map(|body| {
//            let re = Regex::new(r">(?P<ref>.*)</ReferenceCode>").unwrap();
//            match re.captures(&body) {
//                Some(m) => Ok(m.name("ref").map_or("".into(), |m| m.as_str().to_string())),
//                _ => Err("kaputt"),
//            }
//        })
//        .map(|ref_code| {
//     let statement_url = format!("https://gdcdyn.interactivebrokers.com/Universal/servlet/FlexStatementService.GetStatement?q={}&t={}&v={}",
//                                 ref_code.unwrap(), "168331604313807293250457", 3);
//            println!("{:#?}",statement_url);
//            statement_url
//        });
//
//    request_code.and_then(|url| {
//            let mut retries = 3;
//    let mut delay = 1;
//    loop {
//        Client::new()
//            .get(&url)
//            .send()
//            .and_then(|mut res| res.text())
//            .map_err(|err| println!("request error: {}", err))
//            .map(|body| {
//                println!("{:#?}", body);
//            });
//            std::thread::sleep(Duration::from_secs(delay));
//        retries -= 1;
//        delay *= 16;
//    }
//
//    })
//}

//fn main() {
//    tokio::run(fetch());
//}
fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize logger
    //if !opt.quiet {
    //    env_logger::Builder::from_env(Env::default().default_filter_or(match opt.verbose {
    //        0 => "warn",
    //        1 => "info",
    //        2 => "debug",
    //        _ => "trace",
    //    }))
    //    .default_format_timestamp(false)
    //    .init();
    //}
    env_logger::from_env(Env::default().default_filter_or("info")).init();
    let client = Client::new();
    let reference_code = request_ref_code(&client);
    let tmp_dir = Builder::new().prefix("example").tempdir()?;
    let statement_url = format!("https://gdcdyn.interactivebrokers.com/Universal/servlet/FlexStatementService.GetStatement?q={}&t={}&v={}",
                                 reference_code, "168331604313807293250457", 3);
    let mut response = get(&client, &statement_url)?;

    let mut dest = {
        let fname = response
            .url()
            .path_segments()
            .and_then(|segments| segments.last())
            .and_then(|name| if name.is_empty() { None } else { Some(name) })
            .unwrap_or("tmp.bin");

        println!("file to download: '{}'", fname);
        let fname = tmp_dir.path().join(fname);
        println!("will be located under: '{:?}'", fname);
        File::create(fname)?
    };
info!("{:#?}",response.text());
    copy(&mut response, &mut dest)?;
    Ok(())
}
