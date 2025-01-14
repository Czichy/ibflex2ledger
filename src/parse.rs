use std::ops::Neg;

use ibkr_rust_flex::{corporate_actions::CorporateAction,
                     enums::MultiDate,
                     stmt_funds::StatementOfFundsLine,
                     trades::Trade};
use iso_currency::Currency;
use rust_decimal::Decimal;

use crate::{Amount,
            Balance,
            Commodity,
            CommodityPosition,
            Posting,
            Transaction,
            TransactionStatus};

pub(crate) fn parse_transaction(
    statement_of_funds_line: &StatementOfFundsLine,
    src_account: String,
) -> Option<Transaction> {
    statement_of_funds_line
        .activity_code
        .as_ref()
        .map(|code| {
            match code.as_str() {
                "OFEE" | "MFEE" | "SCOM" => {
                    "Ausgaben:Kapitalvermögen:Laufende Ausgaben:Depotspesen".to_owned()
                },
                "DINT" => "Ausgaben:Kapitalvermögen:Laufende Ausgaben:Zinsen".to_owned(),
                "TTAX" | "STAX" | "FRTAX" => {
                    "Ausgaben:Kapitalvermögen:Laufende Ausgaben:Steuern".to_owned()
                },
                "DIV" => "Einnahmen:Kapitalvermögen:Laufende Einnahmen:Dividenden".to_owned(),
                "INTR" | "INTP" => {
                    "Einnahmen:Kapitalvermögen:Laufende Einnahmen:Zinsen aus Finanzinstrumenten"
                        .to_owned()
                },
                "DEP" | "WITH" => {
                    let var_name = "Eigenkapital:Transfers";
                    var_name.to_owned()
                },
                _ => {
                    format!(
                        "{}:{}",
                        "Vermögen:Kapitalvermögen:Finanzinstrumente:Interactive Brokers",
                        src_account
                    )
                },
            }
        })
        .map(|counter_account| {
            Transaction {
                comment: None,
                date:    statement_of_funds_line.date,

                effective_date: {
                    match statement_of_funds_line.settle_date {
                        Some(MultiDate::Date(dt)) => Some(dt),
                        Some(_) => Some(statement_of_funds_line.date),
                        None => None,
                    }
                },
                status:         Some(TransactionStatus::Cleared),
                code:           None,
                description:    statement_of_funds_line.activity_description.clone(),
                postings:       vec![
                    Posting {
                        account:   src_account,
                        amount:    Some(Amount {
                            quantity: statement_of_funds_line.amount.unwrap_or_default(),

                            commodity: Commodity {
                                name:     statement_of_funds_line.currency.code().to_owned(),
                                position: CommodityPosition::Left,
                            },
                        }),
                        balance:   Some(Balance::Amount(Amount {
                            quantity: statement_of_funds_line.balance.unwrap_or_default(),

                            commodity: Commodity {
                                name:     statement_of_funds_line.currency.code().to_owned(),
                                position: CommodityPosition::Left,
                            },
                        })),
                        lot_price: None,
                        price:     None,
                        status:    None,
                        comment:   None, /* Some(
                                          * statement_of_funds_line.transaction_id.clone()
                                          * ,
                                          * ) */
                    },
                    Posting {
                        account:   counter_account,
                        amount:    Some(Amount {
                            quantity:  statement_of_funds_line.amount.unwrap_or_default().neg(),
                            commodity: Commodity {
                                name:     statement_of_funds_line.currency.code().to_owned(),
                                position: CommodityPosition::Left,
                            },
                        }),
                        lot_price: None,
                        price:     None,
                        balance:   None,
                        status:    None,
                        comment:   None,
                    },
                ],
            }
        })
}
#[allow(clippy::too_many_lines)]
pub(crate) fn parse_trade_pnl(trade_line: &Trade, account_id: &String) -> Transaction {
    let cash_account = format!(
        "{}:{}",
        "Vermögen:Kapitalvermögen:Guthaben:Verrechnungskonto:Interactive Brokers", account_id
    );
    let settlement_account = format!(
        "{}:{}",
        "Vermögen:Kapitalvermögen:Finanzinstrumente:Interactive Brokers", account_id
    );

    let pnl_account = format!(
        "{}:{}",
        "Einnahmen:Kapitalvermögen:Einnahmen aus Veräußerungen:Veräußerungserlöse aus \
         Finanzinstrumenten:Interactive Brokers",
        account_id
    );

    let fee_account = pnl_account.clone();
    // let fee_account = format!(
    //     "{}:{}",
    //     "Ausgaben:Kapitalvermögen:Laufende Ausgaben:Depotspesen", account_id
    // );
    let mut postings = vec![
        Posting {
            account:   cash_account.clone(),
            amount:    Some(Amount {
                quantity:  trade_line.proceeds,
                commodity: Commodity {
                    name:     trade_line.currency.code().to_owned(),
                    position: CommodityPosition::Left,
                },
            }),
            lot_price: None,
            price:     None,
            balance:   None,
            status:    None,
            comment:   Some(trade_line.transaction_id.clone()),
        },
        Posting {
            account:   settlement_account.clone(),
            amount:    Some(Amount {
                quantity:  trade_line.quantity
                    * trade_line.contract.multiplier.unwrap_or(Decimal::ONE),
                commodity: Commodity {
                    name:     format!("\"{}\"", trade_line.contract.symbol),
                    position: CommodityPosition::Right,
                },
            }),
            lot_price: if trade_line.open_close_indicator
                == Some(ibkr_rust_flex::enums::OpenClose::C)
            {
                Some(Amount {
                    quantity:  trade_line.cost.abs(), // proceeds
                    // - trade_line.fifo_pnl_realized / trade_line.quantity.abs(),
                    commodity: Commodity {
                        name:     trade_line.currency.code().to_owned(),
                        position: CommodityPosition::Right,
                    },
                })
            } else {
                None
            },
            price:     match trade_line.contract.asset_category {
                ibkr_rust_flex::enums::AssetCategory::BOND => {
                    Some(Amount {
                        quantity:  trade_line.trade_price / Decimal::new(100, 0),
                        commodity: Commodity {
                            name:     trade_line.currency.code().to_owned(),
                            position: CommodityPosition::Right,
                        },
                    })
                },
                _ => {
                    Some(Amount {
                        quantity:  trade_line.trade_price,
                        commodity: Commodity {
                            name:     trade_line.currency.code().to_owned(),
                            position: CommodityPosition::Right,
                        },
                    })
                },
            },
            balance:   None,
            status:    None,
            comment:   None,
        },
        Posting {
            account:   cash_account.clone(),
            amount:    Some(Amount {
                quantity:  trade_line.ib_commission,
                commodity: Commodity {
                    name:     trade_line
                        .ib_commission_currency
                        .unwrap_or(Currency::EUR)
                        .code()
                        .to_owned(),
                    position: CommodityPosition::Left,
                },
            }),
            lot_price: None,
            price:     None,
            balance:   None,
            status:    None,
            comment:   Some(trade_line.transaction_id.clone()),
        },
        Posting {
            account:   fee_account,
            amount:    Some(Amount {
                quantity:  trade_line.ib_commission.neg(),
                commodity: Commodity {
                    name:     trade_line
                        .ib_commission_currency
                        .unwrap_or(Currency::EUR)
                        .code()
                        .to_owned(),
                    position: CommodityPosition::Left,
                },
            }),
            lot_price: None,
            price:     None,
            balance:   None,
            status:    None,
            comment:   None,
        },
    ];
    if !trade_line.fifo_pnl_realized.is_zero() {
        postings.push(Posting {
            account:   pnl_account,
            amount:    Some(Amount {
                quantity:  (trade_line.fifo_pnl_realized - trade_line.ib_commission).neg(),
                commodity: Commodity {
                    name:     trade_line.currency.code().to_owned(),
                    position: CommodityPosition::Left,
                },
            }),
            lot_price: None,
            price:     None,
            balance:   None,
            status:    None,
            comment:   Some(trade_line.transaction_id.clone()),
        });
    }
    Transaction {
        comment: None,
        date: trade_line.trade_date,
        effective_date: trade_line.settle_date_target,
        status: Some(TransactionStatus::Cleared),
        code: None,
        description: format!(
            "{:?} {:?}",
            trade_line.buy_sell, trade_line.contract.description
        ),
        postings,
    }
}

