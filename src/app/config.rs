use std::fmt::{Display, Formatter};

use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};
use time::{OffsetDateTime, UtcOffset};

#[derive(Serialize, Deserialize)]
pub struct Config {
    pub order: Order,
    pub products: Vec<Product>,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            order: Order::default(),
            products: vec![
                Product::new(7, TermType::D, 185, 300, RenewType::P),
                Product::new(7, TermType::D, 185, 300, RenewType::I),
                Product::new(3, TermType::M, 160, 300, RenewType::P),
                Product::new(3, TermType::M, 160, 300, RenewType::I),
                Product::new(6, TermType::M, 180, 345, RenewType::P),
                Product::new(6, TermType::M, 180, 345, RenewType::I),
                Product::new(1, TermType::Y, 200, 345, RenewType::P),
                Product::new(1, TermType::Y, 200, 345, RenewType::I),
                Product::new(3, TermType::Y, 315, 200, RenewType::P),
                Product::new(3, TermType::Y, 315, 200, RenewType::I),
                Product::new(5, TermType::Y, 365, 200, RenewType::P),
                Product::new(5, TermType::Y, 365, 200, RenewType::I),
            ],
        }
    }
}

#[derive(Copy, Clone, Serialize, Deserialize)]
pub struct Order {
    pub principal: Decimal,
    pub save_date: u32,
    pub draw_date: u32,
    pub days: i32,
}

impl Default for Order {
    fn default() -> Self {
        let now = OffsetDateTime::now_utc()
            .to_offset(UtcOffset::from_hms(8, 0, 0).unwrap())
            .date();
        Self {
            principal: Decimal::new(0, 2),
            save_date: now.year() as u32 * 10000 + now.month() as u32 * 100 + now.day() as u32,
            draw_date: (now.year() + 1) as u32 * 10000
                + now.month() as u32 * 100
                + now.day() as u32,
            days: now.replace_year(now.year() + 1).unwrap().to_julian_day() - now.to_julian_day(),
        }
    }
}

#[derive(Serialize, Deserialize, Default)]
pub struct Product {
    pub term: u8,
    pub term_type: TermType,
    pub int_rate: Decimal,
    pub bean_rate: Decimal,
    pub renew_type: RenewType,
    pub interest: Decimal,
    pub bean_int: Decimal,
}

impl Product {
    fn new(
        term: u8,
        term_type: TermType,
        int_rate: i64,
        bean_rate: i64,
        renew_type: RenewType,
    ) -> Self {
        Product {
            term,
            term_type,
            int_rate: Decimal::new(int_rate, 2),
            bean_rate: Decimal::new(bean_rate, 2),
            renew_type,
            ..Default::default()
        }
    }
}

#[derive(Copy, Clone, Serialize, Deserialize, PartialEq)]
pub enum TermType {
    D,
    M,
    Y,
}

impl From<usize> for TermType {
    fn from(i: usize) -> Self {
        match i {
            1 => TermType::M,
            2 => TermType::Y,
            _ => TermType::D,
        }
    }
}

impl Display for TermType {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            TermType::D => write!(f, "天"),
            TermType::M => write!(f, "月"),
            TermType::Y => write!(f, "年"),
        }
    }
}

impl Default for TermType {
    fn default() -> Self {
        TermType::D
    }
}

#[derive(Copy, Clone, Serialize, Deserialize, PartialEq)]
pub enum RenewType {
    N,
    P,
    I,
}

impl From<usize> for RenewType {
    fn from(i: usize) -> Self {
        match i {
            1 => RenewType::P,
            2 => RenewType::I,
            _ => RenewType::N,
        }
    }
}

impl Display for RenewType {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            RenewType::N => write!(f, "不续存"),
            RenewType::P => write!(f, "本金续存"),
            RenewType::I => write!(f, "本息续存"),
        }
    }
}

impl Default for RenewType {
    fn default() -> Self {
        RenewType::N
    }
}
