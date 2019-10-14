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
use std::time::Duration; //use tempfile::Builder;
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

#[derive(Deserialize, Debug)]
struct FlexQueryResponse {
    #[serde(rename = "FlexStatements")]
    flex_statements: FlexStatements,
    #[serde(rename = "queryName")]
    query_name: String,
    #[serde(rename = "type")]
    r#type: String,
}

#[derive(Deserialize, Debug)]
struct FlexStatements {
    #[serde(rename = "FlexStatement")]
    flex_statement: FlexStatement,
    count: String,
}

#[derive(Deserialize, Debug)]
struct FlexStatement {
    #[serde(rename = "accountId")]
    account_id: String,

    #[serde(rename = "fromDate")]
    from_date: String,

    #[serde(rename = "toDate")]
    to_date: String,

    #[serde(rename = "period")]
    period: String,

    #[serde(rename = "whenGenerated")]
    when_generated: String,

    #[serde(rename = "StmtFunds")]
    statement_of_funds: Vec<StatementOfFundsLine>,
    // #[serde(rename = "AccountInformation")]
    // account_information: String,

    // #[serde(rename = "EquitySummaryInBase")]
    // equity_summary_in_base: String,

    // #[serde(rename = "OpenPositions")]
    // open_positions: String,

    // #[serde(rename = "Trades")]
    // trades: String,

    // #[serde(rename = "TradeConfirms")]
    // trade_confirms: String,

    // #[serde(rename = "TransactionTaxes")]
    // transaction_taxes: String,

    // #[serde(rename = "OptionEAE")]
    // option_eae: String,

    // #[serde(rename = "PriorPeriodPositions")]
    // prior_period_positions: String,

    // #[serde(rename = "CorporateActions")]
    // corporate_actions: String,

    // #[serde(rename = "CashTransactions")]
    // cash_transactions: String,

    // #[serde(rename = "CFDCharges")]
    // cfd_charges: String,

    // #[serde(rename = "Transfers")]
    // transfers: String,

    // #[serde(rename = "ChangeInDividendAccruals")]
    // change_in_dividend_accruals: String,

    // #[serde(rename = "OpenDividendAccruals")]
    // open_dividend_accruals: String,

    // #[serde(rename = "SecuritiesInfo")]
    // securities_info: String,

    // #[serde(rename = "ConversionRates")]
    // conversion_rates: String,
}

//#[derive(Deserialize, Debug)]
//struct StmtFunds {
//    #[serde(rename = "StmtFunds")]
//    stmt_funds: Vec<StatementOfFundsLine>,
//}

