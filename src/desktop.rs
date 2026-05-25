//! Native desktop UI for WAPC.
//! @author codex

use std::{
    collections::BTreeMap,
    path::{Path, PathBuf},
    sync::Arc,
};

use anyhow::{Context, Result};
use chrono::{Duration, Local};
use eframe::egui::{
    self, Align, Align2, Color32, FontData, FontDefinitions, FontFamily, FontId, Layout, Rect,
    RichText, Sense, Stroke, Vec2, Visuals,
};

use crate::{
    launchd, scanner,
    store::{DailyToolSummary, UsageStore, UsageSummary},
};

#[derive(Clone, Debug, Default)]
pub struct DesktopSnapshot {
    pub today: Vec<UsageSummary>,
    pub tools: Vec<UsageSummary>,
    pub projects: Vec<UsageSummary>,
    pub daily: Vec<DailyToolSummary>,
    pub trend_days: Vec<String>,
    pub scan_records: usize,
    pub service_installed: bool,
    pub service_loaded: bool,
    pub db_exists: bool,
}

impl DesktopSnapshot {
    pub fn total_tokens_today(&self) -> u64 {
        self.today.iter().map(|summary| summary.usage.total()).sum()
    }

    pub fn total_records_today(&self) -> u64 {
        self.today.iter().map(|summary| summary.records).sum()
    }

    pub fn estimated_cost_today(&self) -> f64 {
        self.today.iter().map(|summary| summary.cost_usd).sum()
    }
}

pub struct WapcDesktopApp {
    home: PathBuf,
    db: PathBuf,
    snapshot: DesktopSnapshot,
    status: String,
    last_error: Option<String>,
}

impl WapcDesktopApp {
    pub fn new(home: PathBuf, db: PathBuf) -> Self {
        let mut app = Self {
            home,
            db,
            snapshot: DesktopSnapshot::default(),
            status: "Ready".to_string(),
            last_error: None,
        };
        app.refresh();
        app
    }

    fn refresh(&mut self) {
        match load_snapshot(&self.home, &self.db) {
            Ok(snapshot) => {
                self.snapshot = snapshot;
                self.status = format!("最后刷新：{}", Local::now().format("%H:%M:%S"));
                self.last_error = None;
            }
            Err(error) => {
                self.last_error = Some(error.to_string());
            }
        }
    }

    fn scan_now(&mut self) {
        match scan_into_store(&self.home, &self.db) {
            Ok(count) => {
                self.status = format!("已索引 {count} 条使用记录");
                self.refresh();
            }
            Err(error) => {
                self.last_error = Some(error.to_string());
            }
        }
    }
}

impl eframe::App for WapcDesktopApp {
    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        egui::CentralPanel::default()
            .frame(egui::Frame::default().fill(PAGE_BG))
            .show_inside(ui, |ui| {
                ui.spacing_mut().item_spacing = Vec2::ZERO;
                let size = ui.available_size();
                ui.horizontal(|ui| {
                    ui.allocate_ui_with_layout(
                        Vec2::new(SIDEBAR_WIDTH, size.y),
                        Layout::top_down(Align::LEFT),
                        |ui| sidebar(ui, self),
                    );
                    ui.allocate_ui_with_layout(
                        Vec2::new((size.x - SIDEBAR_WIDTH).max(0.0), size.y),
                        Layout::top_down(Align::LEFT),
                        |ui| {
                            egui::ScrollArea::vertical()
                                .auto_shrink([false, false])
                                .show(ui, |ui| content(ui, self));
                        },
                    );
                });
            });
    }
}

const PAGE_BG: Color32 = Color32::from_rgb(246, 248, 251);
const SIDEBAR_BG: Color32 = Color32::from_rgb(241, 246, 252);
const SIDEBAR_ACTIVE: Color32 = Color32::from_rgb(224, 237, 255);
const PANEL_BG: Color32 = Color32::from_rgb(255, 255, 255);
const PANEL_SOFT: Color32 = Color32::from_rgb(249, 251, 254);
const BORDER: Color32 = Color32::from_rgb(218, 225, 235);
const BORDER_SOFT: Color32 = Color32::from_rgb(231, 236, 244);
const TEXT: Color32 = Color32::from_rgb(23, 32, 48);
const MUTED: Color32 = Color32::from_rgb(92, 106, 127);
const BLUE: Color32 = Color32::from_rgb(31, 111, 235);
const TEAL: Color32 = Color32::from_rgb(18, 160, 166);
const PURPLE: Color32 = Color32::from_rgb(126, 87, 235);
const ORANGE: Color32 = Color32::from_rgb(241, 118, 31);
const GREEN: Color32 = Color32::from_rgb(38, 151, 91);
const RED: Color32 = Color32::from_rgb(204, 70, 58);
const SIDEBAR_WIDTH: f32 = 196.0;

fn sidebar(ui: &mut egui::Ui, app: &mut WapcDesktopApp) {
    egui::Frame::default()
        .fill(SIDEBAR_BG)
        .stroke(Stroke::new(1.0, BORDER_SOFT))
        .inner_margin(egui::Margin::symmetric(14, 22))
        .show(ui, |ui| {
            ui.set_width(SIDEBAR_WIDTH);
            ui.set_min_height(ui.available_height());
            ui.label(RichText::new("WAPC").size(28.0).strong().color(BLUE));
            ui.add_space(6.0);
            ui.label(
                RichText::new("本机 AI 编程工具\nToken 观测器")
                    .size(13.0)
                    .line_height(Some(18.0))
                    .color(MUTED),
            );
            ui.add_space(32.0);
            nav_item(ui, "概览", true, IconKind::Home);
            nav_item(ui, "工具", false, IconKind::Tools);
            nav_item(ui, "项目", false, IconKind::Folder);
            nav_item(ui, "隐私与数据源", false, IconKind::Shield);
            nav_item(ui, "后台服务", false, IconKind::Server);
            ui.add_space(18.0);
            ui.separator();
            nav_item(ui, "设置", false, IconKind::Settings);
            nav_item(ui, "帮助", false, IconKind::Help);
            nav_item(ui, "关于", false, IconKind::Info);
            ui.with_layout(Layout::bottom_up(Align::LEFT), |ui| {
                ui.label(
                    RichText::new(format!("v{}", env!("CARGO_PKG_VERSION")))
                        .size(12.0)
                        .color(MUTED),
                );
                ui.add_space(18.0);
                if sidebar_button(ui, "手动扫描", IconKind::Refresh).clicked() {
                    app.scan_now();
                }
                ui.add_space(10.0);
                sidebar_status_card(ui, app);
            });
        });
}

