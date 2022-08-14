use std::cmp::min;

use anyhow::{anyhow, bail, Result};
use rust_decimal::Decimal;
use rust_decimal::RoundingStrategy::{MidpointAwayFromZero, ToZero};
use time::{util, Date, Duration, Month};

use crate::app::config::{Order, Product, RenewType, TermType};

#[derive(Copy, Clone, Eq, PartialEq, Hash)]
pub struct Req {
    pub principal: Decimal,
    pub save_date: u32,
    pub draw_date: u32,
    pub term: u8,
    pub term_type: TermType,
    pub int_rate: Decimal,
    pub bean_rate: Decimal,
    pub renew_type: RenewType,
}

impl Req {
    pub fn new(order: &Order, product: &Product) -> Self {
        Self {
            principal: order.principal,
            save_date: order.save_date,
            draw_date: order.draw_date,
            term: product.term,
            term_type: product.term_type,
            int_rate: product.int_rate,
            bean_rate: product.bean_rate,
            renew_type: product.renew_type,
        }
    }
}

pub fn u32_to_date(date: u32) -> Result<Date> {
    Date::from_calendar_date(
        (date / 10000) as i32,
        Month::try_from((date / 100 % 100) as u8).map_err(|e| anyhow!("月份有误！{e}"))?,
        (date % 100) as u8,
    )
    .map_err(|e| anyhow!("日期有误!{e}"))
}

pub fn check_date(order: &mut Order) -> Result<()> {
    if order.save_date < 10000101
        || order.save_date > 99991231
        || order.draw_date < 10000101
        || order.draw_date > 99991231
        || order.save_date > order.draw_date
    {
        bail!("穿越时空？")
    }

    let save_date = u32_to_date(order.save_date)?;
    let draw_date = u32_to_date(order.draw_date)?;

    order.days = (draw_date.to_julian_day() - save_date.to_julian_day()) as i32;
    if order.days > 36500 {
        bail!("你确定可以存一个世纪？")
    }

    Ok(())
}

pub fn calc(req: &Req) -> (Decimal, Decimal) {
    if req.term < 1 {
        return (Decimal::ZERO, Decimal::ZERO);
    }

    let save_date = u32_to_date(req.save_date).unwrap();
    let draw_date = u32_to_date(req.draw_date).unwrap();

    let mut start_date = save_date;
    let mut principal = req.principal;
    let mut interest = Decimal::ZERO;
    let mut bean_int = Decimal::ZERO;
    let mut int_rate = req.int_rate;
    let mut bean_rate = req.bean_rate;

    while start_date < draw_date {
        let mut end_date = match &req.term_type {
            TermType::D => start_date.saturating_add(Duration::days(req.term as i64)),
            TermType::M => {
                let month = start_date.month() as u8 + req.term - 1;
                let year = start_date.year() + month as i32 / 12;
                let month = Month::try_from(month % 12 + 1).unwrap();
                let max_day = util::days_in_year_month(year, month);

                Date::from_calendar_date(year, month, min(start_date.day(), max_day)).unwrap()
            }
            TermType::Y => {
                let year = start_date.year() + req.term as i32;
                let month = start_date.month();
                let max_day = util::days_in_year_month(year, month);
                Date::from_calendar_date(year, month, min(start_date.day(), max_day)).unwrap()
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

        // 利息2位小数四舍五入, 溢出归0
        interest = calc_interest(principal, int_rate, days)
            .and_then(|d| d.checked_add(interest))
            .map(|d| d.round_dp_with_strategy(2, MidpointAwayFromZero))
            .unwrap_or_default();

        // 邦豆2位小数之后全部舍弃, 溢出归0
        bean_int = calc_interest(principal, bean_rate, days)
            .and_then(|d| d.checked_add(bean_int))
            .map(|d| d.round_dp_with_strategy(2, ToZero))
            .unwrap_or_default();

        match req.renew_type {
            RenewType::N => {
                break;
            }
            RenewType::P => {}
            RenewType::I => {
                principal = principal.checked_add(interest).unwrap_or_default();
                interest = Decimal::ZERO;
            }
        }
        start_date = end_date;
    }

    (principal - req.principal + interest, bean_int)
}

fn calc_interest(principal: Decimal, rate: Decimal, days: Decimal) -> Option<Decimal> {
    days.checked_div(Decimal::new(360, 0))
        .and_then(|d| d.checked_mul(rate))
        .and_then(|d| d.checked_div(Decimal::ONE_HUNDRED))
        .and_then(|d| d.checked_mul(principal))
}
