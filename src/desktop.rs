//! Native desktop UI for WAPC.
//! @author codex

use std::{
    path::{Path, PathBuf},
    sync::Arc,
};

use anyhow::{Context, Result};
use chrono::Local;
use eframe::egui::{
    self, Align, Color32, FontData, FontDefinitions, FontFamily, Layout, RichText, Stroke, Vec2,
    Visuals,
};

use crate::{
    launchd, scanner,
    store::{UsageStore, UsageSummary},
};

#[derive(Clone, Debug, Default)]
pub struct DesktopSnapshot {
    pub today: Vec<UsageSummary>,
    pub tools: Vec<UsageSummary>,
    pub projects: Vec<UsageSummary>,
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
                self.status = format!("Refreshed at {}", Local::now().format("%H:%M:%S"));
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
                self.status = format!("Indexed {count} usage records");
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
                        Vec2::new(220.0, size.y),
                        Layout::top_down(Align::LEFT),
                        sidebar,
                    );
                    ui.allocate_ui_with_layout(
                        Vec2::new((size.x - 220.0).max(860.0), size.y),
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

const PAGE_BG: Color32 = Color32::from_rgb(244, 246, 248);
const SIDEBAR_BG: Color32 = Color32::from_rgb(18, 24, 33);
const PANEL_BG: Color32 = Color32::from_rgb(255, 255, 255);
const BORDER: Color32 = Color32::from_rgb(224, 229, 236);
const TEXT: Color32 = Color32::from_rgb(18, 25, 38);
const MUTED: Color32 = Color32::from_rgb(94, 107, 124);
const ACCENT: Color32 = Color32::from_rgb(35, 112, 236);
const TEAL: Color32 = Color32::from_rgb(21, 150, 122);

fn sidebar(ui: &mut egui::Ui) {
    egui::Frame::default()
        .fill(SIDEBAR_BG)
        .inner_margin(egui::Margin::symmetric(18, 20))
        .show(ui, |ui| {
            ui.set_width(220.0);
            ui.set_min_height(ui.available_height());
            ui.label(
                RichText::new("WAPC")
                    .size(28.0)
                    .strong()
                    .color(Color32::WHITE),
            );
            ui.label(
                RichText::new("AI Usage Console")
                    .size(13.0)
                    .color(Color32::from_rgb(148, 163, 184)),
            );
            ui.add_space(28.0);
            nav_item(ui, "Overview", true);
            nav_item(ui, "Tools", false);
            nav_item(ui, "Projects", false);
            nav_item(ui, "Privacy", false);
            nav_item(ui, "Service", false);
            ui.with_layout(Layout::bottom_up(Align::LEFT), |ui| {
                ui.label(
                    RichText::new("Local only")
                        .size(12.0)
                        .color(Color32::from_rgb(148, 163, 184)),
                );
                ui.label(
                    RichText::new("No prompt storage")
                        .size(12.0)
                        .color(Color32::from_rgb(148, 163, 184)),
                );
            });
        });
}

fn nav_item(ui: &mut egui::Ui, label: &str, active: bool) {
    let fill = if active {
        Color32::from_rgb(32, 43, 58)
    } else {
        SIDEBAR_BG
    };
    egui::Frame::default()
        .fill(fill)
        .corner_radius(8.0)
        .inner_margin(egui::Margin::symmetric(12, 9))
        .show(ui, |ui| {
            ui.set_width(176.0);
            let color = if active {
                Color32::WHITE
            } else {
                Color32::from_rgb(176, 190, 207)
            };
            ui.label(RichText::new(label).size(14.0).strong().color(color));
        });
    ui.add_space(6.0);
}

fn content(ui: &mut egui::Ui, app: &mut WapcDesktopApp) {
    egui::Frame::default()
        .fill(PAGE_BG)
        .inner_margin(egui::Margin::same(24))
        .show(ui, |ui| {
            ui.set_min_width(ui.available_width());
            header(ui, app);
            ui.add_space(20.0);
            kpi_row(ui, app);
            ui.add_space(18.0);
            ui.columns(2, |columns| {
                usage_panel(&mut columns[0], &app.snapshot);
                status_panel(&mut columns[1], app);
            });
            ui.add_space(18.0);
            ui.columns(2, |columns| {
                modern_table(
                    &mut columns[0],
                    "Tool Breakdown",
                    "工具",
                    &app.snapshot.tools,
                    true,
                );
                modern_table(
                    &mut columns[1],
                    "Project Usage",
                    "项目",
                    &app.snapshot.projects,
                    false,
                );
            });
        });
}

fn header(ui: &mut egui::Ui, app: &mut WapcDesktopApp) {
    ui.horizontal(|ui| {
        ui.vertical(|ui| {
            ui.label(
                RichText::new("AI Coding Usage")
                    .size(28.0)
                    .strong()
                    .color(TEXT),
            );
            ui.horizontal(|ui| {
                status_chip(ui, "Local only", TEAL);
                status_chip(
                    ui,
                    if app.snapshot.service_loaded {
                        "Service running"
                    } else {
                        "Service stopped"
                    },
                    if app.snapshot.service_loaded {
                        TEAL
                    } else {
                        Color32::from_rgb(210, 84, 74)
                    },
                );
                ui.label(RichText::new(&app.status).size(13.0).color(MUTED));
            });
        });
        ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
            if primary_button(ui, "Scan now").clicked() {
                app.scan_now();
            }
            if secondary_button(ui, "Refresh").clicked() {
                app.refresh();
            }
        });
    });
    if let Some(error) = &app.last_error {
        ui.add_space(8.0);
        ui.colored_label(Color32::from_rgb(180, 35, 24), error);
    }
}