fn nav_item(ui: &mut egui::Ui, label: &str, active: bool, icon: IconKind) {
    let fill = if active { SIDEBAR_ACTIVE } else { SIDEBAR_BG };
    let color = if active { BLUE } else { TEXT };
    egui::Frame::default()
        .fill(fill)
        .corner_radius(8.0)
        .inner_margin(egui::Margin::symmetric(10, 9))
        .show(ui, |ui| {
            ui.set_width(158.0);
            ui.horizontal(|ui| {
                nav_icon(ui, icon, color);
                ui.add_space(8.0);
                ui.label(RichText::new(label).size(14.0).strong().color(color));
            });
        });
    ui.add_space(6.0);
}

fn sidebar_status_card(ui: &mut egui::Ui, app: &WapcDesktopApp) {
    egui::Frame::default()
        .fill(PANEL_BG)
        .stroke(Stroke::new(1.0, BORDER))
        .corner_radius(8.0)
        .inner_margin(egui::Margin::same(12))
        .show(ui, |ui| {
            ui.set_width(158.0);
            ui.horizontal(|ui| {
                dot(
                    ui,
                    if app.snapshot.service_loaded {
                        GREEN
                    } else {
                        RED
                    },
                );
                ui.label(
                    RichText::new(if app.snapshot.service_loaded {
                        "服务运行中"
                    } else {
                        "服务未运行"
                    })
                    .size(13.0)
                    .strong()
                    .color(TEXT),
                );
            });
            ui.add_space(8.0);
            ui.label(RichText::new("每 15 分钟自动扫描").size(12.0).color(MUTED));
            ui.label(
                RichText::new(format!(
                    "下次运行：{}",
                    (Local::now() + Duration::minutes(15)).format("%H:%M")
                ))
                .size(12.0)
                .color(MUTED),
            );
        });
}

fn sidebar_button(ui: &mut egui::Ui, label: &str, icon: IconKind) -> egui::Response {
    ui.add_sized(
        [166.0, 36.0],
        egui::Button::new({
            let text = format!("   {label}");
            RichText::new(text).size(13.0).strong().color(TEXT)
        })
        .fill(PANEL_BG)
        .stroke(Stroke::new(1.0, BORDER))
        .corner_radius(7.0),
    )
    .on_hover_ui(|ui| {
        ui.horizontal(|ui| {
            nav_icon(ui, icon, TEXT);
            ui.label(label);
        });
    })
}

fn content(ui: &mut egui::Ui, app: &mut WapcDesktopApp) {
    egui::Frame::default()
        .fill(PAGE_BG)
        .inner_margin(egui::Margin::symmetric(22, 24))
        .show(ui, |ui| {
            ui.set_min_width(ui.available_width());
            header(ui, app);
            ui.add_space(22.0);
            kpi_row(ui, app);
            ui.add_space(20.0);
            two_column(
                ui,
                0.58,
                16.0,
                |ui| trend_panel(ui, &app.snapshot),
                |ui| service_panel(ui, app),
            );
            ui.add_space(18.0);
            two_column(
                ui,
                0.43,
                16.0,
                |ui| tool_table(ui, &app.snapshot),
                |ui| project_table(ui, &app.snapshot),
            );
            ui.add_space(18.0);
            footer(ui, app);
        });
}

fn two_column(
    ui: &mut egui::Ui,
    left_ratio: f32,
    gap: f32,
    left: impl FnOnce(&mut egui::Ui),
    right: impl FnOnce(&mut egui::Ui),
) {
    let available = ui.available_width();
    let usable = (available - gap).max(0.0);
    let left_width = usable * left_ratio;
    let right_width = usable - left_width;
    ui.horizontal_top(|ui| {
        ui.spacing_mut().item_spacing.x = gap;
        ui.allocate_ui_with_layout(
            Vec2::new(left_width, 1.0),
            Layout::top_down(Align::LEFT),
            |ui| {
                ui.set_width(left_width);
                left(ui);
            },
        );
        ui.allocate_ui_with_layout(
            Vec2::new(right_width, 1.0),
            Layout::top_down(Align::LEFT),
            |ui| {
                ui.set_width(right_width);
                right(ui);
            },
        );
    });
}

