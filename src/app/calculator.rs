use std::cmp::min;
use std::collections::HashMap;

use anyhow::{anyhow, bail, Result};
use rust_decimal::Decimal;
use rust_decimal::RoundingStrategy::{MidpointAwayFromZero, ToZero};
use time::{util, Date, Duration, Month};

use crate::app::config::{Order, Product, RenewType, TermType};

#[derive(Default)]
pub struct Calculator {
    cache: HashMap<String, (i32, Decimal, Decimal)>,
}

impl Calculator {
    pub fn calc(&mut self, order: &mut Order, product: &mut Product) -> Result<()> {
        if order.save_date < 10000101
            || order.save_date > 99991231
            || order.draw_date < 10000101
            || order.draw_date > 99991231
            || order.save_date > order.draw_date
        {
            bail!("穿越时空？")
        }

        if product.term < 1 {
            return Ok(());
        }

        let key = format!(
            "{}_{}_{}_{}_{}_{}_{}_{}",
            order.principal,
            order.save_date,
            order.draw_date,
            product.term,
            product.term_type,
            product.int_rate,
            product.bean_rate,
            product.renew_type
        );

        if let Some(v) = self.cache.get(&key) {
            order.days = v.0;
            product.interest = v.1;
            product.bean_int = v.2;
            return Ok(());
        }

        let save_date = Date::from_calendar_date(
            (order.save_date / 10000) as i32,
            Month::try_from((order.save_date / 100 % 100) as u8)
                .map_err(|e| anyhow!("购买月份有误！{e}"))?,
            (order.save_date % 100) as u8,
        )
        .map_err(|e| anyhow!("购买日期有误!{e}"))?;

        let draw_date = Date::from_calendar_date(
            (order.draw_date / 10000) as i32,
            Month::try_from((order.draw_date / 100 % 100) as u8)
                .map_err(|e| anyhow!("支取月份有误！{e}"))?,
            (order.draw_date % 100) as u8,
        )
        .map_err(|e| anyhow!("支取日期有误!{e}"))?;

        order.days = (draw_date.to_julian_day() - save_date.to_julian_day()) as i32;
        if order.days > 36500 {
            bail!("你确定可以存一个世纪？")
        }

        let year_days = Decimal::new(360, 0);

        let mut start_date = save_date;
        let mut principal = order.principal;
        let mut interest = Decimal::ZERO;
        let mut int_rate = product.int_rate;
        let mut bean_rate = product.bean_rate;
        product.bean_int = Decimal::ZERO;

        while start_date < draw_date {
            let mut end_date = match &product.term_type {
                TermType::D => start_date.saturating_add(Duration::days(product.term as i64)),
                TermType::M => {
                    let month = start_date.month() as u8 + product.term - 1;
                    let year = start_date.year() + month as i32 / 12;
                    let month = Month::try_from(month % 12 + 1).unwrap();
                    let max_day = util::days_in_year_month(year, month);

                    Date::from_calendar_date(year, month, min(start_date.day(), max_day))?
                }
                TermType::Y => {
                    let year = start_date.year() + product.term as i32;
                    let month = start_date.month();
                    let max_day = util::days_in_year_month(year, month);
                    Date::from_calendar_date(year, month, min(start_date.day(), max_day))?
                }
            };

            if end_date > draw_date {
                end_date = draw_date;
                int_rate = Decimal::new(35, 2);
                bean_rate = Decimal::ZERO;
            }

            let days = Decimal::new(
                (end_date.to_julian_day() - start_date.to_julian_day()) as i64,
                0,
            );
            interest += (days / year_days * int_rate / Decimal::ONE_HUNDRED * principal)
                .round_dp_with_strategy(2, MidpointAwayFromZero);
            product.bean_int += (days / year_days * bean_rate / Decimal::ONE_HUNDRED * principal)
                .round_dp_with_strategy(2, ToZero);

            match product.renew_type {
                RenewType::N => {
                    break;
                }
                RenewType::P => {}
                RenewType::I => {
                    principal += interest;
                    interest = Decimal::ZERO;
                }
            }
            start_date = end_date;
        }

        product.interest = principal - order.principal + interest;
        self.cache
            .insert(key, (order.days, product.interest, product.bean_int));

        Ok(())
    }
}
