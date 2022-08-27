use std::collections::HashMap;

use anyhow::{anyhow, Result};
use async_channel::{Receiver, Sender};
use eframe::egui::{Align, Color32, ComboBox, RichText, TextEdit, Widget};
use eframe::{egui, Frame, Storage};
use egui_extras::{Size, TableBuilder};
use futures::executor::ThreadPoolBuilder;
use rayon::prelude::*;
use rust_decimal::Decimal;
use rust_decimal::RoundingStrategy::ToZero;

use config::{Config, Product, RenewType, TermType};

use crate::app::calculator::Req;

mod calculator;
mod config;

pub struct App {
    cfg: Config,
    warn: Result<()>,
    // worker: ThreadPool,
    req_s: Sender<Req>,
    res_r: Receiver<HashMap<Req, (Decimal, Decimal)>>,
    cache: HashMap<Req, (Decimal, Decimal)>,
}

impl eframe::App for App {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut Frame) {
        self.refresh_cache();

        if let Err(e) = &self.warn {
            egui::TopBottomPanel::top("my_panel").show(ctx, |ui| {
                let warn = RichText::from(e.to_string()).color(Color32::RED);
                ui.label(warn);
            });
        }

        egui::CentralPanel::default().show(ctx, |ui| {
            let text_height = egui::TextStyle::Body.resolve(ui.style()).size * 2.0;

            TableBuilder::new(ui)
                .cell_layout(egui::Layout::left_to_right(Align::Center))
                .column(Size::remainder())
                .column(Size::remainder())
                .column(Size::remainder())
                .column(Size::remainder())
                .header(text_height, |mut header| {
                    header.col(|ui| {
                        ui.heading("本金");
                        let mut principal = format!("{:.2}", self.cfg.order.principal);
                        if ui.text_edit_singleline(&mut principal).changed() {
                            self.principal_changed(&*principal);
                        };
                    });

                    header.col(|ui| {
                        ui.heading("购买日期：");
                        let mut save_date = self.cfg.order.save_date.to_string();
                        if ui.text_edit_singleline(&mut save_date).changed() {
                            save_date.truncate(8);
                            self.save_date_changed(&*save_date);
                        }
                    });

                    header.col(|ui| {
                        ui.heading("支取日期：");
                        let mut draw_date = self.cfg.order.draw_date.to_string();
                        if ui.text_edit_singleline(&mut draw_date).changed() {
                            draw_date.truncate(8);
                            self.draw_date_changed(&*draw_date);
                        }
                    });

                    header.col(|ui| {
                        ui.heading(format!("天数：{}", self.cfg.order.days));
                    });
                });

            ui.separator();

            TableBuilder::new(ui)
                .striped(true)
                .cell_layout(egui::Layout::left_to_right(Align::Center))
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
                            self.cfg.products.push(Product::default());
                        }
                    });
                })
                .body(|body| {
                    body.rows(
                        text_height,
                        self.cfg.products.len(),
                        |row_index, mut row| {
                            if self.cfg.products.get_mut(row_index).is_none() {
                                return;
                            }
                            row.col(|ui| {
                                ui.horizontal(|ui| {
                                    let mut term = self.cfg.products[row_index].term.to_string();
                                    if TextEdit::singleline(&mut term)
                                        .desired_width(20.0)
                                        .ui(ui)
                                        .changed()
                                    {
                                        self.term_changed(&*term, row_index);
                                    }

                                    let mut term_type =
                                        self.cfg.products[row_index].term_type as usize;
                                    if ComboBox::from_id_source(format!("存期类型{}", row_index))
                                        .width(20.0)
                                        .show_index(ui, &mut term_type, 3, |i| {
                                            TermType::from(i).to_string()
                                        })
                                        .changed()
                                    {
                                        self.term_type_changed(term_type, row_index);
                                    };
                                });
                            });
                            row.col(|ui| {
                                let mut int_rate =
                                    format!("{:.2}", self.cfg.products[row_index].int_rate);
                                if ui.text_edit_singleline(&mut int_rate).changed() {
                                    self.int_rate_changed(&*int_rate, row_index);
                                };
                            });
                            row.col(|ui| {
                                let mut bean_rate =
                                    format!("{:.2}", self.cfg.products[row_index].bean_rate);
                                if ui.text_edit_singleline(&mut bean_rate).changed() {
                                    self.bean_rate_changed(&*bean_rate, row_index);
                                };
                            });
                            row.col(|ui| {
                                let mut renew_type =
                                    self.cfg.products[row_index].renew_type as usize;
                                if ComboBox::from_id_source(format!("续存方式{}", row_index))
                                    .width(80.0)
                                    .show_index(ui, &mut renew_type, 3, |i| {
                                        RenewType::from(i).to_string()
                                    })
                                    .changed()
                                {
                                    self.renew_type_changed(renew_type, row_index);
                                };
                            });
                            row.col(|ui| {
                                ui.label(format!("{:.2}", self.cfg.products[row_index].interest));
                            });
                            row.col(|ui| {
                                ui.label(format!("{:.2}", self.cfg.products[row_index].bean_int));
                            });
                            row.col(|ui| {
                                if ui.button("删除").clicked() {
                                    self.cfg.products.remove(row_index);
                                }
                            });
                        },
                    );
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
        let egui_ctx = cc.egui_ctx.clone();
        setup_custom_fonts(&egui_ctx);
        // egui_ctx.set_visuals(egui::Visuals::dark());
        // egui_ctx.set_debug_on_hover(true);

        let cfg = cc
            .storage
            .and_then(|storage| eframe::get_value::<Config>(storage, eframe::APP_KEY))
            .unwrap_or_default();

        let worker = ThreadPoolBuilder::new().pool_size(1).create().unwrap();

        //req 需要计算的key (本金-购买日期-支取日期-产品存期-存期类型-利率-邦豆利率-滚存类型)
        let (req_s, req_r) = async_channel::unbounded::<Req>();
        //res 计算结果HashMap<key,value> ()
        let (res_s, res_r) = async_channel::unbounded::<HashMap<Req, (Decimal, Decimal)>>();

        worker.spawn_ok(async move {
            while let Ok(req) = req_r.recv().await {
                let mut reqs = HashMap::new();
                reqs.insert(req, (Decimal::ZERO, Decimal::ZERO));

                while let Ok(req_more) = req_r.try_recv() {
                    reqs.insert(req_more, (Decimal::ZERO, Decimal::ZERO));
                }

                reqs.par_iter_mut()
                    .for_each(|(req, res)| *res = calculator::calc(req));

                res_s.send(reqs).await.ok();
                egui_ctx.request_repaint();
            }
        });

        let mut cache = HashMap::new();
        cfg.products.iter().for_each(|p| {
            cache.insert(Req::new(&cfg.order, p), (p.interest, p.bean_int));
        });

        Self {
            cfg,
            warn: Ok(()),
            // worker,
            req_s,
            res_r,
            cache,
        }
    }

    fn refresh_cache(&mut self) {
        while let Ok(res) = self.res_r.try_recv() {
            res.iter().for_each(|(k, v)| {
                self.cache.insert(*k, *v);
            })
        }

        self.cfg.products.iter_mut().for_each(|p| {
            if let Some(res) = self.cache.get(&Req::new(&self.cfg.order, p)) {
                p.interest = res.0;
                p.bean_int = res.1;
            }
        })
    }

    fn calc(&mut self, index: Option<usize>) {
        self.warn = calculator::check_date(&mut self.cfg.order);

        if self.warn.is_ok() {
            let order = &self.cfg.order;
            self.cfg
                .products
                .par_iter_mut()
                .enumerate()
                .for_each(|(i, product)| {
                    if index.is_some() && index.ne(&Some(i)) {
                        return;
                    }

                    let req = Req::new(order, product);
                    if let Some(res) = self.cache.get(&req) {
                        product.interest = res.0;
                        product.bean_int = res.1;
                    } else {
                        self.req_s.send_blocking(req).unwrap();
                    }
                });
        }
    }

    fn principal_changed(&mut self, principal: &str) {
        if let Ok(mut v) = principal.parse::<Decimal>() {
            v = v.round_dp_with_strategy(2, ToZero);
            if v >= Decimal::new(1000_0000_0000, 0) {
                self.warn = Err(anyhow!("一千亿啊，土豪，还需要算吗？"))
            } else {
                self.cfg.order.principal = v;
                self.calc(None);
            }
        }
    }

    fn save_date_changed(&mut self, save_date: &str) {
        if let Ok(v) = save_date.parse() {
            self.cfg.order.save_date = v;
            self.calc(None);
        }
    }

    fn draw_date_changed(&mut self, draw_date: &str) {
        if let Ok(v) = draw_date.parse() {
            self.cfg.order.draw_date = v;
            self.calc(None);
        }
    }

    fn term_changed(&mut self, term: &str, row_index: usize) {
        if let Ok(v) = term.parse() {
            self.cfg.products[row_index].term = v;
            self.calc(Some(row_index));
        }
    }

    fn term_type_changed(&mut self, term_type: usize, row_index: usize) {
        self.cfg.products[row_index].term_type = TermType::from(term_type);
        self.calc(Some(row_index));
    }

    fn int_rate_changed(&mut self, int_rate: &str, row_index: usize) {
        if let Ok(mut v) = int_rate.parse::<Decimal>() {
            v = v.round_dp_with_strategy(2, ToZero);
            if v > Decimal::TEN {
                self.warn = Err(anyhow!("哪里有这么高的利率，苟富贵勿相忘啊，兄弟！"));
            } else {
                self.cfg.products[row_index].int_rate = v;
                self.calc(Some(row_index));
            }
        }
    }

    fn bean_rate_changed(&mut self, bean_rate: &str, row_index: usize) {
        if let Ok(mut v) = bean_rate.parse::<Decimal>() {
            v = v.round_dp_with_strategy(2, ToZero);
            if v > Decimal::TEN {
                self.warn = Err(anyhow!("哪里有这么高的利率，苟富贵勿相忘啊，兄弟！"));
            } else {
                self.cfg.products[row_index].bean_rate = v;
                self.calc(Some(row_index));
            }
        }
    }

    fn renew_type_changed(&mut self, renew_type: usize, row_index: usize) {
        self.cfg.products[row_index].renew_type = RenewType::from(renew_type);
        self.calc(Some(row_index));
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