fn header(ui: &mut egui::Ui, app: &mut WapcDesktopApp) {
    ui.horizontal_top(|ui| {
        ui.vertical(|ui| {
            ui.label(
                RichText::new("AI Coding Usage")
                    .size(30.0)
                    .strong()
                    .color(TEXT),
            );
            ui.add_space(3.0);
            ui.label(
                RichText::new("本机 AI 编程工具 Token 使用情况总览")
                    .size(14.0)
                    .color(MUTED),
            );
        });
        ui.allocate_ui_with_layout(
            Vec2::new(ui.available_width(), 46.0),
            Layout::right_to_left(Align::Center),
            |ui| {
                if icon_button(ui, IconKind::Refresh, "刷新").clicked() {
                    app.refresh();
                }
                status_chip(
                    ui,
                    if app.snapshot.service_loaded {
                        "服务运行中"
                    } else {
                        "服务未运行"
                    },
                    if app.snapshot.service_loaded {
                        GREEN
                    } else {
                        RED
                    },
                    true,
                );
                status_chip(ui, "本地存储 · 不上传", BLUE, false);
            },
        );
    });
    if let Some(error) = &app.last_error {
        ui.add_space(10.0);
        egui::Frame::default()
            .fill(RED.linear_multiply(0.08))
            .stroke(Stroke::new(1.0, RED.linear_multiply(0.2)))
            .corner_radius(8.0)
            .inner_margin(egui::Margin::symmetric(12, 8))
            .show(ui, |ui| {
                ui.label(RichText::new(error).size(13.0).color(RED));
            });
    }
}

fn status_chip(ui: &mut egui::Ui, label: &str, color: Color32, dot_first: bool) {
    egui::Frame::default()
        .fill(color.linear_multiply(0.09))
        .stroke(Stroke::new(1.0, color.linear_multiply(0.35)))
        .corner_radius(6.0)
        .inner_margin(egui::Margin::symmetric(14, 10))
        .show(ui, |ui| {
            ui.horizontal(|ui| {
                if dot_first {
                    dot(ui, color);
                    ui.add_space(5.0);
                } else {
                    lock_icon(ui, color);
                    ui.add_space(5.0);
                }
                ui.label(RichText::new(label).size(13.0).strong().color(color));
            });
        });
}

fn icon_button(ui: &mut egui::Ui, icon: IconKind, tooltip: &str) -> egui::Response {
    let response = ui
        .add_sized(
            [46.0, 42.0],
            egui::Button::new("")
                .fill(PANEL_BG)
                .stroke(Stroke::new(1.0, BORDER))
                .corner_radius(6.0),
        )
        .on_hover_text(tooltip);
    let rect = response.rect;
    let mut child = ui.new_child(
        egui::UiBuilder::new()
            .max_rect(rect)
            .layout(Layout::centered_and_justified(egui::Direction::LeftToRight)),
    );
    nav_icon(&mut child, icon, TEXT);
    response
}

fn kpi_row(ui: &mut egui::Ui, app: &WapcDesktopApp) {
    let gap = 12.0;
    let width = ((ui.available_width() - gap * 3.0) / 4.0).clamp(140.0, 190.0);
    let daily_tokens = daily_token_values(&app.snapshot, None);
    ui.horizontal_top(|ui| {
        ui.spacing_mut().item_spacing.x = gap;
        kpi_card(
            ui,
            width,
            KpiCard {
                label: "今日 Token",
                value: format_number(app.snapshot.total_tokens_today()),
                caption: "近 7 日趋势",
                accent: BLUE,
                icon: IconKind::Database,
            },
            &daily_tokens,
        );
        kpi_card(
            ui,
            width,
            KpiCard {
                label: "今日会话数",
                value: format_number(app.snapshot.total_records_today()),
                caption: "本机记录聚合",
                accent: TEAL,
                icon: IconKind::Chat,
            },
            &daily_tokens,
        );
        kpi_card(
            ui,
            width,
            KpiCard {
                label: "预估费用 (USD)",
                value: format_currency(app.snapshot.estimated_cost_today()),
                caption: "按工具原始费用",
                accent: PURPLE,
                icon: IconKind::Cost,
            },
            &daily_tokens,
        );
        kpi_card(
            ui,
            width,
            KpiCard {
                label: "已索引事件",
                value: format_number(app.snapshot.scan_records as u64),
                caption: "后台扫描入库",
                accent: ORANGE,
                icon: IconKind::Events,
            },
            &daily_tokens,
        );
    });
}

struct KpiCard<'a> {
    label: &'a str,
    value: String,
    caption: &'a str,
    accent: Color32,
    icon: IconKind,
}

fn kpi_card(ui: &mut egui::Ui, width: f32, card: KpiCard<'_>, trend: &[u64]) {
    ui.allocate_ui_with_layout(
        Vec2::new(width, 132.0),
        Layout::top_down(Align::LEFT),
        |ui| {
            ui.set_width(width);
            panel_frame()
                .inner_margin(egui::Margin::same(16))
                .show(ui, |ui| {
                    ui.set_width((width - 32.0).max(1.0));
                    ui.set_min_height(120.0);
                    ui.horizontal(|ui| {
                        icon_badge(ui, card.icon, card.accent);
                        ui.add_space(10.0);
                        ui.vertical(|ui| {
                            ui.label(RichText::new(card.label).size(13.0).strong().color(MUTED));
                            ui.add_space(6.0);
                            ui.label(RichText::new(card.value).size(24.0).strong().color(TEXT));
                        });
                    });
                    ui.add_space(14.0);
                    ui.horizontal(|ui| {
                        ui.label(
                            RichText::new(card.caption)
                                .size(12.0)
                                .strong()
                                .color(card.accent),
                        );
                        ui.label(RichText::new("↑").size(13.0).strong().color(card.accent));
                    });
                    ui.add_space(6.0);
                    sparkline(ui, trend, card.accent);
                });
        },
    );
}

fn trend_panel(ui: &mut egui::Ui, snapshot: &DesktopSnapshot) {
    panel_frame()
        .inner_margin(egui::Margin::same(18))
        .show(ui, |ui| {
            ui.set_min_height(296.0);
            ui.horizontal(|ui| {
                ui.vertical(|ui| {
                    section_title(ui, "Token 使用趋势（近 7 天）", "");
                    ui.add_space(4.0);
                    legend_row(ui, snapshot);
                });
                ui.with_layout(Layout::right_to_left(Align::TOP), |ui| {
                    small_select(ui, "近 7 天");
                });
            });
            ui.add_space(16.0);
            draw_trend_chart(ui, snapshot, ui.available_width(), 184.0);
        });
}