pub(crate) fn parse_corporate_action(
    corporate_action: &CorporateAction,
    account_id: &String,
) -> Transaction {
    let cash_account = format!(
        "{}:{}",
        "Vermögen:Kapitalvermögen:Guthaben:Verrechnungskonto:Interactive Brokers", account_id
    );
    let settlement_account = format!(
        "{}:{}",
        "Vermögen:Kapitalvermögen:Finanzinstrumente:Interactive Brokers", account_id
    );

    let pnl_account = format!(
        "{}:{}",
        "Einnahmen:Kapitalvermögen:Einnahmen aus Veräußerungen:Veräußerungserlöse aus \
         Finanzinstrumenten:Interactive Brokers",
        account_id
    );

    let mut postings = vec![
        Posting {
            account:   cash_account.clone(),
            amount:    Some(Amount {
                quantity:  corporate_action.proceeds,
                commodity: Commodity {
                    name:     corporate_action.currency.code().to_owned(),
                    position: CommodityPosition::Left,
                },
            }),
            lot_price: None,
            price:     None,
            balance:   None,
            status:    None,
            comment:   Some(corporate_action.transaction_id.clone()),
        },
        Posting {
            account:   settlement_account.clone(),
            amount:    Some(Amount {
                quantity:  corporate_action.quantity
                    * corporate_action.contract.multiplier.unwrap_or(Decimal::ONE),
                commodity: Commodity {
                    name:     format!("\"{}\"", corporate_action.contract.symbol),
                    position: CommodityPosition::Right,
                },
            }),
            lot_price: None,
            price:     None,
            // lot_price: if let Some(ibkr_rust_flex::enums::OpenClose::C) =
            //     corporate_action.open_close_indicator
            // {
            //     Some(Amount {
            //         quantity: corporate_action.cost.abs(), //proceeds
            //         // - corporate_action.fifo_pnl_realized / corporate_action.quantity.abs(),
            //         commodity: Commodity {
            //             name: corporate_action.currency.code().to_string(),
            //             position: CommodityPosition::Right,
            //         },
            //     })
            // } else {
            //     None
            // },
            // price: match corporate_action.contract.asset_category {
            //     ibkr_rust_flex::enums::AssetCategory::BOND => Some(Amount {
            //         quantity: corporate_action.trade_price / Decimal::new(100, 0),
            //         commodity: Commodity {
            //             name: corporate_action.currency.code().to_string(),
            //             position: CommodityPosition::Right,
            //         },
            //     }),
            //     _ => Some(Amount {
            //         quantity: corporate_action.trade_price,
            //         commodity: Commodity {
            //             name: corporate_action.currency.code().to_string(),
            //             position: CommodityPosition::Right,
            //         },
            //     }),
            // },
            balance:   None,
            status:    None,
            comment:   None,
        },
        // Posting {
        //     account: cash_account.clone(),
        //     amount: Some(Amount {
        //         quantity: corporate_action.ib_commission,
        //         commodity: Commodity {
        //             name: corporate_action
        //                 .ib_commission_currency
        //                 .unwrap_or(Currency::EUR)
        //                 .code()
        //                 .to_string(),
        //             position: CommodityPosition::Left,
        //         },
        //     }),
        //     lot_price: None,
        //     price: None,
        //     balance: None,
        //     status: None,
        //     comment: Some(corporate_action.transaction_id.clone()),
        // },
        // Posting {
        //     account: fee_account,
        //     amount: Some(Amount {
        //         quantity: corporate_action.ib_commission.neg(),
        //         commodity: Commodity {
        //             name: corporate_action
        //                 .ib_commission_currency
        //                 .unwrap_or(Currency::EUR)
        //                 .code()
        //                 .to_string(),
        //             position: CommodityPosition::Left,
        //         },
        //     }),
        //     lot_price: None,
        //     price: None,
        //     balance: None,
        //     status: None,
        //     comment: None,
        // },
    ];
    if !corporate_action.fifo_pnl_realized.is_zero() {
        postings.push(Posting {
            account:   pnl_account,
            amount:    Some(Amount {
                quantity:  corporate_action.fifo_pnl_realized.neg(),
                commodity: Commodity {
                    name:     corporate_action.currency.code().to_owned(),
                    position: CommodityPosition::Left,
                },
            }),
            lot_price: None,
            price:     None,
            balance:   None,
            status:    None,
            comment:   Some(corporate_action.transaction_id.clone()),
        });
    }
    Transaction {
        comment: None,
        date: corporate_action.report_date.expect(""),
        effective_date: corporate_action.report_date,
        status: Some(TransactionStatus::Cleared),
        code: None,
        description: corporate_action.action_description.clone(),
        postings,
    }
}
pub(crate) fn parse_fx_trade(
    trade_line: &Trade,
    cash_account: String,
    fee_account: String,
) -> Transaction {
    let postings = vec![
        Posting {
            account:   cash_account.clone(),
            amount:    Some(Amount {
                quantity:  trade_line.proceeds,
                commodity: Commodity {
                    name:     trade_line.currency.code().to_owned(),
                    position: CommodityPosition::Left,
                },
            }),
            lot_price: None,
            price:     None,
            balance:   None,
            status:    None,
            comment:   Some(trade_line.transaction_id.clone()),
        },
        Posting {
            account:   cash_account.clone(),
            amount:    Some(Amount {
                quantity:  trade_line.quantity,
                commodity: Commodity {
                    name:     trade_line
                        .contract
                        .symbol
                        .split('.')
                        .next()
                        .unwrap_or("")
                        .to_owned(),
                    position: CommodityPosition::Right,
                },
            }),
            lot_price: None,
            price:     None,
            balance:   None,
            status:    None,
            comment:   None,
        },
        Posting {
            account:   cash_account.clone(),
            amount:    Some(Amount {
                quantity:  trade_line.ib_commission,
                commodity: Commodity {
                    name:     trade_line
                        .ib_commission_currency
                        .unwrap_or(Currency::EUR)
                        .code()
                        .to_owned(),
                    position: CommodityPosition::Left,
                },
            }),
            lot_price: None,
            price:     None,
            balance:   None,
            status:    None,
            comment:   Some(trade_line.transaction_id.clone()),
        },
        Posting {
            account:   fee_account,
            amount:    Some(Amount {
                quantity:  trade_line.ib_commission.neg(),
                commodity: Commodity {
                    name:     trade_line
                        .ib_commission_currency
                        .unwrap_or(Currency::EUR)
                        .code()
                        .to_owned(),
                    position: CommodityPosition::Left,
                },
            }),
            lot_price: None,
            price:     None,
            balance:   None,
            status:    None,
            comment:   None,
        },
    ];
    Transaction {
        comment: None,
        date: trade_line.trade_date,
        effective_date: trade_line.settle_date_target,
        status: Some(TransactionStatus::Cleared),
        code: None,
        description: format!(
            "{:?} {:?}",
            trade_line.buy_sell, trade_line.contract.description
        ),
        postings,
    }
}
// #[tokio::main]
// pub async fn main() -> std::result::Result<(), Box<dyn std::error::Error>> {
//     let mut transactions: Vec<Transaction> = vec![];
//     let response =
// ibkr_rust_flex::flex_statement_from_file(opt.flex_file).await;     {
//         if let Ok(response) = response {
//             let account_id = &response.account_id;
//             if let Some(ref statements) = response.statement_of_funds {
//                 let mut statement_of_funds_lines = statements
//                     .items
//                     .iter()
//                     .filter_map(|item| {
//                         if item.activity_code == None
//                     || item.activity_code == Some("BUY".into())
//                     || item.activity_code == Some("SELL".into())
//                     || item.activity_code == Some("FOREX".into())
//                     || item.activity_code == Some("CORP".into())
//                     // TODO: do we need this?
//                     || item.activity_code == Some("ADJ".into())
//                         {
//                             return None;
//                         }
//                         parse_transaction(
//                             item,
//                             format!(
//                         "{}:{}",
//
// "Vermögen:Kapitalvermögen:Guthaben:Verrechnungskonto:Interactive Brokers",
//                         account_id
//                     ),
//                         )
//                     })
//                     .collect::<Vec<_>>();
//                 transactions.append(&mut statement_of_funds_lines);
//             }
//             if let Some(ref statements) = response.trades {
//                 let mut trade_lines = statements
//                     .items
//                     .iter()
//                     .filter_map(|item| {
//                         if let TradeElements::Trade(trade) = item {
//                             if trade.contract.asset_category ==
// AssetCategory::CASH                             {
//                                 Some(parse_fx_trade(trade,
//                                 format!(        "{}:{}",
//         "Vermögen:Kapitalvermögen:Guthaben:Verrechnungskonto:Interactive
// Brokers", account_id     ),
// format!(        "{}:{}",
//         "Vermögen:Kapitalvermögen:Finanzinstrumente:Interactive Brokers",
// account_id     )                                ))
//                             } else {
//                                 Some(parse_trade_pnl(trade, account_id))
//                             }
//                         } else {
//                             None
//                         }
//                     })
//                     .collect::<Vec<_>>();
//                 transactions.append(&mut trade_lines);
//             }

// if let Some(ref statements) = response.corporate_actions{
//                 let mut actions = statements
//                     .items
//                     .iter()
//                     .map(|action| {
//                                 parse_corporate_action(action, account_id)
//                     })
//                     .collect::<Vec<_>>();
//                 transactions.append(&mut actions);
//             }        }
//     }
//     transactions.iter().for_each(|t| println!("{t}"));
//     if let Some(path) = opt.journal_file {
//         let file_name: PathBuf = PathBuf::from(&path);
//         tracing::error!("{:?}", &file_name);
//         let mut f = std::fs::File::create(file_name)?;

//         for t in transactions.iter() {
//             write!(&mut f, "{t}\n");
//         }
//     }

//     Ok(())
// }
