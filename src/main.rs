#![deny(warnings)]
// use structopt::clap::{crat_authors, crate_description, crate_name,
// crate_version};
use std::{io::Write, path::PathBuf};

use clap::Parser;
mod args;
pub mod model;
mod parse;
pub use model::*;

// fn parse_transaction(
//     statement_of_funds_line: roxmltree::Node,
//     src_account: String,
//     counter_account: String,
// ) -> Transaction {
//     Transaction {
//         comment:        None,
//         date:           NaiveDate::parse_from_str(
//             statement_of_funds_line.attribute("date").unwrap_or(&""),
//             "%Y-%m-%d",
//         )
//         .unwrap(),
//         effective_date: NaiveDate::parse_from_str(
//             statement_of_funds_line
//                 .attribute("settleDate")
//                 .unwrap_or(&""),
//             "%Y-%m-%d",
//         )
//         .ok(),
//         status:         Some(TransactionStatus::Cleared),
//         code:           None,
//         description:    String::from(
//             statement_of_funds_line
//                 .attribute("activityDescription")
//                 .unwrap_or(&""),
//         ),
//         postings:       vec![
//             Posting {
//                 account: src_account,
//                 amount:  Some(Amount {
//                     quantity:  Decimal::from_str(
//
// statement_of_funds_line.attribute("amount").unwrap_or(""),
// )                     .unwrap(),
//                     commodity: Commodity {
//                         name:     statement_of_funds_line
//                             .attribute("currency")
//                             .unwrap()
//                             .to_string(),
//                         position: CommodityPosition::Left,
//                     },
//                 }),
//                 balance: Some(Balance::Amount(Amount {
//                     quantity:  Decimal::from_str(
//
// statement_of_funds_line.attribute("balance").unwrap_or(""),
// )                     .unwrap(),
//                     commodity: Commodity {
//                         name:     statement_of_funds_line
//                             .attribute("currency")
//                             .unwrap()
//                             .to_string(),
//                         position: CommodityPosition::Left,
//                     },
//                 })),
//                 status:  None,
//                 comment: Some(
//                     statement_of_funds_line
//                         .attribute("transactionID")
//                         .unwrap()
//                         .to_string(),
//                 ),
//             },
//             Posting {
//                 account: counter_account,
//                 amount:  Some(Amount {
//                     quantity:  Decimal::from_str(
//
// statement_of_funds_line.attribute("amount").unwrap_or(""),
// )                     .unwrap()
//                     .neg(),
//                     commodity: Commodity {
//                         name:     statement_of_funds_line
//                             .attribute("currency")
//                             .unwrap()
//                             .to_string(),
//                         position: CommodityPosition::Left,
//                     },
//                 }),
//                 balance: None,
//                 status:  None,
//                 comment: None,
//             },
//         ],
//     }
// }

#[tokio::main]
pub async fn main() -> std::result::Result<(), Box<dyn std::error::Error>> {
    let opt = args::OptArgs::parse();
    // Initialize logger
    env_logger::Builder::new()
        .filter_level(opt.verbose.log_level_filter())
        .init();
let mut transactions: Vec<Transaction> = vec![];
    let response = ibkr_rust_flex::flex_statement_from_file(opt.flex_file).await;
    {
        if let Ok(response) = response {
            let account_id = &response.account_id;
            if let Some(ref statements) = response.statement_of_funds {
                let mut statement_of_funds_lines = statements
                    .items
                    .iter()
                    .filter_map(|item| {
                        if item.activity_code == None                         
                    || item.activity_code == Some("BUY".into())
                    || item.activity_code == Some("SELL".into())
                    || item.activity_code == Some("FOREX".into())
                    || item.activity_code == Some("CORP".into())
                    // TODO: do we need this?
                    || item.activity_code == Some("ADJ".into())
                        {
                            return None;
                        }
                        parse::parse_transaction(
                            item,
                            format!(
                        "{}:{}",
                        "Vermögen:Kapitalvermögen:Guthaben:Verrechnungskonto:Interactive Brokers",
                        account_id
                    ),
                        )
                    })
                    .collect::<Vec<_>>();
                transactions.append(&mut statement_of_funds_lines);
            }
            if let Some(ref statements) = response.trades {
                let mut trade_lines = statements
                    .items
                    .iter()
                    .filter_map(|item| {
                        if let ibkr_rust_flex::trades::TradeElements::Trade(trade) = item {
                            if trade.contract.asset_category == ibkr_rust_flex::enums::AssetCategory::CASH 
                            {
                                Some(parse::parse_fx_trade(trade, 
                                format!(        "{}:{}",
        "Vermögen:Kapitalvermögen:Guthaben:Verrechnungskonto:Interactive Brokers", account_id
    ),
format!(        "{}:{}",
        "Vermögen:Kapitalvermögen:Finanzinstrumente:Interactive Brokers", account_id
    )                                ))
                            } else {
                                Some(parse::parse_trade_pnl(trade, account_id))
                            }
                        } else {
                            None
                        }
                    })
                    .collect::<Vec<_>>();
                transactions.append(&mut trade_lines);
            }

if let Some(ref statements) = response.corporate_actions{
                let mut actions = statements
                    .items
                    .iter()
                    .map(|action| {
                                parse::parse_corporate_action(action, account_id)
                    })
                    .collect::<Vec<_>>();
                transactions.append(&mut actions);
            }        }
    }
    transactions.iter().for_each(|t| println!("{t}"));
    if let Some(path) = opt.journal_file {
        let file_name: PathBuf = PathBuf::from(&path);
        tracing::error!("{:?}", &file_name);
        let f = std::fs::File::create(file_name)?;

        for t in transactions.iter() {
            writeln!(&f, "{t}").unwrap();
        }
    }



    Ok(())
}