fn service_panel(ui: &mut egui::Ui, app: &WapcDesktopApp) {
    panel_frame()
        .inner_margin(egui::Margin::same(18))
        .show(ui, |ui| {
            ui.set_min_height(296.0);
            section_title(ui, "服务状态", "");
            ui.add_space(12.0);
            status_row(ui, "LaunchAgent", app.snapshot.service_installed, "已安装");
            status_row(
                ui,
                "运行状态",
                app.snapshot.service_loaded,
                if app.snapshot.service_loaded {
                    "运行中"
                } else {
                    "未运行"
                },
            );
            status_row(ui, "执行间隔", true, "每 15 分钟");
            status_row(
                ui,
                "下次运行",
                app.snapshot.service_loaded,
                &format!("{}", (Local::now() + Duration::minutes(15)).format("%H:%M")),
            );
            ui.add_space(14.0);
            ui.separator();
            ui.add_space(12.0);
            section_title(ui, "数据源状态", "");
            ui.add_space(8.0);
            for source in data_sources(&app.home) {
                source_row(ui, &source.0, source.1, &source.2);
            }
            ui.add_space(14.0);
            let _ = outline_button(ui, "查看详细检查报告");
        });
}

fn tool_table(ui: &mut egui::Ui, snapshot: &DesktopSnapshot) {
    panel_frame()
        .inner_margin(egui::Margin::same(14))
        .show(ui, |ui| {
            ui.set_min_height(246.0);
            section_title(ui, "工具使用详情", "");
            ui.add_space(10.0);
            egui::Grid::new("tool-table")
                .striped(true)
                .num_columns(6)
                .spacing(Vec2::new(18.0, 11.0))
                .min_col_width(58.0)
                .show(ui, |ui| {
                    table_head(ui, "工具");
                    table_head(ui, "会话数");
                    table_head(ui, "Token 总数");
                    table_head(ui, "输入 Token");
                    table_head(ui, "输出 Token");
                    table_head(ui, "费用");
                    ui.end_row();
                    for (index, summary) in snapshot.tools.iter().take(5).enumerate() {
                        tool_cell(ui, &summary.name, tool_color(index));
                        table_value(ui, &format_number(summary.records), false);
                        table_value(ui, &format_number(summary.usage.total()), true);
                        table_value(ui, &format_compact(summary.usage.input), false);
                        table_value(ui, &format_compact(summary.usage.output), false);
                        table_value(ui, &format_currency(summary.cost_usd), false);
                        ui.end_row();
                    }
                });
            ui.add_space(8.0);
            let _ = outline_button(ui, "查看全部工具");
        });
}

fn project_table(ui: &mut egui::Ui, snapshot: &DesktopSnapshot) {
    panel_frame()
        .inner_margin(egui::Margin::same(14))
        .show(ui, |ui| {
            ui.set_min_height(246.0);
            section_title(ui, "项目使用详情", "");
            ui.add_space(10.0);
            egui::Grid::new("project-table")
                .striped(true)
                .num_columns(3)
                .spacing(Vec2::new(8.0, 11.0))
                .min_col_width(42.0)
                .show(ui, |ui| {
                    table_head(ui, "项目路径");
                    table_head(ui, "记录数");
                    table_head(ui, "Token 总数");
                    ui.end_row();
                    for summary in snapshot.projects.iter().take(6) {
                        ui.label(
                            RichText::new(truncate_middle(&summary.name, 18))
                                .size(13.0)
                                .color(TEXT),
                        );
                        table_value(ui, &format_number(summary.records), false);
                        table_value(ui, &format_number(summary.usage.total()), true);
                        ui.end_row();
                    }
                });
            ui.add_space(8.0);
            let _ = outline_button(ui, "查看全部项目");
        });
}

fn footer(ui: &mut egui::Ui, app: &WapcDesktopApp) {
    ui.horizontal(|ui| {
        ui.label(
            RichText::new(format!(
                "数据库：{}",
                truncate_middle(&app.db.display().to_string(), 42)
            ))
            .size(12.0)
            .color(MUTED),
        );
        ui.add_space(20.0);
        dot(ui, if app.snapshot.db_exists { GREEN } else { RED });
        ui.label(
            RichText::new(if app.snapshot.db_exists {
                "数据正常"
            } else {
                "数据库未创建"
            })
            .size(12.0)
            .color(MUTED),
        );
        ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
            ui.label(RichText::new(&app.status).size(12.0).color(MUTED));
        });
    });
}

fn panel_frame() -> egui::Frame {
    egui::Frame::default()
        .fill(PANEL_BG)
        .stroke(Stroke::new(1.0, BORDER))
        .corner_radius(8.0)
}

fn section_title(ui: &mut egui::Ui, title: &str, subtitle: &str) {
    ui.label(RichText::new(title).size(16.0).strong().color(TEXT));
    if !subtitle.is_empty() {
        ui.label(RichText::new(subtitle).size(12.0).color(MUTED));
    }
}

fn table_head(ui: &mut egui::Ui, label: &str) {
    ui.label(
        RichText::new(label)
            .size(12.0)
            .strong()
            .color(Color32::from_rgb(82, 99, 123)),
    );
}

fn table_value(ui: &mut egui::Ui, value: &str, strong: bool) {
    let text = if strong {
        RichText::new(value).size(13.0).strong().color(TEXT)
    } else {
        RichText::new(value).size(13.0).color(MUTED)
    };
    ui.label(text);
}

