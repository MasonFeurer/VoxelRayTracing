use crate::{Crosshair, GameState, Timers};
use egui::{Align, Layout, Ui};
use std::net::SocketAddr;
use std::time::SystemTime;
use client::common::resources::{Resources, WorldInfo, CURRENT_VERSION};

macro_rules! true_horizontal_centered {
    ($ui:ident, $left:expr, $right:expr) => {
        $ui.columns_const(|[col0, col1]| {
            col0.with_layout(Layout::top_down(Align::Max), $left);
            col1.with_layout(Layout::top_down(Align::Min), $right);
        });
    };
}

pub type WorldOptions = WorldInfo;

#[derive(Default)]
pub struct UiResponse {
    pub host_world: Option<String>,
    pub host_new_world: Option<WorldOptions>,
    pub join_game: Option<SocketAddr>,
}
//

// Underneath every menu, a render-pass will happen for rendering the game (if there is a GameState).
// So the UI doesn't need to concern itself with rendering the game.
#[derive(Default, Clone, Debug)]
pub struct UiState {
    pages: Vec<Page>,
    last_click: Option<SystemTime>,
    create_world_name: String,
    create_world_seed: String,
    create_world_asset_folder: String,
    join_world_address: String,
}
impl UiState {
    pub fn page_is_open(&self) -> bool {
        self.pages.len() > 0
    }
    pub fn open_page(&mut self, page: Page) {
        self.pages.push(page);
        self.last_click = Some(SystemTime::now());
    }
    pub fn close_page(&mut self) {
        _ = self.pages.pop();
        self.last_click = Some(SystemTime::now());
    }
    
    pub fn clear_pages(&mut self) {
        self.pages.clear();
    }

    fn is_click_recent(&self) -> bool {
        self.last_click.map(|t| SystemTime::now().duration_since(t).unwrap() < std::time::Duration::from_millis(20)).unwrap_or(false)
    }
}
#[derive(Clone, Copy, Debug)]
pub enum Page {
    PauseMenu,
    Options,
    MyWorlds,
    CreateWorld,
    JoinWorld,
}

pub fn show_title_screen(ui: &mut Ui, state: &mut UiState) -> UiResponse {
    let mut rs = UiResponse::default();

    ui.heading("Block World");
    ui.add_space(10.0);
    if ui.button("My Worlds").clicked() {
        state.open_page(Page::MyWorlds);
    }
    if ui.button("Join World").clicked() {
        state.open_page(Page::JoinWorld);
    }
    if ui.button("Options").clicked() {
        state.open_page(Page::Options);
    }
    if ui.button("Exit").clicked() {
        // TODO
    }
    rs
}

pub fn show_game_overlay(
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

pub fn show_ui_state(ui: &mut Ui, state: &mut UiState, resources: &Resources) -> Option<UiResponse> {
    Some(match state.pages.last()? {
        Page::PauseMenu => show_pause_menu(ui, state),
        Page::Options => show_options(ui, state, resources),
        Page::MyWorlds => show_my_worlds(ui, state, resources),
        Page::CreateWorld => show_create_world(ui, state, resources),
        Page::JoinWorld => show_join_world(ui, state),
    })
}

fn show_pause_menu(ui: &mut Ui, state: &mut UiState) -> UiResponse {
    if ui.button("Resume").clicked() {}
    UiResponse::default()
}

fn show_options(ui: &mut Ui, state: &mut UiState, resources: &Resources) -> UiResponse {
    if ui.button("Back").clicked() {
        state.close_page();
    }
    UiResponse::default()
}

fn show_my_worlds(ui: &mut Ui, state: &mut UiState, resources: &Resources) -> UiResponse {
    let mut rs = UiResponse::default();
    for world in &resources.worlds {
        ui.group(|ui| {
            true_horizontal_centered!(ui, 
                |ui| {
                    ui.heading(world.name.clone());
                },
                |ui| {
                    if ui.button("Play").clicked() {
                        rs.host_world = Some(world.name.clone());
                    }
                }
            );
        });
    }

    true_horizontal_centered!(ui,
        |ui| {
           if ui.button("Create World").clicked() {
                state.open_page(Page::CreateWorld);
            }
        },
        |ui| {
            if ui.button("Back").clicked() {
                state.close_page();
            }
        }
    );
    rs
}

fn show_create_world(ui: &mut Ui, state: &mut UiState, resources: &Resources) -> UiResponse {
    let mut rs = UiResponse::default();

    ui.text_edit_singleline(&mut state.create_world_name);
    let path = resources.path.join("worlds").join(&state.create_world_name);
    ui.label(format!("Stored at {}", path.display()));
    ui.text_edit_singleline(&mut state.create_world_seed);

    ui.separator();

    true_horizontal_centered!(ui,
        |ui| {
            if ui.button("Create").clicked() {
                state.pages.clear();
                rs.host_new_world = Some(WorldInfo {
                    name: state.create_world_name.clone(),
                    version: CURRENT_VERSION,
                    datapack: String::from("blockworld.vanilla"),
                    stylepack: String::from("blockworld.vanilla"),
                });
            }
        },
        |ui| {
            if ui.button("Back").clicked() {
                state.close_page();
            }
        }
    );
    rs
}

fn show_join_world(ui: &mut Ui, state: &mut UiState) -> UiResponse {
    if ui.button("Back").clicked() {
        state.close_page()
    }
    UiResponse::default()
}