#[cfg(test)]
mod tests {
    use ibkr_rust_flex::flex_statements_from_str;
    use tracing_bunyan_formatter::{BunyanFormattingLayer, JsonStorageLayer};
    use tracing_log::LogTracer;
    use tracing_subscriber::{layer::SubscriberExt, registry::Registry, EnvFilter};

    #[ctor::ctor]
    fn init() {
        LogTracer::init().expect("Unable to setup log tracer!");
        let env_filter =
            EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("debug"));
        let app_name = concat!(env!("CARGO_PKG_NAME"), "-", env!("CARGO_PKG_VERSION")).to_string();
        ////let (non_blocking_writer, _guard) =
        ////let tracing_appender::non_blocking(std::io::stdout());
        // let bunyan_formatting_layer = BunyanFormattingLayer::new(app_name,
        // std::io::stdout);//non_blocking_writer);
        let formatting_layer = BunyanFormattingLayer::new(app_name, std::io::stdout);
        let subscriber = Registry::default()
            .with(env_filter)
            .with(JsonStorageLayer)
            .with(formatting_layer);
        tracing::subscriber::set_global_default(subscriber).unwrap();
    }
    #[test]
    fn flex_map_empty_ok() {
        let xml = r#"
<FlexQueryResponse queryName="Cash-Flow LongTerm" type="AF">
<FlexStatements count="1">
<FlexStatement accountId="U11213636" fromDate="2024-01-01" toDate="2024-12-31" period="LastBusinessDay" whenGenerated="2022-02-2204:07:52">
<StmtFunds>
</StmtFunds>
<CommissionCredits> </CommissionCredits>
<Trades> </Trades>
<TransactionTaxes> </TransactionTaxes>
<RoutingCommissions> </RoutingCommissions>
<UnbundledCommissionDetails> </UnbundledCommissionDetails>
<InterestAccruals> </InterestAccruals>
<HardToBorrowDetails> </HardToBorrowDetails>
<SLBFees> </SLBFees>
</FlexStatement>
</FlexStatements>
</FlexQueryResponse>
"#;
        let statement = flex_statements_from_str(xml).unwrap();
        // assert_eq!(response.query_name, "Trading EOD");
        // assert_eq!(response.query_type, "AF");
        // let statement = &response.flex_statements.statements[0];
        // assert_eq!(statement.account_id, "U7502027");
        // assert_eq!(
        //     statement.from_date,
        //     NaiveDate::from_str("2022-02-21").unwrap()
        // );
        // assert_eq!(
        //     statement.to_date,
        //     NaiveDate::from_str("2022-02-21").unwrap()
        // );
    }

    #[test]
    fn flex_map_ok() {
        let xml = r#"
<FlexQueryResponse queryName="Cash-Flow LongTerm" type="AF">
<FlexStatements count="1">
<FlexStatement accountId="U11213636" fromDate="2024-01-01" toDate="2024-12-31" period="LastBusinessDay" whenGenerated="2022-02-2204:07:52">
<StmtFunds>
<StatementOfFundsLine accountId="U11213636" acctAlias="" model="" currency="EUR" fxRateToBase="1" assetCategory="OPT" symbol="P ODXS 20240119 16400 M" description="DAX 19JAN24 16400 P" conid="664151922" securityID="" securityIDType="" cusip="" isin="" listingExchange="EUREX" underlyingConid="825711" underlyingSymbol="DAX" underlyingSecurityID="" underlyingListingExchange="" issuer="" multiplier="1" strike="16400" expiry="2024-01-19" putCall="P" principalAdjustFactor="" reportDate="2024-01-04" date="2024-01-04" settleDate="2024-01-05" activityCode="BUY" activityDescription="Buy 3 DAX 19JAN24 16400 P " tradeID="" orderID="" buySell="BUY" tradeQuantity="3" tradePrice="0" tradeGross="-285" tradeCommission="-5.1" tradeTax="0" debit="-290.1" credit="" amount="-290.1" tradeCode="" balance="709.561795778" levelOfDetail="Currency" transactionID="2389106807" />
<StatementOfFundsLine accountId="U11213636" acctAlias="" model="" currency="EUR" fxRateToBase="1" assetCategory="OPT" symbol="P ODXS 20240119 16400 M" description="DAX 19JAN24 16400 P" conid="664151922" securityID="" securityIDType="" cusip="" isin="" listingExchange="EUREX" underlyingConid="825711" underlyingSymbol="DAX" underlyingSecurityID="" underlyingListingExchange="" issuer="" multiplier="1" strike="16400" expiry="2024-01-19" putCall="P" principalAdjustFactor="" reportDate="2024-01-17" date="2024-01-17" settleDate="2024-01-18" activityCode="SELL" activityDescription="Sell -3 DAX 19JAN24 16400 P " tradeID="" orderID="" buySell="SELL" tradeQuantity="-3" tradePrice="0" tradeGross="300" tradeCommission="-5.1" tradeTax="0" debit="" credit="294.9" amount="294.9" tradeCode="" balance="1004.461795778" levelOfDetail="Currency" transactionID="2418450284" />
<StatementOfFundsLine accountId="U11213636" acctAlias="" model="" currency="EUR" fxRateToBase="1" assetCategory="CASH" symbol="" description="" conid="" securityID="" securityIDType="" cusip="" isin="" listingExchange="" underlyingConid="" underlyingSymbol="" underlyingSecurityID="" underlyingListingExchange="" issuer="" multiplier="0" strike="" expiry="" putCall="" principalAdjustFactor="" reportDate="2024-02-01" date="2024-02-01" settleDate="2024-02-01" activityCode="WITH" activityDescription="Cash Transfer" tradeID="" orderID="" buySell="" tradeQuantity="0" tradePrice="0" tradeGross="0" tradeCommission="0" tradeTax="0" debit="-237" credit="" amount="-237" tradeCode="" balance="650.461795778" levelOfDetail="Currency" transactionID="2456676134" />
<StatementOfFundsLine accountId="U11213636" acctAlias="" model="" currency="EUR" fxRateToBase="1" assetCategory="OPT" symbol="P ODXS 20240216 16850 M" description="DAX 16FEB24 16850 P" conid="667286019" securityID="" securityIDType="" cusip="" isin="" listingExchange="EUREX" underlyingConid="825711" underlyingSymbol="DAX" underlyingSecurityID="" underlyingListingExchange="" issuer="" multiplier="1" strike="16850" expiry="2024-02-16" putCall="P" principalAdjustFactor="" reportDate="2024-02-02" date="2024-02-02" settleDate="2024-02-05" activityCode="BUY" activityDescription="Buy 3 DAX 16FEB24 16850 P " tradeID="" orderID="" buySell="BUY" tradeQuantity="3" tradePrice="0" tradeGross="-270" tradeCommission="-5.1" tradeTax="0" debit="-275.1" credit="" amount="-275.1" tradeCode="" balance="375.361795778" levelOfDetail="Currency" transactionID="2460511198" />
<StatementOfFundsLine accountId="U11213636" acctAlias="" model="" currency="EUR" fxRateToBase="1" assetCategory="BOND" symbol="DBR 1 3/4 02/15/24" description="DBR 1 3/4 02/15/24" conid="142870711" securityID="DE0001102333" securityIDType="ISIN" cusip="" isin="DE0001102333" listingExchange="" underlyingConid="" underlyingSymbol="" underlyingSecurityID="" underlyingListingExchange="" issuer="" multiplier="1" strike="" expiry="" putCall="" principalAdjustFactor="1" reportDate="2024-02-15" date="2024-02-15" settleDate="2024-02-15" activityCode="INTR" activityDescription="Bond Coupon Payment (DBR 1 3/4 02/15/24 - BUNDESREPUB. DEUTSCHLAND DBR 1 3/4 02/15/24)" tradeID="" orderID="" buySell="" tradeQuantity="0" tradePrice="0" tradeGross="0" tradeCommission="0" tradeTax="0" debit="" credit="262.5" amount="262.5" tradeCode="" balance="939.361795778" levelOfDetail="Currency" transactionID="2494867565" />
<StatementOfFundsLine accountId="U11213636" acctAlias="" model="" currency="EUR" fxRateToBase="1" assetCategory="BOND" symbol="DBR 1 3/4 02/15/24" description="DBR 1 3/4 02/15/24" conid="142870711" securityID="DE0001102333" securityIDType="ISIN" cusip="" isin="DE0001102333" listingExchange="" underlyingConid="" underlyingSymbol="" underlyingSecurityID="" underlyingListingExchange="" issuer="" multiplier="1" strike="" expiry="" putCall="" principalAdjustFactor="1" reportDate="2024-02-15" date="2024-02-14" settleDate="2024-02-15" activityCode="CORP" activityDescription="(DE0001102333)  Bond Maturity FOR EUR 1.00 PER BOND (DBR 1 3/4 02/15/24, DBR 1 3/4 02/15/24, DE0001102333)" tradeID="" orderID="" buySell="" tradeQuantity="0" tradePrice="0" tradeGross="0" tradeCommission="0" tradeTax="0" debit="" credit="15000" amount="15000" tradeCode="" balance="15939.361795778" levelOfDetail="Currency" transactionID="2495820369" />
<StatementOfFundsLine accountId="U11213636" acctAlias="" model="" currency="EUR" fxRateToBase="1" assetCategory="STK" symbol="XEOD" description="X EUR OVERNIGHT RATE SWAP 1D" conid="74992041" securityID="LU0335044896" securityIDType="ISIN" cusip="" isin="LU0335044896" listingExchange="IBIS2" underlyingConid="" underlyingSymbol="" underlyingSecurityID="" underlyingListingExchange="" issuer="" multiplier="1" strike="" expiry="" putCall="" principalAdjustFactor="" reportDate="2024-02-16" date="2024-02-16" settleDate="2024-02-20" activityCode="BUY" activityDescription="Buy 100 X EUR OVERNIGHT RATE SWAP 1D " tradeID="" orderID="" buySell="BUY" tradeQuantity="100" tradePrice="127.407925" tradeGross="-12761.5" tradeCommission="-6.809118" tradeTax="0" debit="-12768.309118" credit="" amount="-12768.309118" tradeCode="" balance="3171.052677778" levelOfDetail="Currency" transactionID="2498606814" />
<StatementOfFundsLine accountId="U11213636" acctAlias="" model="" currency="EUR" fxRateToBase="1" assetCategory="" symbol="" description="" conid="" securityID="" securityIDType="" cusip="" isin="" listingExchange="" underlyingConid="" underlyingSymbol="" underlyingSecurityID="" underlyingListingExchange="" issuer="" multiplier="0" strike="" expiry="" putCall="" principalAdjustFactor="" reportDate="2024-03-05" date="2024-03-05" settleDate="2024-03-05" activityCode="CINT" activityDescription="EUR Credit Interest for Feb-2024" tradeID="" orderID="" buySell="" tradeQuantity="0" tradePrice="0" tradeGross="0" tradeCommission="0" tradeTax="0" debit="" credit="0.9" amount="0.9" tradeCode="" balance="704.526137778" levelOfDetail="Currency" transactionID="2542582757" />
<StatementOfFundsLine accountId="U11213636" acctAlias="" model="" currency="EUR" fxRateToBase="1" assetCategory="" symbol="" description="" conid="" securityID="" securityIDType="" cusip="" isin="" listingExchange="" underlyingConid="" underlyingSymbol="" underlyingSecurityID="" underlyingListingExchange="" issuer="" multiplier="0" strike="" expiry="" putCall="" principalAdjustFactor="" reportDate="2024-03-05" date="2024-03-05" settleDate="2024-03-05" activityCode="FRTAX" activityDescription="Withholding @ 20% on Credit Interest for Feb-2024" tradeID="" orderID="" buySell="" tradeQuantity="0" tradePrice="0" tradeGross="0" tradeCommission="0" tradeTax="0" debit="-0.18" credit="" amount="-0.18" tradeCode="" balance="704.346137778" levelOfDetail="Currency" transactionID="2543416777" />
<StatementOfFundsLine accountId="U11213636" acctAlias="" model="" currency="EUR" fxRateToBase="1" assetCategory="STK" symbol="XEOD" description="X EUR OVERNIGHT RATE SWAP 1D" conid="74992041" securityID="LU0335044896" securityIDType="ISIN" cusip="" isin="LU0335044896" listingExchange="IBIS2" underlyingConid="" underlyingSymbol="" underlyingSecurityID="" underlyingListingExchange="" issuer="" multiplier="1" strike="" expiry="" putCall="" principalAdjustFactor="" reportDate="2024-03-07" date="2024-03-07" settleDate="2024-03-07" activityCode="DIV" activityDescription="XEOD(LU0335044896) Cash Dividend EUR 1.201 per Share (Mixed Income)" tradeID="" orderID="" buySell="" tradeQuantity="0" tradePrice="0" tradeGross="0" tradeCommission="0" tradeTax="0" debit="" credit="120.1" amount="120.1" tradeCode="" balance="824.446137778" levelOfDetail="Currency" transactionID="2549899814" />
<StatementOfFundsLine accountId="U11213636" acctAlias="" model="" currency="EUR" fxRateToBase="1" assetCategory="" symbol="" description="" conid="" securityID="" securityIDType="" cusip="" isin="" listingExchange="" underlyingConid="" underlyingSymbol="" underlyingSecurityID="" underlyingListingExchange="" issuer="" multiplier="0" strike="" expiry="" putCall="" principalAdjustFactor="" reportDate="2024-04-12" date="2024-04-12" settleDate="2024-04-12" activityCode="DEP" activityDescription="Electronic Fund Transfer" tradeID="" orderID="" buySell="" tradeQuantity="0" tradePrice="0" tradeGross="0" tradeCommission="0" tradeTax="0" debit="" credit="3000" amount="3000" tradeCode="" balance="3824.446137778" levelOfDetail="Currency" transactionID="2643267973" />
<StatementOfFundsLine accountId="U11213636" acctAlias="" model="" currency="EUR" fxRateToBase="1" assetCategory="" symbol="" description="" conid="" securityID="" securityIDType="" cusip="" isin="" listingExchange="" underlyingConid="" underlyingSymbol="" underlyingSecurityID="" underlyingListingExchange="" issuer="" multiplier="" strike="" expiry="" putCall="" principalAdjustFactor="" reportDate="2024-04-12" date="2024-04-12" settleDate="2024-04-16" activityCode="FOREX" activityDescription="Traded Currency Leg from Forex Trade" tradeID="719613135" orderID="" buySell="" tradeQuantity="0" tradePrice="0" tradeGross="0" tradeCommission="0" tradeTax="0" debit="-3000" credit="" amount="-3000" tradeCode="" balance="824.446137778" levelOfDetail="Currency" transactionID="2643562096" />
<StatementOfFundsLine accountId="U11213636" acctAlias="" model="" currency="EUR" fxRateToBase="1" assetCategory="" symbol="" description="" conid="" securityID="" securityIDType="" cusip="" isin="" listingExchange="" underlyingConid="" underlyingSymbol="" underlyingSecurityID="" underlyingListingExchange="" issuer="" multiplier="" strike="" expiry="" putCall="" principalAdjustFactor="" reportDate="2024-04-12" date="2024-04-12" settleDate="2024-04-12" activityCode="FOREX" activityDescription="Commission from Forex Trade" tradeID="719613135" orderID="" buySell="" tradeQuantity="0" tradePrice="0" tradeGross="0" tradeCommission="0" tradeTax="0" debit="-1.8646" credit="" amount="-1.8646" tradeCode="" balance="822.581537778" levelOfDetail="Currency" transactionID="2643566782" />
<StatementOfFundsLine accountId="U11213636" acctAlias="" model="" currency="EUR" fxRateToBase="1" assetCategory="OPT" symbol="P ODAX 20240510 17700 W" description="DAX 10MAY24 17700 P" conid="694533828" securityID="" securityIDType="" cusip="" isin="" listingExchange="EUREX" underlyingConid="825711" underlyingSymbol="DAX" underlyingSecurityID="" underlyingListingExchange="" issuer="" multiplier="5" strike="17700" expiry="2024-05-10" putCall="P" principalAdjustFactor="" reportDate="2024-05-02" date="2024-05-02" settleDate="2024-05-03" activityCode="BUY" activityDescription="Buy 1 DAX 10MAY24 17700 P " tradeID="" orderID="" buySell="BUY" tradeQuantity="1" tradePrice="0" tradeGross="-220" tradeCommission="-1.7" tradeTax="0" debit="-221.7" credit="" amount="-221.7" tradeCode="" balance="754.981537778" levelOfDetail="Currency" transactionID="2694770117" />
<StatementOfFundsLine accountId="U11213636" acctAlias="" model="" currency="EUR" fxRateToBase="1" assetCategory="OPT" symbol="P ODAX 20240510 17700 W" description="DAX 10MAY24 17700 P" conid="694533828" securityID="" securityIDType="" cusip="" isin="" listingExchange="EUREX" underlyingConid="825711" underlyingSymbol="DAX" underlyingSecurityID="" underlyingListingExchange="" issuer="" multiplier="5" strike="17700" expiry="2024-05-10" putCall="P" principalAdjustFactor="" reportDate="2024-05-10" date="2024-05-10" settleDate="2024-05-13" activityCode="SELL" activityDescription="Sell -1 DAX 10MAY24 17700 P " tradeID="" orderID="" buySell="SELL" tradeQuantity="-1" tradePrice="0" tradeGross="0" tradeCommission="0" tradeTax="0" debit="" credit="" amount="0" tradeCode="" balance="754.981537778" levelOfDetail="Currency" transactionID="2718063900" />
<StatementOfFundsLine accountId="U11213636" acctAlias="" model="" currency="EUR" fxRateToBase="1" assetCategory="" symbol="" description="" conid="" securityID="" securityIDType="" cusip="" isin="" listingExchange="" underlyingConid="" underlyingSymbol="" underlyingSecurityID="" underlyingListingExchange="" issuer="" multiplier="0" strike="" expiry="" putCall="" principalAdjustFactor="" reportDate="2024-05-14" date="2024-05-14" settleDate="2024-05-14" activityCode="DEP" activityDescription="Electronic Fund Transfer" tradeID="" orderID="" buySell="" tradeQuantity="0" tradePrice="0" tradeGross="0" tradeCommission="0" tradeTax="0" debit="" credit="1000" amount="1000" tradeCode="" balance="1754.981537778" levelOfDetail="Currency" transactionID="2723673876" />
<StatementOfFundsLine accountId="U11213636" acctAlias="" model="" currency="EUR" fxRateToBase="1" assetCategory="OPT" symbol="C ODAP 20240531 18900 E" description="DAX 31MAY24 18900 C" conid="688228941" securityID="" securityIDType="" cusip="" isin="" listingExchange="EUREX" underlyingConid="825711" underlyingSymbol="DAX" underlyingSecurityID="" underlyingListingExchange="" issuer="" multiplier="5" strike="18900" expiry="2024-05-31" putCall="C" principalAdjustFactor="" reportDate="2024-05-16" date="2024-05-16" settleDate="2024-05-17" activityCode="BUY" activityDescription="Buy 2 DAX 31MAY24 18900 C " tradeID="" orderID="" buySell="BUY" tradeQuantity="2" tradePrice="0" tradeGross="-1250" tradeCommission="-3.4" tradeTax="0" debit="-1253.4" credit="" amount="-1253.4" tradeCode="" balance="313.181537778" levelOfDetail="Currency" transactionID="2731857901" />
<StatementOfFundsLine accountId="U11213636" acctAlias="" model="" currency="EUR" fxRateToBase="1" assetCategory="OPT" symbol="C ODAP 20240531 18900 E" description="DAX 31MAY24 18900 C" conid="688228941" securityID="" securityIDType="" cusip="" isin="" listingExchange="EUREX" underlyingConid="825711" underlyingSymbol="DAX" underlyingSecurityID="" underlyingListingExchange="" issuer="" multiplier="5" strike="18900" expiry="2024-05-31" putCall="C" principalAdjustFactor="" reportDate="2024-05-17" date="2024-05-17" settleDate="2024-05-20" activityCode="BUY" activityDescription="Buy 1 DAX 31MAY24 18900 C " tradeID="" orderID="" buySell="BUY" tradeQuantity="1" tradePrice="0" tradeGross="-325" tradeCommission="-1.7" tradeTax="0" debit="-326.7" credit="" amount="-326.7" tradeCode="" balance="-13.518462222" levelOfDetail="Currency" transactionID="2735826881" />
<StatementOfFundsLine accountId="U11213636" acctAlias="" model="" currency="EUR" fxRateToBase="1" assetCategory="" symbol="" description="" conid="" securityID="" securityIDType="" cusip="" isin="" listingExchange="" underlyingConid="" underlyingSymbol="" underlyingSecurityID="" underlyingListingExchange="" issuer="" multiplier="" strike="" expiry="" putCall="" principalAdjustFactor="" reportDate="2024-05-17" date="2024-05-17" settleDate="2024-05-20" activityCode="FOREX" activityDescription="Traded Currency Leg from Forex Trade" tradeID="739950134" orderID="" buySell="" tradeQuantity="0" tradePrice="0" tradeGross="0" tradeCommission="0" tradeTax="0" debit="" credit="13.52" amount="13.52" tradeCode="" balance="0.001537778" levelOfDetail="Currency" transactionID="2735829644" />
<StatementOfFundsLine accountId="U11213636" acctAlias="" model="" currency="EUR" fxRateToBase="1" assetCategory="OPT" symbol="C ODAP 20240531 18900 E" description="DAX 31MAY24 18900 C" conid="688228941" securityID="" securityIDType="" cusip="" isin="" listingExchange="EUREX" underlyingConid="825711" underlyingSymbol="DAX" underlyingSecurityID="" underlyingListingExchange="" issuer="" multiplier="5" strike="18900" expiry="2024-05-31" putCall="C" principalAdjustFactor="" reportDate="2024-05-17" date="2024-05-17" settleDate="2024-05-20" activityCode="SELL" activityDescription="Sell -1 DAX 31MAY24 18900 C " tradeID="" orderID="" buySell="SELL" tradeQuantity="-1" tradePrice="0" tradeGross="375" tradeCommission="-1.7" tradeTax="0" debit="" credit="373.3" amount="373.3" tradeCode="" balance="373.301537778" levelOfDetail="Currency" transactionID="2736225201" />
<StatementOfFundsLine accountId="U11213636" acctAlias="" model="" currency="EUR" fxRateToBase="1" assetCategory="OPT" symbol="C ODAP 20240531 18900 E" description="DAX 31MAY24 18900 C" conid="688228941" securityID="" securityIDType="" cusip="" isin="" listingExchange="EUREX" underlyingConid="825711" underlyingSymbol="DAX" underlyingSecurityID="" underlyingListingExchange="" issuer="" multiplier="5" strike="18900" expiry="2024-05-31" putCall="C" principalAdjustFactor="" reportDate="2024-05-23" date="2024-05-23" settleDate="2024-05-24" activityCode="SELL" activityDescription="Sell -2 DAX 31MAY24 18900 C " tradeID="" orderID="" buySell="SELL" tradeQuantity="-2" tradePrice="0" tradeGross="225" tradeCommission="-3.4" tradeTax="0" debit="" credit="221.6" amount="221.6" tradeCode="" balance="594.901537778" levelOfDetail="Currency" transactionID="2751415648" />
<StatementOfFundsLine accountId="U11213636" acctAlias="" model="" currency="EUR" fxRateToBase="1" assetCategory="" symbol="" description="" conid="" securityID="" securityIDType="" cusip="" isin="" listingExchange="" underlyingConid="" underlyingSymbol="" underlyingSecurityID="" underlyingListingExchange="" issuer="" multiplier="" strike="" expiry="" putCall="" principalAdjustFactor="" reportDate="2024-05-23" date="2024-05-23" settleDate="2024-05-24" activityCode="FOREX" activityDescription="Traded Currency Leg from Forex Trade" tradeID="743824496" orderID="" buySell="" tradeQuantity="0" tradePrice="0" tradeGross="0" tradeCommission="0" tradeTax="0" debit="-18.63" credit="" amount="-18.63" tradeCode="" balance="576.271537778" levelOfDetail="Currency" transactionID="2752267209" />
<StatementOfFundsLine accountId="U11213636" acctAlias="" model="" currency="EUR" fxRateToBase="1" assetCategory="STK" symbol="XEOD" description="X EUR OVERNIGHT RATE SWAP 1D" conid="74992041" securityID="LU0335044896" securityIDType="ISIN" cusip="" isin="LU0335044896" listingExchange="IBIS2" underlyingConid="" underlyingSymbol="" underlyingSecurityID="" underlyingListingExchange="" issuer="" multiplier="1" strike="" expiry="" putCall="" principalAdjustFactor="" reportDate="2024-06-07" date="2024-06-07" settleDate="2024-06-07" activityCode="DIV" activityDescription="XEOD(LU0335044896) Cash Dividend EUR 1.204 per Share (Mixed Income)" tradeID="" orderID="" buySell="" tradeQuantity="0" tradePrice="0" tradeGross="0" tradeCommission="0" tradeTax="0" debit="" credit="120.4" amount="120.4" tradeCode="" balance="574.871537778" levelOfDetail="Currency" transactionID="2792456062" />
</StmtFunds>
<CommissionCredits>
</CommissionCredits>
<Trades>
<Trade accountId="U7502027" acctAlias="" model="" currency="USD" fxRateToBase="1" assetCategory="STK" symbol="GOOGL" description="ALPHABET INC-CL A" conid="208813719" securityID="US02079K3059" securityIDType="ISIN" cusip="02079K305" isin="US02079K3059" listingExchange="NASDAQ" underlyingConid="" underlyingSymbol="" underlyingSecurityID="" underlyingListingExchange="" issuer="" multiplier="1" strike="" expiry="" tradeID="596715684" putCall="" reportDate="2023-08-29" principalAdjustFactor="" dateTime="2023-08-2909:45:54" tradeDate="2023-08-29" settleDateTarget="2023-08-31" transactionType="ExchTrade" exchange="DRCTEDGE" quantity="60" tradePrice="132.75" tradeMoney="7965" proceeds="-7965" taxes="0" ibCommission="-0.31425725" ibCommissionCurrency="USD" netCash="-7965.31425725" closePrice="134.57" openCloseIndicator="O" notes="" cost="7965.31425725" fifoPnlRealized="0" fxPnl="0" mtmPnl="109.2" origTradePrice="0" origTradeDate="" origTradeID="" origOrderID="0" clearingFirmID="" transactionID="2101225667" buySell="BUY" ibOrderID="507031561" ibExecID="0000f62c.64ed9adf.01.01" brokerageOrderID="0004f96a.00014c43.64ed853a.0001" orderReference="" volatilityOrderLink="2083705967.0" exchOrderId="N/A" extExecID="940370002911B" orderTime="2023-08-2909:45:54" openDateTime="" holdingPeriodDateTime="" whenRealized="" whenReopened="" levelOfDetail="EXECUTION" changeInPrice="0" changeInQuantity="0" orderType="MIDPX" traderID="" isAPIOrder="N" accruedInt="0" serialNumber="" deliveryType="" commodityType="" fineness="0.0" weight="0.0 ()"/>
</Trades><TransactionTaxes>
</TransactionTaxes><RoutingCommissions>
</RoutingCommissions><UnbundledCommissionDetails>
</UnbundledCommissionDetails><InterestAccruals>
<InterestAccrualsCurrency accountId="U7502027" acctAlias="" model="" currency="BASE_SUMMARY" fromDate="2023-07-25" toDate="2023-07-25" startingAccrualBalance="35.66" interestAccrued="0.3" accrualReversal="0" fxTranslation="0" endingAccrualBalance="35.96" />
</InterestAccruals>
<HardToBorrowDetails>
</HardToBorrowDetails><SLBFees>
</SLBFees>
</FlexStatement>
</FlexStatements>
</FlexQueryResponse>
        "#;
        let statement = flex_statements_from_str(xml).unwrap();
    }
}