fn tool_cell(ui: &mut egui::Ui, name: &str, color: Color32) {
    ui.horizontal(|ui| {
        let (rect, _) = ui.allocate_exact_size(Vec2::splat(18.0), Sense::hover());
        ui.painter().rect_filled(rect, 5.0, color);
        ui.label(
            RichText::new(tool_display_name(name))
                .size(13.0)
                .strong()
                .color(TEXT),
        );
    });
}

fn status_row(ui: &mut egui::Ui, label: &str, ok: bool, detail: &str) {
    ui.horizontal(|ui| {
        check_icon(ui, if ok { GREEN } else { RED });
        ui.label(RichText::new(label).size(13.0).color(TEXT));
        ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
            ui.label(
                RichText::new(detail)
                    .size(13.0)
                    .strong()
                    .color(if ok { GREEN } else { RED }),
            );
        });
    });
    ui.add_space(8.0);
}

fn source_row(ui: &mut egui::Ui, name: &str, ok: bool, detail: &str) {
    ui.horizontal(|ui| {
        ui.label(RichText::new(name).size(13.0).color(TEXT));
        ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
            check_icon(ui, if ok { GREEN } else { RED });
            ui.label(
                RichText::new(truncate_middle(detail, 32))
                    .size(12.0)
                    .color(MUTED),
            );
        });
    });
    ui.add_space(7.0);
}

fn outline_button(ui: &mut egui::Ui, label: &str) -> egui::Response {
    ui.add_sized(
        [164.0, 34.0],
        egui::Button::new(RichText::new(format!("{label}  >")).size(13.0).color(TEXT))
            .fill(PANEL_BG)
            .stroke(Stroke::new(1.0, BORDER))
            .corner_radius(6.0),
    )
}

fn small_select(ui: &mut egui::Ui, label: &str) {
    egui::Frame::default()
        .fill(PANEL_SOFT)
        .stroke(Stroke::new(1.0, BORDER))
        .corner_radius(6.0)
        .inner_margin(egui::Margin::symmetric(14, 8))
        .show(ui, |ui| {
            ui.label(RichText::new(format!("{label}  ˅")).size(13.0).color(TEXT));
        });
}

fn legend_row(ui: &mut egui::Ui, snapshot: &DesktopSnapshot) {
    ui.horizontal(|ui| {
        for (index, summary) in snapshot.tools.iter().take(4).enumerate() {
            dot(ui, tool_color(index));
            ui.label(
                RichText::new(tool_display_name(&summary.name))
                    .size(12.0)
                    .color(MUTED),
            );
            ui.add_space(12.0);
        }
    });
}

fn draw_trend_chart(ui: &mut egui::Ui, snapshot: &DesktopSnapshot, width: f32, height: f32) {
    let (rect, _) = ui.allocate_exact_size(Vec2::new(width, height), Sense::hover());
    let painter = ui.painter();
    painter.rect_filled(rect, 8.0, Color32::from_rgb(252, 253, 255));
    let plot = Rect::from_min_max(
        egui::pos2(rect.left() + 48.0, rect.top() + 12.0),
        egui::pos2(rect.right() - 18.0, rect.bottom() - 28.0),
    );
    let tools = snapshot
        .tools
        .iter()
        .take(4)
        .map(|summary| summary.name.clone())
        .collect::<Vec<_>>();
    let daily = daily_map(snapshot);
    let mut max_value = 1_u64;
    for day in &snapshot.trend_days {
        for tool in &tools {
            max_value = max_value.max(
                daily
                    .get(day)
                    .and_then(|tools| tools.get(tool))
                    .copied()
                    .unwrap_or(0),
            );
        }
    }
    for step in 0..=3 {
        let ratio = step as f32 / 3.0;
        let y = plot.bottom() - plot.height() * ratio;
        painter.line_segment(
            [egui::pos2(plot.left(), y), egui::pos2(plot.right(), y)],
            Stroke::new(1.0, Color32::from_rgb(224, 232, 242)),
        );
        let label = format_compact((max_value as f32 * ratio) as u64);
        painter.text(
            egui::pos2(rect.left() + 4.0, y),
            Align2::LEFT_CENTER,
            label,
            FontId::proportional(11.0),
            MUTED,
        );
    }
    let day_count = snapshot.trend_days.len().max(1) as f32;
    for (index, day) in snapshot.trend_days.iter().enumerate() {
        let x = if snapshot.trend_days.len() == 1 {
            plot.center().x
        } else {
            plot.left() + plot.width() * index as f32 / (day_count - 1.0)
        };
        painter.line_segment(
            [egui::pos2(x, plot.top()), egui::pos2(x, plot.bottom())],
            Stroke::new(1.0, Color32::from_rgb(235, 240, 248)),
        );
        painter.text(
            egui::pos2(x, rect.bottom() - 12.0),
            Align2::CENTER_CENTER,
            day_short(day),
            FontId::proportional(11.0),
            MUTED,
        );
    }
    for (tool_index, tool) in tools.iter().enumerate() {
        let color = tool_color(tool_index);
        let mut previous = None;
        for (day_index, day) in snapshot.trend_days.iter().enumerate() {
            let value = daily
                .get(day)
                .and_then(|tools| tools.get(tool))
                .copied()
                .unwrap_or(0);
            let x = if snapshot.trend_days.len() == 1 {
                plot.center().x
            } else {
                plot.left() + plot.width() * day_index as f32 / (day_count - 1.0)
            };
            let y = plot.bottom() - plot.height() * value as f32 / max_value as f32;
            let point = egui::pos2(x, y);
            if let Some(last) = previous {
                painter.line_segment([last, point], Stroke::new(2.0, color));
            }
            painter.circle_filled(point, 3.0, PANEL_BG);
            painter.circle_stroke(point, 3.0, Stroke::new(2.0, color));
            previous = Some(point);
        }
    }
}