fn status_chip(ui: &mut egui::Ui, label: &str, color: Color32) {
    egui::Frame::default()
        .fill(color.linear_multiply(0.12))
        .stroke(Stroke::new(1.0, color.linear_multiply(0.28)))
        .corner_radius(999.0)
        .inner_margin(egui::Margin::symmetric(10, 5))
        .show(ui, |ui| {
            ui.label(RichText::new(label).size(12.0).strong().color(color));
        });
}

fn primary_button(ui: &mut egui::Ui, label: &str) -> egui::Response {
    ui.add_sized(
        [96.0, 34.0],
        egui::Button::new(RichText::new(label).strong().color(Color32::WHITE))
            .fill(ACCENT)
            .corner_radius(8.0),
    )
}

fn secondary_button(ui: &mut egui::Ui, label: &str) -> egui::Response {
    ui.add_sized(
        [84.0, 34.0],
        egui::Button::new(RichText::new(label).strong().color(TEXT))
            .fill(PANEL_BG)
            .stroke(Stroke::new(1.0, BORDER))
            .corner_radius(8.0),
    )
}

fn kpi_row(ui: &mut egui::Ui, app: &WapcDesktopApp) {
    ui.horizontal(|ui| {
        kpi_card(
            ui,
            "Today Tokens",
            app.snapshot.total_tokens_today(),
            ACCENT,
        );
        kpi_card(ui, "Sessions", app.snapshot.total_records_today(), TEAL);
        kpi_card(
            ui,
            "Indexed Events",
            app.snapshot.scan_records as u64,
            Color32::from_rgb(111, 85, 214),
        );
        service_kpi(
            ui,
            app.snapshot.service_installed,
            app.snapshot.service_loaded,
        );
    });
}

fn kpi_card(ui: &mut egui::Ui, label: &str, value: u64, accent: Color32) {
    panel_frame()
        .inner_margin(egui::Margin::same(16))
        .show(ui, |ui| {
            ui.vertical(|ui| {
                ui.set_width(190.0);
                ui.set_min_height(92.0);
                ui.horizontal(|ui| {
                    ui.label(RichText::new(label).size(13.0).strong().color(MUTED));
                    ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                        dot(ui, accent);
                    });
                });
                ui.add_space(12.0);
                ui.label(
                    RichText::new(format_number(value))
                        .size(25.0)
                        .strong()
                        .color(TEXT),
                );
                ui.add_space(8.0);
                mini_bar(ui, 0.72, accent);
            });
        });
    ui.add_space(12.0);
}

