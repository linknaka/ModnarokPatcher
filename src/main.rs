#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod config;
mod manifest;
mod patcher;
mod troubleshooting;

use eframe::egui;
use std::sync::{Arc, Mutex};

#[derive(Default)]
struct UiState {
    status: String,
    busy: bool,
    help_status: String,
}

struct ModnarokApp {
    state: Arc<Mutex<UiState>>,
    runtime: tokio::runtime::Runtime,
}

impl ModnarokApp {
    fn new() -> Self {
        let runtime = tokio::runtime::Builder::new_multi_thread()
            .enable_all()
            .build()
            .expect("Não foi possível iniciar o runtime");
        Self {
            state: Arc::new(Mutex::new(UiState {
                status: "Pronto para verificar atualizações.".into(),
                ..Default::default()
            })),
            runtime,
        }
    }

    fn start_update(&self) {
        let state = self.state.clone();
        if let Ok(mut s) = state.lock() {
            if s.busy { return; }
            s.busy = true;
            s.status = "Verificando atualizações...".into();
        }
        self.runtime.spawn(async move {
            let result = patcher::update_client().await;
            if let Ok(mut s) = state.lock() {
                s.status = result.unwrap_or_else(|e| format!("Falha ao atualizar: {e:#}"));
                s.busy = false;
            }
        });
    }

    fn load_help(&self) {
        let state = self.state.clone();
        self.runtime.spawn(async move {
            let message = match troubleshooting::fetch_articles().await {
                Ok(items) if items.is_empty() => "Nenhum artigo publicado ainda.".into(),
                Ok(items) => format!("Central online acessível: {} artigos publicados. Abra a central para ler as instruções.", items.len()),
                Err(e) => format!("Não foi possível carregar a central online: {e:#}"),
            };
            if let Ok(mut s) = state.lock() { s.help_status = message; }
        });
    }
}

impl eframe::App for ModnarokApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        let (status, busy, help_status) = self.state.lock()
            .map(|s| (s.status.clone(), s.busy, s.help_status.clone()))
            .unwrap_or_default();

        egui::CentralPanel::default().show(ctx, |ui| {
            ui.heading("MODNAROK");
            ui.label("Atualizador oficial do cliente");
            ui.separator();
            ui.label("Verifica os arquivos publicados e baixa somente o que mudou.");
            ui.add_space(8.0);
            ui.horizontal(|ui| {
                if ui.add_enabled(!busy, egui::Button::new("Verificar e atualizar")).clicked() {
                    self.start_update();
                }
                if ui.button("Troubleshooting").clicked() {
                    self.load_help();
                }
                if ui.button("Site oficial").clicked() {
                    let _ = open_url(config::SITE_URL);
                }
            });
            ui.add_space(10.0);
            ui.label(&status);
            if busy { ui.spinner(); }
            if !help_status.is_empty() {
                ui.separator();
                ui.label(&help_status);
                if ui.button("Abrir central completa").clicked() {
                    let _ = open_url(config::TROUBLESHOOTING_PAGE);
                }
            }
            ui.with_layout(egui::Layout::bottom_up(egui::Align::LEFT), |ui| {
                ui.weak("MODNAROK Patcher — primeira base de desenvolvimento");
            });
        });
        ctx.request_repaint_after(std::time::Duration::from_millis(150));
    }
}

fn open_url(url: &str) -> anyhow::Result<()> {
    #[cfg(target_os = "windows")]
    { std::process::Command::new("cmd").args(["/C", "start", "", url]).spawn()?; }
    #[cfg(target_os = "macos")]
    { std::process::Command::new("open").arg(url).spawn()?; }
    #[cfg(all(unix, not(target_os = "macos")))]
    { std::process::Command::new("xdg-open").arg(url).spawn()?; }
    Ok(())
}

fn main() -> eframe::Result<()> {
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_inner_size([640.0, 400.0])
            .with_min_inner_size([540.0, 340.0]),
        ..Default::default()
    };
    eframe::run_native("MODNAROK Patcher", options, Box::new(|_cc| Ok(Box::new(ModnarokApp::new()))))
}