fn sparkline(ui: &mut egui::Ui, values: &[u64], color: Color32) {
    let (rect, _) = ui.allocate_exact_size(Vec2::new(ui.available_width(), 28.0), Sense::hover());
    let max_value = values.iter().copied().max().unwrap_or(1).max(1);
    let count = values.len().max(2) as f32;
    let mut previous = None;
    for (index, value) in values.iter().enumerate() {
        let x = rect.left() + rect.width() * index as f32 / (count - 1.0);
        let y = rect.bottom() - rect.height() * *value as f32 / max_value as f32;
        let point = egui::pos2(x, y);
        if let Some(last) = previous {
            ui.painter()
                .line_segment([last, point], Stroke::new(1.8, color));
        }
        previous = Some(point);
    }
}

fn data_sources(home: &Path) -> Vec<(String, bool, String)> {
    let sources = [
        ("Claude Code", home.join(".claude/projects")),
        ("Codex", home.join(".codex/sessions")),
        ("Gemini CLI", home.join(".gemini/tmp")),
        ("OpenCode", home.join(".local/share/opencode/storage")),
    ];
    sources
        .into_iter()
        .map(|(name, path)| {
            (
                name.to_string(),
                path.exists(),
                path.strip_prefix(home)
                    .map(|path| format!("~/{}", path.display()))
                    .unwrap_or_else(|_| path.display().to_string()),
            )
        })
        .collect()
}

fn daily_map(snapshot: &DesktopSnapshot) -> BTreeMap<String, BTreeMap<String, u64>> {
    let mut map = BTreeMap::new();
    for row in &snapshot.daily {
        map.entry(row.day.clone())
            .or_insert_with(BTreeMap::new)
            .insert(row.tool.clone(), row.total_tokens);
    }
    map
}

fn daily_token_values(snapshot: &DesktopSnapshot, tool: Option<&str>) -> Vec<u64> {
    let daily = daily_map(snapshot);
    snapshot
        .trend_days
        .iter()
        .map(|day| {
            let Some(tools) = daily.get(day) else {
                return 0;
            };
            match tool {
                Some(tool) => tools.get(tool).copied().unwrap_or(0),
                None => tools.values().copied().sum(),
            }
        })
        .collect()
}

fn recent_days(days: i64) -> Vec<String> {
    let today = Local::now().date_naive();
    (0..days)
        .rev()
        .map(|offset| {
            (today - Duration::days(offset))
                .format("%Y-%m-%d")
                .to_string()
        })
        .collect()
}

fn day_short(day: &str) -> String {
    day.get(5..).unwrap_or(day).to_string()
}

#[derive(Clone, Copy)]
enum IconKind {
    Home,
    Tools,
    Folder,
    Shield,
    Server,
    Settings,
    Help,
    Info,
    Refresh,
    Database,
    Chat,
    Cost,
    Events,
}

fn nav_icon(ui: &mut egui::Ui, icon: IconKind, color: Color32) {
    let (rect, _) = ui.allocate_exact_size(Vec2::splat(17.0), Sense::hover());
    draw_icon(ui, rect, icon, color);
}

fn icon_badge(ui: &mut egui::Ui, icon: IconKind, color: Color32) {
    let (rect, _) = ui.allocate_exact_size(Vec2::splat(44.0), Sense::hover());
    ui.painter()
        .circle_filled(rect.center(), 22.0, color.linear_multiply(0.12));
    let icon_rect = Rect::from_center_size(rect.center(), Vec2::splat(20.0));
    draw_icon(ui, icon_rect, icon, color);
}

