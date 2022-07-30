use std::cmp::min;

use anyhow::{anyhow, bail, Result};
use eframe::egui::{Color32, ComboBox, RichText, TextEdit, Widget};
use eframe::{egui, Frame, Storage};
use egui_extras::{Size, TableBuilder};
use rust_decimal::Decimal;
use rust_decimal::RoundingStrategy::{MidpointAwayFromZero, ToZero};
use time::{util, Date, Duration, Month};

use config::{Config, Order, Product, RenewType, TermType};

mod config;

pub struct App {
    cfg: Config,
    warn: Result<()>,
}

impl Default for App {
    fn default() -> Self {
        Self {
            cfg: Config::default(),
            warn: Ok(()),
        }
    }
}

impl eframe::App for App {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut Frame) {
        let Config { order, products } = &mut self.cfg;

        if let Err(e) = &self.warn {
            egui::TopBottomPanel::top("my_panel").show(ctx, |ui| {
                let warn = RichText::from(e.to_string()).color(Color32::RED);
                ui.label(warn);
            });
        }

        egui::CentralPanel::default().show(ctx, |ui| {
            let text_height = egui::TextStyle::Body.resolve(ui.style()).size * 2.0;

            TableBuilder::new(ui)
                .cell_layout(egui::Layout::left_to_right())
                .column(Size::remainder())
                .column(Size::remainder())
                .column(Size::remainder())
                .column(Size::remainder())
                .header(text_height, |mut header| {
                    header.col(|ui| {
                        ui.heading("本金");
                        let mut principal = format!("{:.2}", order.principal);
                        if ui.text_edit_singleline(&mut principal).changed() {
                            if let Ok(mut v) = principal.parse::<Decimal>() {
                                v = v.round_dp_with_strategy(2, ToZero);
                                if v >= Decimal::new(1000_0000_0000, 0) {
                                    self.warn = Err(anyhow!("一千亿啊，土豪，还需要算吗？"))
                                } else {
                                    order.principal = v;
                                    for product in products.iter_mut() {
                                        self.warn = calc(order, product);
                                    }
                                }
                            }
                        };
                    });

                    header.col(|ui| {
                        ui.heading("购买日期：");
                        let mut save_date = order.save_date.to_string();
                        if ui.text_edit_singleline(&mut save_date).changed() {
                            save_date.truncate(8);
                            if let Ok(v) = save_date.parse() {
                                order.save_date = v;
                                for product in products.iter_mut() {
                                    self.warn = calc(order, product);
                                }
                            }
                        }
                    });

                    header.col(|ui| {
                        ui.heading("支取日期：");
                        let mut draw_date = order.draw_date.to_string();
                        if ui.text_edit_singleline(&mut draw_date).changed() {
                            draw_date.truncate(8);
                            if let Ok(v) = draw_date.parse() {
                                order.draw_date = v;
                                for product in products.iter_mut() {
                                    self.warn = calc(order, product);
                                }
                            }
                        }
                    });

                    header.col(|ui| {
                        ui.heading(format!("天数：{}", order.days));
                    });
                });

            ui.separator();

            TableBuilder::new(ui)
                .striped(true)
                .cell_layout(egui::Layout::left_to_right())
                .column(Size::initial(80.0))
                .column(Size::remainder())
                .column(Size::remainder())
                .column(Size::initial(90.0))
                .column(Size::remainder())
                .column(Size::remainder())
                .column(Size::initial(40.0))
                .header(text_height, |mut header| {
                    header.col(|ui| {
                        ui.heading("存期");
                    });
                    header.col(|ui| {
                        ui.heading("利率(%)");
                    });
                    header.col(|ui| {
                        ui.heading("邦豆利率(%)");
                    });
                    header.col(|ui| {
                        ui.heading("续存类型");
                    });
                    header.col(|ui| {
                        ui.heading("利息");
                    });
                    header.col(|ui| {
                        ui.heading("邦豆利息");
                    });
                    header.col(|ui| {
                        if ui.button("添加").clicked() {
                            products.push(Product::default());
                        }
                    });
                })
                .body(|body| {
                    body.rows(text_height, products.len(), |row_index, mut row| {
                        if let Some(product) = products.get_mut(row_index) {
                            row.col(|ui| {
                                ui.horizontal(|ui| {
                                    let mut term = product.term.to_string();
                                    if TextEdit::singleline(&mut term)
                                        .desired_width(20.0)
                                        .ui(ui)
                                        .changed()
                                    {
                                        if let Ok(v) = term.parse() {
                                            product.term = v;
                                            self.warn = calc(order, product);
                                        }
                                    }

                                    let mut selected = product.term_type as usize;
                                    if ComboBox::from_id_source(format!("存期类型{}", row_index))
                                        .width(20.0)
                                        .show_index(ui, &mut selected, 3, |i| {
                                            TermType::from(i).to_string()
                                        })
                                        .changed()
                                    {
                                        product.term_type = TermType::from(selected);
                                        self.warn = calc(order, product);
                                    };
                                });
                            });
                            row.col(|ui| {
                                let mut int_rate = format!("{:.2}", product.int_rate);
                                if ui.text_edit_singleline(&mut int_rate).changed() {
                                    if let Ok(mut v) = int_rate.parse::<Decimal>() {
                                        v = v.round_dp_with_strategy(2, ToZero);
                                        if v > Decimal::TEN {
                                            self.warn = Err(anyhow!(
                                                "哪里有这么高的利率，苟富贵勿相忘啊，兄弟！"
                                            ));
                                        } else {
                                            product.int_rate = v;
                                            self.warn = calc(order, product);
                                        }
                                    }
                                };
                            });
                            row.col(|ui| {
                                let mut bean_rate = format!("{:.2}", product.bean_rate);
                                if ui.text_edit_singleline(&mut bean_rate).changed() {
                                    if let Ok(mut v) = bean_rate.parse::<Decimal>() {
                                        v = v.round_dp_with_strategy(2, ToZero);
                                        if v > Decimal::TEN {
                                            self.warn = Err(anyhow!(
                                                "哪里有这么高的利率，苟富贵勿相忘啊，兄弟！"
                                            ));
                                        } else {
                                            product.bean_rate = v;
                                            self.warn = calc(order, product);
                                        }
                                    }
                                };
                            });
                            row.col(|ui| {
                                let mut selected = product.renew_type as usize;
                                if ComboBox::from_id_source(format!("续存方式{}", row_index))
                                    .width(80.0)
                                    .show_index(ui, &mut selected, 3, |i| {
                                        RenewType::from(i).to_string()
                                    })
                                    .changed()
                                {
                                    product.renew_type = RenewType::from(selected);
                                    self.warn = calc(order, product);
                                };
                            });
                            row.col(|ui| {
                                ui.label(format!("{:.2}", product.interest));
                            });
                            row.col(|ui| {
                                ui.label(format!("{:.2}", product.bean_int));
                            });
                            row.col(|ui| {
                                if ui.button("删除").clicked() {
                                    products.remove(row_index);
                                }
                            });
                        }
                    });
                });
        });