#[derive(Deserialize, Debug)]
struct StatementOfFundsLine {
    #[serde(rename = "accountId")]
    account_id: String,
    #[serde(rename = "acctAlias")]
    acct_alias: String,
    #[serde(rename = "activityCode")]
    activity_code: String,
    #[serde(rename = "activityDescription")]
    activity_description: String,
    #[serde(rename = "amount")]
    amount: String,
    #[serde(rename = "assetCategory")]
    asset_category: String,
    #[serde(rename = "balance")]
    balance: String,
    #[serde(rename = "buySell")]
    buy_sell: String,
    #[serde(rename = "conid")]
    conid: String,
    #[serde(rename = "credit")]
    credit: String,
    #[serde(rename = "currency")]
    currency: String,
    #[serde(rename = "cusip")]
    cusip: String,
    #[serde(rename = "date")]
    date: String,
    #[serde(rename = "debit")]
    debit: String,
    #[serde(rename = "description")]
    description: String,
    #[serde(rename = "expiry")]
    expiry: String,
    #[serde(rename = "fxRateToBase")]
    fx_rate_to_base: String,
    #[serde(rename = "isin")]
    isin: String,
    #[serde(rename = "issuer")]
    issuer: String,
    #[serde(rename = "levelOfDetail")]
    level_of_detail: String,
    #[serde(rename = "listingExchange")]
    listing_exchange: String,
    #[serde(rename = "model")]
    model: String,
    #[serde(rename = "multiplier")]
    multiplier: String,
    #[serde(rename = "orderID")]
    order_id: String,
    #[serde(rename = "principalAdjustFactor")]
    principal_adjust_factor: String,
    #[serde(rename = "putCall")]
    put_call: String,
    #[serde(rename = "reportDate")]
    report_date: String,
    #[serde(rename = "securityID")]
    security_id: String,
    #[serde(rename = "securityIDType")]
    security_id_type: String,
    #[serde(rename = "settleDate")]
    settle_date: String,
    #[serde(rename = "strike")]
    strike: String,
    #[serde(rename = "symbol")]
    symbol: String,
    #[serde(rename = "tradeCode")]
    trade_code: String,
    #[serde(rename = "tradeCommission")]
    trade_commission: String,
    #[serde(rename = "tradeGross")]
    trade_gross: String,
    #[serde(rename = "tradeID")]
    trade_id: String,
    #[serde(rename = "tradePrice")]
    trade_price: String,
    #[serde(rename = "tradeQuantity")]
    trade_quantity: String,
    #[serde(rename = "tradeTax")]
    trade_tax: String,
    #[serde(rename = "underlyingConid")]
    underlying_conid: String,
    #[serde(rename = "underlyingListingExchange")]
    underlying_listing_exchange: String,
    #[serde(rename = "underlyingSecurityID")]
    underlying_security_id: String,
    #[serde(rename = "underlyingSymbol")]
    underlying_symbol: String,
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
fn request_report(
    client: &reqwest::Client,
    reference_code: String,
) -> Result<String, Box<dyn Error>> {
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
    let response = request_report(&client, reference_code)?;
    let mut _dest = File::create("/home/czichy/tmp/test.xml")?;
    //info!("Response: {:#?}", response);
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
                "DEP" => "Equity:Transfers".to_string(),
                _ => "Vermögen:Kapitalvermögen:Finanzinstrumente:Interactive Brokers".to_string(),
            };
            let trans = parse_transaction(
                record_data,
                "Vermögen:Kapitalvermögen:Guthaben:Verrechnungskonto:Interactive Brokers"
                    .to_string(),
                counter_account,
            );
            //let trans = Transaction {
            //    comment: None,
            //    date: NaiveDate::parse_from_str(
            //        record_data.attribute("date").unwrap_or(&""),
            //        "%Y-%m-%d",
            //    ).unwrap(),
            //    effective_date: NaiveDate::parse_from_str(
            //        record_data.attribute("settleDate").unwrap_or(&""),
            //        "%Y-%m-%d",
            //    )
            //    .ok(),
            //    status: Some(TransactionStatus::Cleared),
            //    code: None,
            //    description: String::from(record_data.attribute("activityDescription").unwrap_or(&"")),
            //    postings: vec![
            //        Posting {
            //            account:
            //                "Vermögen:Kapitalvermögen:Guthaben:Verrechnungskonto:Interactive Brokers"
            //                    .to_string(),
            //            amount: Some(Amount {
            //                quantity: Decimal::from_str(record_data.attribute("amount").unwrap_or("")).unwrap(),
            //                commodity: Commodity {
            //                    name: record_data.attribute("currency").unwrap().to_string(),
            //                    position: CommodityPosition::Left,
            //                },
            //            }),
            //            balance: Some(Balance::Amount(Amount {
            //                quantity: Decimal::from_str(
            //                    record_data.attribute("balance").unwrap_or(""),
            //                ).unwrap(),
            //                commodity: Commodity {
            //                    name: record_data.attribute("currency").unwrap().to_string(),
            //                    position: CommodityPosition::Left,
            //                },
            //            })),
            //            status: None,
            //            comment: Some(record_data.attribute("transactionID").unwrap().to_string()),
            //        },
            //        Posting {
            //            account: counter_account,
            //            amount: Some(Amount {
            //                quantity: Decimal::from_str(record_data.attribute("amount").unwrap_or("")).unwrap()
            //                    .neg(),
            //                commodity: Commodity {
            //                    name: record_data.attribute("currency").unwrap().to_string(),
            //                    position: CommodityPosition::Left,
            //                },
            //            }),
            //            balance: None,
            //            status: None,
            //            comment: None,
            //        },
            //    ],
            //};

            //                    let funds_line :Result<
            //                        StatementOfFundsLine,
            //                        serde_xml_rs::Error,
            //                    > = serde_xml_rs::from_str(&record_data.tail().unwrap());
            //info!("{:?}", funds_line);
            trans
        })
        .for_each(|t| writeln!(_journal, "{}", t).unwrap());
    Ok(())
}