fn draw_icon(ui: &mut egui::Ui, rect: Rect, icon: IconKind, color: Color32) {
    let painter = ui.painter();
    let stroke = Stroke::new(1.7, color);
    match icon {
        IconKind::Home => {
            painter.line_segment(
                [
                    egui::pos2(rect.left() + 2.0, rect.center().y),
                    egui::pos2(rect.center().x, rect.top() + 2.0),
                ],
                stroke,
            );
            painter.line_segment(
                [
                    egui::pos2(rect.center().x, rect.top() + 2.0),
                    egui::pos2(rect.right() - 2.0, rect.center().y),
                ],
                stroke,
            );
            painter.rect_stroke(
                Rect::from_min_max(
                    egui::pos2(rect.left() + 4.0, rect.center().y - 1.0),
                    egui::pos2(rect.right() - 4.0, rect.bottom() - 2.0),
                ),
                2.0,
                stroke,
                egui::StrokeKind::Inside,
            );
        }
        IconKind::Tools => {
            painter.line_segment(
                [
                    egui::pos2(rect.left() + 3.0, rect.bottom() - 3.0),
                    egui::pos2(rect.right() - 3.0, rect.top() + 3.0),
                ],
                stroke,
            );
            painter.circle_stroke(egui::pos2(rect.left() + 5.0, rect.top() + 5.0), 3.0, stroke);
            painter.circle_stroke(
                egui::pos2(rect.right() - 5.0, rect.bottom() - 5.0),
                3.0,
                stroke,
            );
        }
        IconKind::Folder => {
            painter.rect_stroke(rect.shrink(2.0), 2.0, stroke, egui::StrokeKind::Inside);
            painter.line_segment(
                [
                    egui::pos2(rect.left() + 3.0, rect.top() + 6.0),
                    egui::pos2(rect.right() - 3.0, rect.top() + 6.0),
                ],
                stroke,
            );
        }
        IconKind::Shield => {
            painter.line_segment(
                [
                    egui::pos2(rect.center().x, rect.top() + 2.0),
                    egui::pos2(rect.right() - 3.0, rect.top() + 6.0),
                ],
                stroke,
            );
            painter.line_segment(
                [
                    egui::pos2(rect.right() - 3.0, rect.top() + 6.0),
                    egui::pos2(rect.center().x, rect.bottom() - 2.0),
                ],
                stroke,
            );
            painter.line_segment(
                [
                    egui::pos2(rect.center().x, rect.bottom() - 2.0),
                    egui::pos2(rect.left() + 3.0, rect.top() + 6.0),
                ],
                stroke,
            );
            painter.line_segment(
                [
                    egui::pos2(rect.left() + 3.0, rect.top() + 6.0),
                    egui::pos2(rect.center().x, rect.top() + 2.0),
                ],
                stroke,
            );
        }
        IconKind::Server | IconKind::Database => {
            for offset in [3.0, 8.0, 13.0] {
                painter.line_segment(
                    [
                        egui::pos2(rect.left() + 3.0, rect.top() + offset),
                        egui::pos2(rect.right() - 3.0, rect.top() + offset),
                    ],
                    stroke,
                );
            }
        }
        IconKind::Settings => {
            painter.circle_stroke(rect.center(), 5.0, stroke);
            painter.circle_filled(rect.center(), 1.8, color);
        }
        IconKind::Help => {
            painter.circle_stroke(rect.center(), 7.0, stroke);
            painter.text(
                rect.center(),
                Align2::CENTER_CENTER,
                "?",
                FontId::proportional(13.0),
                color,
            );
        }
        IconKind::Info => {
            painter.circle_stroke(rect.center(), 7.0, stroke);
            painter.text(
                rect.center(),
                Align2::CENTER_CENTER,
                "i",
                FontId::proportional(13.0),
                color,
            );
        }
        IconKind::Refresh => {
            painter.circle_stroke(rect.center(), 6.0, stroke);
            painter.line_segment(
                [
                    egui::pos2(rect.right() - 6.0, rect.top() + 4.0),
                    egui::pos2(rect.right() - 2.0, rect.top() + 4.0),
                ],
                stroke,
            );
        }
        IconKind::Chat => {
            painter.rect_stroke(rect.shrink(3.0), 5.0, stroke, egui::StrokeKind::Inside);
            painter.line_segment(
                [
                    egui::pos2(rect.left() + 7.0, rect.bottom() - 4.0),
                    egui::pos2(rect.left() + 5.0, rect.bottom() - 1.0),
                ],
                stroke,
            );
            for x in [rect.left() + 7.0, rect.center().x, rect.right() - 7.0] {
                painter.circle_filled(egui::pos2(x, rect.center().y), 1.2, color);
            }
        }
        IconKind::Cost => {
            painter.circle_stroke(rect.center(), 8.0, stroke);
            painter.text(
                rect.center(),
                Align2::CENTER_CENTER,
                "$",
                FontId::proportional(15.0),
                color,
            );
        }
        IconKind::Events => {
            for (index, height) in [8.0, 13.0, 10.0].iter().enumerate() {
                let x = rect.left() + 4.0 + index as f32 * 5.0;
                painter.line_segment(
                    [
                        egui::pos2(x, rect.bottom() - 3.0),
                        egui::pos2(x, rect.bottom() - 3.0 - height),
                    ],
                    Stroke::new(3.0, color),
                );
            }
        }
    }
}

fn lock_icon(ui: &mut egui::Ui, color: Color32) {
    let (rect, _) = ui.allocate_exact_size(Vec2::splat(14.0), Sense::hover());
    let painter = ui.painter();
    painter.rect_stroke(
        Rect::from_min_max(
            egui::pos2(rect.left() + 3.0, rect.center().y - 1.0),
            egui::pos2(rect.right() - 3.0, rect.bottom() - 2.0),
        ),
        2.0,
        Stroke::new(1.5, color),
        egui::StrokeKind::Inside,
    );
    painter.line_segment(
        [
            egui::pos2(rect.left() + 5.0, rect.center().y - 1.0),
            egui::pos2(rect.left() + 5.0, rect.top() + 5.0),
        ],
        Stroke::new(1.5, color),
    );
    painter.line_segment(
        [
            egui::pos2(rect.left() + 5.0, rect.top() + 5.0),
            egui::pos2(rect.right() - 5.0, rect.top() + 5.0),
        ],
        Stroke::new(1.5, color),
    );
    painter.line_segment(
        [
            egui::pos2(rect.right() - 5.0, rect.top() + 5.0),
            egui::pos2(rect.right() - 5.0, rect.center().y - 1.0),
        ],
        Stroke::new(1.5, color),
    );
}

fn dot(ui: &mut egui::Ui, color: Color32) {
    let (rect, _) = ui.allocate_exact_size(Vec2::splat(9.0), Sense::hover());
    ui.painter().circle_filled(rect.center(), 4.5, color);
}

fn check_icon(ui: &mut egui::Ui, color: Color32) {
    let (rect, _) = ui.allocate_exact_size(Vec2::splat(16.0), Sense::hover());
    ui.painter()
        .circle_stroke(rect.center(), 6.0, Stroke::new(1.4, color));
    ui.painter().line_segment(
        [
            egui::pos2(rect.left() + 5.0, rect.center().y),
            egui::pos2(rect.center().x - 1.0, rect.bottom() - 5.0),
        ],
        Stroke::new(1.4, color),
    );
    ui.painter().line_segment(
        [
            egui::pos2(rect.center().x - 1.0, rect.bottom() - 5.0),
            egui::pos2(rect.right() - 4.0, rect.top() + 5.0),
        ],
        Stroke::new(1.4, color),
    );
}

fn tool_color(index: usize) -> Color32 {
    [BLUE, TEAL, PURPLE, ORANGE, Color32::from_rgb(75, 85, 99)][index % 5]
}