        egui::TopBottomPanel::bottom("bottom").show(ctx, |ui| {
            egui::widgets::global_dark_light_mode_switch(ui);
        });
    }

    fn save(&mut self, storage: &mut dyn Storage) {
        eframe::set_value(storage, eframe::APP_KEY, &self.cfg);
    }
}

impl App {
    pub fn new(cc: &eframe::CreationContext<'_>) -> Self {
        setup_custom_fonts(&cc.egui_ctx);
        // cc.egui_ctx.set_visuals(egui::Visuals::dark());
        // cc.egui_ctx.set_debug_on_hover(true);

        cc.storage
            .and_then(|storage| eframe::get_value::<Config>(storage, eframe::APP_KEY))
            .map(|cfg| Self {
                cfg,
                ..Default::default()
            })
            .unwrap_or_default()
    }
}

fn setup_custom_fonts(ctx: &egui::Context) {
    // Start with the default fonts (we will be adding to them rather than replacing them).
    let mut fonts = egui::FontDefinitions::default();

    // Install my own font (maybe supporting non-latin characters).
    // .ttf and .otf files supported.
    fonts.font_data.insert(
        "simkai".to_owned(),
        egui::FontData::from_static(include_bytes!("../../resource/simkai.ttf")),
    );

    let entry = fonts
        .families
        .entry(egui::FontFamily::Proportional)
        .or_default();
    entry.push("simkai".to_owned());

    let entry = fonts
        .families
        .entry(egui::FontFamily::Monospace)
        .or_default();
    entry.push("simkai".to_owned());

    // Tell egui to use these fonts:
    ctx.set_fonts(fonts);
}

fn calc(order: &mut Order, product: &mut Product) -> Result<()> {
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

    Ok(())
}