fn service_kpi(ui: &mut egui::Ui, installed: bool, loaded: bool) {
    panel_frame()
        .inner_margin(egui::Margin::same(16))
        .show(ui, |ui| {
            ui.vertical(|ui| {
                ui.set_width(210.0);
                ui.set_min_height(92.0);
                ui.label(
                    RichText::new("Service Health")
                        .size(13.0)
                        .strong()
                        .color(MUTED),
                );
                ui.add_space(12.0);
                let color = if installed && loaded {
                    TEAL
                } else {
                    Color32::from_rgb(210, 84, 74)
                };
                ui.label(
                    RichText::new(if loaded { "Running" } else { "Stopped" })
                        .size(25.0)
                        .strong()
                        .color(color),
                );
                ui.add_space(8.0);
                ui.label(
                    RichText::new(format!("installed={installed} loaded={loaded}"))
                        .size(12.0)
                        .color(MUTED),
                );
            });
        });
}

fn usage_panel(ui: &mut egui::Ui, snapshot: &DesktopSnapshot) {
    panel_frame()
        .inner_margin(egui::Margin::same(18))
        .show(ui, |ui| {
            ui.set_min_height(230.0);
            section_title(
                ui,
                "Usage Mix",
                "按工具分布的累计 token，占比越高说明消耗越集中",
            );
            ui.add_space(14.0);
            let total = snapshot
                .tools
                .iter()
                .map(|summary| summary.usage.total())
                .sum::<u64>()
                .max(1);
            for (index, summary) in snapshot.tools.iter().take(5).enumerate() {
                let ratio = summary.usage.total() as f32 / total as f32;
                ui.horizontal(|ui| {
                    ui.set_height(28.0);
                    ui.label(
                        RichText::new(summary.name.as_str())
                            .size(13.0)
                            .strong()
                            .color(TEXT),
                    );
                    ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                        ui.label(
                            RichText::new(format_number(summary.usage.total()))
                                .size(13.0)
                                .color(MUTED),
                        );
                    });
                });
                progress_bar(ui, ratio, tool_color(index));
                ui.add_space(8.0);
            }
        });
}

fn status_panel(ui: &mut egui::Ui, app: &WapcDesktopApp) {
    panel_frame()
        .inner_margin(egui::Margin::same(18))
        .show(ui, |ui| {
            ui.set_min_height(230.0);
            section_title(
                ui,
                "Data Sources",
                "旁路读取本机 usage 文件，不保存对话正文",
            );
            ui.add_space(12.0);
            status_row(
                ui,
                "Database",
                app.snapshot.db_exists,
                &app.db.display().to_string(),
            );
            status_row(
                ui,
                "LaunchAgent",
                app.snapshot.service_loaded,
                if app.snapshot.service_loaded {
                    "background scan active"
                } else {
                    "not loaded"
                },
            );
            status_row(ui, "Privacy", true, "metadata only, prompt text excluded");
            ui.add_space(10.0);
            ui.separator();
            ui.add_space(8.0);
            for path in scanner::audit_paths(&app.home).into_iter().take(5) {
                status_row(
                    ui,
                    path.file_name()
                        .and_then(|name| name.to_str())
                        .unwrap_or("source"),
                    path.exists(),
                    &truncate_middle(&path.display().to_string(), 42),
                );
            }
        });
}