fn tool_display_name(name: &str) -> &str {
    match name {
        "claude" => "Claude Code",
        "codex" => "Codex",
        "gemini" => "Gemini CLI",
        "opencode" => "OpenCode",
        _ => name,
    }
}

pub fn run_desktop() -> Result<()> {
    let home = dirs_next::home_dir().context("cannot resolve home directory")?;
    let db = home.join(".wapc/wapc.db");
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_title("WAPC")
            .with_inner_size([1180.0, 960.0])
            .with_min_inner_size([980.0, 700.0]),
        ..Default::default()
    };
    eframe::run_native(
        "WAPC",
        options,
        Box::new(move |cc| {
            install_system_fonts(&cc.egui_ctx);
            cc.egui_ctx.set_visuals(Visuals::light());
            Ok(Box::new(WapcDesktopApp::new(home, db)))
        }),
    )
    .map_err(|error| anyhow::anyhow!(error.to_string()))
}

fn install_system_fonts(ctx: &egui::Context) {
    let Some(font_bytes) = load_cjk_font() else {
        return;
    };
    let mut fonts = FontDefinitions::default();
    fonts.font_data.insert(
        "wapc-cjk".to_string(),
        Arc::new(FontData::from_owned(font_bytes)),
    );
    for family in [FontFamily::Proportional, FontFamily::Monospace] {
        fonts
            .families
            .entry(family)
            .or_default()
            .insert(0, "wapc-cjk".to_string());
    }
    ctx.set_fonts(fonts);
}

fn load_cjk_font() -> Option<Vec<u8>> {
    for path in [
        "/System/Library/Fonts/Hiragino Sans GB.ttc",
        "/System/Library/Fonts/STHeiti Medium.ttc",
        "/System/Library/Fonts/STHeiti Light.ttc",
    ] {
        if let Ok(bytes) = std::fs::read(path) {
            return Some(bytes);
        }
    }
    None
}

pub fn scan_into_store(home: &Path, db: &Path) -> Result<usize> {
    let records = scanner::scan_home(home)?;
    let store = UsageStore::open(db)?;
    store.upsert_records(&records)
}

pub fn load_snapshot(home: &Path, db: &Path) -> Result<DesktopSnapshot> {
    let store = UsageStore::open(db)?;
    let today = Local::now().format("%Y-%m-%d").to_string();
    let trend_days = recent_days(7);
    Ok(DesktopSnapshot {
        today: store.summary_by_tool_filtered(None, Some(&today))?,
        tools: store.summary_by_tool(None)?,
        projects: store.summary_by_project_filtered(None, None)?,
        daily: store.daily_tool_totals(&trend_days)?,
        trend_days,
        scan_records: scanner::scan_home(home)?.len(),
        service_installed: launchd::is_installed(home),
        service_loaded: launchd::is_loaded(),
        db_exists: db.exists(),
    })
}

fn format_number(value: u64) -> String {
    let raw = value.to_string();
    let mut result = String::new();
    for (index, ch) in raw.chars().rev().enumerate() {
        if index > 0 && index % 3 == 0 {
            result.push(',');
        }
        result.push(ch);
    }
    result.chars().rev().collect()
}

fn format_currency(value: f64) -> String {
    format!("${value:.2}")
}

fn format_compact(value: u64) -> String {
    if value >= 1_000_000_000 {
        format!("{:.1}B", value as f64 / 1_000_000_000.0)
    } else if value >= 1_000_000 {
        format!("{:.1}M", value as f64 / 1_000_000.0)
    } else if value >= 1_000 {
        format!("{:.1}K", value as f64 / 1_000.0)
    } else {
        value.to_string()
    }
}

fn truncate_middle(value: &str, max_chars: usize) -> String {
    if value.chars().count() <= max_chars {
        return value.to_string();
    }
    let keep = max_chars.saturating_sub(1) / 2;
    let prefix: String = value.chars().take(keep).collect();
    let suffix: String = value
        .chars()
        .rev()
        .take(keep)
        .collect::<String>()
        .chars()
        .rev()
        .collect();
    format!("{prefix}…{suffix}")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn formats_large_numbers_with_grouping() {
        assert_eq!(format_number(9), "9");
        assert_eq!(format_number(1234), "1,234");
        assert_eq!(format_number(123456789), "123,456,789");
    }

    #[test]
    fn snapshot_totals_today_summaries() {
        let snapshot = DesktopSnapshot {
            today: vec![UsageSummary {
                name: "codex".to_string(),
                records: 2,
                usage: crate::model::TokenUsage {
                    input: 10,
                    output: 20,
                    ..Default::default()
                },
                cost_usd: 0.0,
            }],
            ..Default::default()
        };

        assert_eq!(snapshot.total_tokens_today(), 30);
        assert_eq!(snapshot.total_records_today(), 2);
    }

    #[test]
    fn formats_compact_numbers_for_dense_tables() {
        assert_eq!(format_compact(999), "999");
        assert_eq!(format_compact(1_200), "1.2K");
        assert_eq!(format_compact(5_200_000), "5.2M");
        assert_eq!(format_compact(2_700_000_000), "2.7B");
    }

    #[test]
    fn snapshot_sums_estimated_cost_for_top_card() {
        let snapshot = DesktopSnapshot {
            today: vec![
                UsageSummary {
                    name: "codex".to_string(),
                    records: 1,
                    usage: crate::model::TokenUsage::default(),
                    cost_usd: 1.25,
                },
                UsageSummary {
                    name: "claude".to_string(),
                    records: 1,
                    usage: crate::model::TokenUsage::default(),
                    cost_usd: 2.5,
                },
            ],
            ..Default::default()
        };

        assert_eq!(snapshot.estimated_cost_today(), 3.75);
        assert_eq!(format_currency(snapshot.estimated_cost_today()), "$3.75");
    }
}
