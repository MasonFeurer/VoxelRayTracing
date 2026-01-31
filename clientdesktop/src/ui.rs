use crate::{Crosshair, GameState, Timers};
use egui::Ui;
use std::net::SocketAddr;

#[derive(Default)]
pub struct UiResponse {
    pub new_ui_state: Option<UiState>,
    pub host_game: bool,
    pub join_game: Option<SocketAddr>,
}

// Underneath every menu, a render-pass will happen for rendering the game (if there is a GameState).
// So the UI doesn't need to concern itself with rendering the game.
#[derive(Default)]
pub enum UiState {
    GamePlay, // Only render game overlay (HUD)
    #[default]
    TitleScreen, // Render the title screen
    PauseMenu, // Render the pause UI
    Settings,
    CreateWorld {
        name: String,
        seed: String,
        asset_folder: String,
    },
    JoinWorld {
        address: String,
    },
}

pub fn draw_create_screen(ui: &mut Ui) -> UiResponse {
    let mut rs = UiResponse::default();

    ui.label("Title screen!");
    if ui.button("Create World").clicked() {
        rs.new_ui_state = Some(UiState::CreateWorld {
            name: Default::default(),
            seed: Default::default(),
            asset_folder: Default::default(),
        });
    }
    if ui.button("Join World").clicked() {
        rs.new_ui_state = Some(UiState::JoinWorld {
            address: Default::default(),
        });
    }
    if ui.button("Settings").clicked() {
        rs.new_ui_state = Some(UiState::Settings);
    }
    if ui.button("Exit").clicked() {
        // TODO
    }
    rs
}

pub fn draw_title_screen(ui: &mut Ui) -> UiResponse {
    let mut rs = UiResponse::default();

    ui.label("Title screen!");
    if ui.button("Create World").clicked() {
        rs.new_ui_state = Some(UiState::GamePlay);
        rs.join_game = Some(crate::local_server_addr());
        rs.host_game = true;
        // rs.new_ui_state = Some(UiState::CreateWorld {
        //     name: Default::default(),
        //     seed: Default::default(),
        //     asset_folder: Default::default(),
        // });
    }
    if ui.button("Join World").clicked() {
        rs.new_ui_state = Some(UiState::JoinWorld {
            address: Default::default(),
        });
    }
    if ui.button("Settings").clicked() {
        rs.new_ui_state = Some(UiState::Settings);
    }
    if ui.button("Exit").clicked() {
        // TODO
    }
    rs
}

pub fn draw_game_overlay(
    ui: &mut Ui,
    game: &mut GameState,
    crosshair: &mut Crosshair,
    timers: &Timers,
) {
    ui.visuals_mut().override_text_color = Some(egui::Color32::WHITE);

    ui.painter().rect_filled(
        ui.max_rect(),
        egui::CornerRadius::same(5),
        egui::Color32::from_rgba_unmultiplied(0, 0, 0, 200),
    );
    ui.add_space(40.0);
    ui.separator();
    {
        ui.collapsing("Crosshair", |ui| {
            ui.horizontal(|ui| {
                ui.heading("style: ");
                if ui.button("off").clicked() {
                    crosshair.style = 0;
                }
                if ui.button("dot").clicked() {
                    crosshair.style = 1;
                }
                if ui.button("cross").clicked() {
                    crosshair.style = 2;
                }
            });
            ui.add(egui::Slider::new(&mut crosshair.size, 1.0..=30.0).text("size"));
            ui.color_edit_button_rgba_unmultiplied(&mut crosshair.color);
        });
    }
    ui.separator();
    {
        ui.heading(format!("FPS: {}", timers.fps));
        ui.heading(format!(
            "pos: {:.2} {:.2} {:.2}",
            game.player.pos.x, game.player.pos.y, game.player.pos.z
        ));
        ui.horizontal(|ui| {
            ui.heading(format!("speed: {:.2}", game.player.speed));
            if ui.button("-").clicked() {
                game.player.speed -= 0.1;
                game.player.speed = game.player.speed.max(0.0);
            }
            if ui.button("+").clicked() {
                game.player.speed += 0.1;
            }
        });
    }
    ui.separator();
    {
        let (free, capacity) = game.world.chunk_alloc_status();
        let used = ((capacity - free) as f32 / capacity as f32) * 100.0;
        ui.heading(format!("world size: {}", game.world.size_in_chunks()));
        ui.heading(format!(
            "chunk count: {} ({})",
            game.world.chunk_count(),
            game.world.populated_count(),
        ));
        ui.heading(&format!("memory: %{used:.0}"));
    }
}
