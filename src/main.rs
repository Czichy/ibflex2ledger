#![deny(warnings)]

extern crate reqwest;

use regex::Regex;
use reqwest::Client; //, Response};
use serde::Deserialize;
use chrono::NaiveDate;
use env_logger::Env;
use log::{error, info};
use reqwest::StatusCode;
use rust_decimal::Decimal;
use std::error::Error;
use std::fs::File;
use std::fs::OpenOptions;
use std::io::copy;
use std::io::prelude::*;
use std::ops::Neg;
use std::str::FromStr;
use std::time::Duration;
use structopt::StructOpt;
//use structopt::clap::{crat_authors, crate_description, crate_name, crate_version};
use std::process::Command;
use std::path::{PathBuf};
pub mod model;
pub use model::*;

#[derive(Deserialize, Debug)]
struct FlexStatementResponse {
    #[serde(rename = "Status")]
    status: String,
    #[serde(rename = "ErrorCode")]
    errorcode: u16,
    #[serde(rename = "ErrorMessage")]
    errormessage: String,
}
#[derive(Debug, StructOpt)]
//#[structopt(
//    raw(name = "crate_name!()"),
//    raw(version = "crate_version!()"),
//    raw(author = "crate_authors!()"),
//    raw(about = "crate_description!()")
//)]
struct Opt {
    /// Config file
    #[structopt(short = "f", long = "flex_query")]
    query: String,

    /// token file
    #[structopt(short = "t", long = "token")]
    token: String,

    /// download files into destination folder
    #[structopt(short = "d", long = "dump_to_file", parse(from_os_str))]
    dump_file: Option<PathBuf>,

    /// download files into destination folder
    #[structopt(short = "j", long = "journal_file", parse(from_os_str))]
    journal_file: Option<PathBuf>,

    /// Silence all log output
    #[structopt(short = "q", long = "quiet")]
    quiet: bool,

    /// Verbose logging mode (-v, -vv, -vvv)
    #[structopt(short = "v", long = "verbose", parse(from_occurrences))]
    verbose: usize,
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

fn parse_transaction(
    statement_of_funds_line: roxmltree::Node,
    src_account: String,
    counter_account: String,
) -> Transaction {
    Transaction {
        comment: None,
        date: NaiveDate::parse_from_str(
            statement_of_funds_line.attribute("date").unwrap_or(&""),
            "%Y-%m-%d",
        )
        .unwrap(),
        effective_date: NaiveDate::parse_from_str(
            statement_of_funds_line
                .attribute("settleDate")
                .unwrap_or(&""),
            "%Y-%m-%d",
        )
        .ok(),
        status: Some(TransactionStatus::Cleared),
        code: None,
        description: String::from(
            statement_of_funds_line
                .attribute("activityDescription")
                .unwrap_or(&""),
        ),
        postings: vec![
            Posting {
                account: src_account,
                amount: Some(Amount {
                    quantity: Decimal::from_str(
                        statement_of_funds_line.attribute("amount").unwrap_or(""),
                    )
                    .unwrap(),
                    commodity: Commodity {
                        name: statement_of_funds_line
                            .attribute("currency")
                            .unwrap()
                            .to_string(),
                        position: CommodityPosition::Left,
                    },
                }),
                balance: Some(Balance::Amount(Amount {
                    quantity: Decimal::from_str(
                        statement_of_funds_line.attribute("balance").unwrap_or(""),
                    )
                    .unwrap(),
                    commodity: Commodity {
                        name: statement_of_funds_line
                            .attribute("currency")
                            .unwrap()
                            .to_string(),
                        position: CommodityPosition::Left,
                    },
                })),
                status: None,
                comment: Some(
                    statement_of_funds_line
                        .attribute("transactionID")
                        .unwrap()
                        .to_string(),
                ),
            },
            Posting {
                account: counter_account,
                amount: Some(Amount {
                    quantity: Decimal::from_str(
                        statement_of_funds_line.attribute("amount").unwrap_or(""),
                    )
                    .unwrap()
                    .neg(),
                    commodity: Commodity {
                        name: statement_of_funds_line
                            .attribute("currency")
                            .unwrap()
                            .to_string(),
                        position: CommodityPosition::Left,
                    },
                }),
                balance: None,
                status: None,
                comment: None,
            },
        ],
    }
}

fn request_ref_code(client: &reqwest::Client,token: &str,query: String) -> String {
    //let mut fail_count = 0;
    let _url = format!(
        "https://gdcdyn.interactivebrokers.com/Universal/servlet/FlexStatementService.SendRequest?t={}&q={}&v={}",
        token, query, 3
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
fn request_report(
    client: &reqwest::Client,
    token: &str,
    reference_code: String,
) -> Result<String, Box<dyn Error>> {
    let mut retries = 3;
    let mut delay = 1;
    let statement_url = format!("https://gdcdyn.interactivebrokers.com/Universal/servlet/FlexStatementService.GetStatement?q={}&t={}&v={}",
                                 reference_code, token, 3);

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
                            let statement_response: Result<
                                FlexStatementResponse,
                                serde_xml_rs::Error,
                            > = serde_xml_rs::from_str(&text);
                            match statement_response {
                                Ok(resp) => match resp.errorcode {
                                    1019 => info!("1019 -- {:#?}", text),
                                    _ => panic!("errororor"),
                                },
                                _ => {
                                    return Ok(text);
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

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let opt = Opt::from_args();
    // Initialize logger
    if !opt.quiet {
        env_logger::Builder::from_env(Env::default().default_filter_or(match opt.verbose {
            0 => "warn",
            1 => "info",
            2 => "debug",
            _ => "trace",
        }))
        .init();
    }
                let token = match Command::new("sh").arg("-c").arg(&opt.token).output() {
                Ok(output) => String::from_utf8_lossy(&output.stdout).into_owned(),
                Err(e) => {
                    error!("Failed to launch password command for {}: {}", &opt.token, e);
                    return Err(format!("Failed to launch password command for {}: {}", &opt.token, e).into());
                }
            };

    let client = Client::new();
    let reference_code = request_ref_code(&client,&token,opt.query);
    let response = request_report(&client,&token, reference_code)?;
    let mut _dest = File::create("/home/czichy/tmp/test.xml")?;
    copy(&mut response.as_bytes(), &mut _dest)?;
    let doc = match roxmltree::Document::parse(&response) {
        Ok(doc) => doc,
        Err(e) => {
            println!("Error: {}.", e);
            return Err(Box::new(e));
        }
    };
    let mut _journal = OpenOptions::new()
        .write(true)
        .create(true)
        //.append(true)
        .open("/home/czichy/tmp/test.journal")
        .unwrap();

    doc.descendants()
        .filter(|n| n.tag_name().name() == "StatementOfFundsLine")
        .map(|record_data| {
            let counter_account = match record_data.attribute("activityCode").unwrap() {
                "OFEE" => "Ausgaben:Kapitalvermögen:Laufende Ausgaben:Depotspesen".to_string(),
                "DEP" | "WITH"  => "Equity:Transfers".to_string(),
                _ => "Vermögen:Kapitalvermögen:Finanzinstrumente:Interactive Brokers".to_string(),
            };
            parse_transaction(
                record_data,
                "Vermögen:Kapitalvermögen:Guthaben:Verrechnungskonto:Interactive Brokers"
                    .to_string(),
                counter_account,
            )
        })
        .for_each(|t| writeln!(_journal, "{}", t).unwrap());
    Ok(())
}
