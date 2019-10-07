#![deny(warnings)]

extern crate futures;
extern crate reqwest;
extern crate tokio;

//use std::io::{self, Cursor};
//use futures::Future;
use regex::Regex;
use reqwest::{Client};//, Response};
use serde::Deserialize;
//use serde_xml_rs::{from_str, to_string};
use env_logger::Env;
use log::{error, info};
use reqwest::StatusCode;
use std::error::Error;
use std::fs::File;
use std::io::Write;
use std::time::Duration;
//use tempfile::Builder;

#[derive(Deserialize, Debug)]
struct FlexStatementResponse {
    #[serde(rename = "Status")]
    status: String,
    #[serde(rename = "ErrorCode")]
    errorcode: u16,
    #[serde(rename = "ErrorMessage")]
    errormessage: String,
}

fn get(client: &reqwest::Client, uri: &str) -> Result<reqwest::Response, Box<dyn Error>> {
    let mut retries = 3;
    let mut delay = 3;
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
fn request_report(client: &reqwest::Client, reference_code: String) -> Result<reqwest::Response, Box<dyn Error>> {
    let mut retries = 3;
    let mut delay = 1;
    let statement_url = format!("https://gdcdyn.interactivebrokers.com/Universal/servlet/FlexStatementService.GetStatement?q={}&t={}&v={}",
                                 reference_code, "168331604313807293250457", 3);

    while retries > 0 {
        info!("retries: {} fetching report: {}", retries, &statement_url);
        match get(client, &statement_url) {
            Err(error) => {
                error!("{}", error);
                panic!("Error fetching items");
            }
            Ok(mut response) => match response.status() {
                StatusCode::OK => {
                    match response.text() {
                        Ok(text) => {
                            let statement_response:Result<FlexStatementResponse,serde_xml_rs::Error> = serde_xml_rs::from_str(&text);
                            match statement_response {
                                Ok(resp) => {
                                match resp.errorcode{
                                        1019 => info!("1019 -- {:#?}", text),
                                        _ => panic!("errororor"),
                                }
                                }
                                _=> {
                                    return Ok(response);
                                }
                            }
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
        std::thread::sleep(Duration::from_secs(delay));
        retries -= 1;
        delay *= 16;
    }
    panic!("Unexpected response");
}

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
    //let tmp_dir = Builder::new().prefix("example").tempdir()?;
    let mut response = request_report(&client, reference_code)?;
    let mut _dest = File::create("/home/czichy/tmp/test.xml")?;
    //info!("Response: {:#?}", response);
 _dest.write_all(&response.text()?.as_bytes())?;
   // response.copy_to(&mut _dest)?;
    Ok(())
}