fn modern_table(
    ui: &mut egui::Ui,
    title: &str,
    first_column: &str,
    summaries: &[UsageSummary],
    show_token_breakdown: bool,
) {
    panel_frame()
        .inner_margin(egui::Margin::same(16))
        .show(ui, |ui| {
            ui.set_min_height(280.0);
            section_title(
                ui,
                title,
                if show_token_breakdown {
                    "工具调用和 token 明细"
                } else {
                    "项目目录消耗排行"
                },
            );
            ui.add_space(12.0);
            egui::Grid::new(title)
                .striped(true)
                .num_columns(if show_token_breakdown { 5 } else { 3 })
                .min_col_width(if show_token_breakdown { 68.0 } else { 54.0 })
                .spacing(Vec2::new(
                    if show_token_breakdown { 18.0 } else { 12.0 },
                    10.0,
                ))
                .show(ui, |ui| {
                    table_head(ui, first_column);
                    table_head(ui, "Records");
                    table_head(ui, "Tokens");
                    if show_token_breakdown {
                        table_head(ui, "Input");
                        table_head(ui, "Output");
                    }
                    ui.end_row();
                    for summary in summaries.iter().take(8) {
                        ui.label(
                            RichText::new(truncate_middle(
                                &summary.name,
                                if show_token_breakdown { 18 } else { 31 },
                            ))
                            .size(13.0)
                            .strong()
                            .color(TEXT),
                        );
                        ui.label(
                            RichText::new(format_number(summary.records))
                                .size(13.0)
                                .color(MUTED),
                        );
                        ui.label(
                            RichText::new(if show_token_breakdown {
                                format_number(summary.usage.total())
                            } else {
                                format_compact(summary.usage.total())
                            })
                            .size(13.0)
                            .strong()
                            .color(TEXT),
                        );
                        if show_token_breakdown {
                            ui.label(
                                RichText::new(format_compact(summary.usage.input))
                                    .size(13.0)
                                    .color(MUTED),
                            );
                            ui.label(
                                RichText::new(format_compact(summary.usage.output))
                                    .size(13.0)
                                    .color(MUTED),
                            );
                        }
                        ui.end_row();
                    }
                });
        });
}

fn panel_frame() -> egui::Frame {
    egui::Frame::default()
        .fill(PANEL_BG)
        .stroke(Stroke::new(1.0, BORDER))
        .corner_radius(10.0)
}

fn section_title(ui: &mut egui::Ui, title: &str, subtitle: &str) {
    ui.label(RichText::new(title).size(16.0).strong().color(TEXT));
    ui.label(RichText::new(subtitle).size(12.0).color(MUTED));
}

fn table_head(ui: &mut egui::Ui, label: &str) {
    ui.label(
        RichText::new(label)
            .size(12.0)
            .strong()
            .color(Color32::from_rgb(100, 116, 139)),
    );
}

fn status_row(ui: &mut egui::Ui, label: &str, ok: bool, detail: &str) {
    ui.horizontal(|ui| {
        dot(
            ui,
            if ok {
                TEAL
            } else {
                Color32::from_rgb(210, 84, 74)
            },
        );
        ui.label(RichText::new(label).size(13.0).strong().color(TEXT));
        ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
            ui.label(RichText::new(detail).size(12.0).color(MUTED));
        });
    });
}

fn dot(ui: &mut egui::Ui, color: Color32) {
    let (rect, _) = ui.allocate_exact_size(Vec2::splat(8.0), egui::Sense::hover());
    ui.painter().circle_filled(rect.center(), 4.0, color);
}

fn mini_bar(ui: &mut egui::Ui, ratio: f32, color: Color32) {
    progress_bar(ui, ratio, color);
}

fn progress_bar(ui: &mut egui::Ui, ratio: f32, color: Color32) {
    let width = ui.available_width().max(120.0);
    let (rect, _) = ui.allocate_exact_size(Vec2::new(width, 7.0), egui::Sense::hover());
    ui.painter()
        .rect_filled(rect, 999.0, Color32::from_rgb(235, 239, 245));
    let fill_width = rect.width() * ratio.clamp(0.0, 1.0);
    let fill = egui::Rect::from_min_size(rect.min, Vec2::new(fill_width, rect.height()));
    ui.painter().rect_filled(fill, 999.0, color);
}

fn tool_color(index: usize) -> Color32 {
    [
        ACCENT,
        TEAL,
        Color32::from_rgb(111, 85, 214),
        Color32::from_rgb(224, 135, 55),
        Color32::from_rgb(75, 85, 99),
    ][index % 5]
}

pub fn run_desktop() -> Result<()> {
    let home = dirs_next::home_dir().context("cannot resolve home directory")?;
    let db = home.join(".wapc/wapc.db");
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_title("WAPC")
            .with_inner_size([1280.0, 820.0])
            .with_min_inner_size([1100.0, 720.0]),
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
    Ok(DesktopSnapshot {
        today: store.summary_by_tool_filtered(None, Some(&today))?,
        tools: store.summary_by_tool(None)?,
        projects: store.summary_by_project_filtered(None, None)?,
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
}
