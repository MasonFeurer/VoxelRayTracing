use crate::{GameState, Timers};
use egui::{Align, Layout, Rect, Ui};
use std::net::SocketAddr;
use std::str::FromStr;
use client::common::resources::{Resources, WorldInfo, CURRENT_VERSION};
use crate::graphics::Crosshair;

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
    pub quit_game: bool,
}
//

// Underneath every menu, a render-pass will happen for rendering the game (if there is a GameState).
// So the UI doesn't need to concern itself with rendering the game.
#[derive(Clone, Debug)]
pub struct UiState {
    pages: Vec<Page>,
    create_world_name: String,
    create_world_seed: String,
    join_world_address: String,

    s2_open: bool,
    s3_open: bool,
}
impl Default for UiState {
    fn default() -> Self {
        Self {
            pages: vec![],
            create_world_name: String::from("New World"),
            create_world_seed: String::new(),
            join_world_address: String::from("127.0.0.1:60000"),

            s2_open: false,
            s3_open: false,
        }
    }
}
impl UiState {
    pub fn page_is_open(&self) -> bool {
        self.pages.len() > 0
    }
    pub fn open_page(&mut self, page: Page) {
        self.pages.push(page);
    }
    pub fn close_page(&mut self) {
        _ = self.pages.pop();
    }

    pub fn clear_pages(&mut self) {
        self.pages.clear();
    }
}
#[derive(Clone, Copy, Debug)]
pub enum Page {
    PauseMenu,
    Options,
    MyWorlds,
    CreateWorld,
    JoinWorld,
    Visuals,
    Controls,
}

pub fn show_title_screen(ui: &mut Ui, state: &mut UiState) -> UiResponse {
    let rs = UiResponse::default();

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
    rs
}

pub fn show_game_overlay(
    ui: &mut Ui,
    state: &mut UiState,
    game: &mut GameState,
    timers: &Timers,
) {
    ui.visuals_mut().override_text_color = Some(egui::Color32::WHITE);

    ui.allocate_at_least(egui::vec2(200.0, 10.0), egui::Sense::empty());
    ui.painter().rect_filled(
        ui.max_rect(),
        egui::CornerRadius::same(5),
        egui::Color32::from_rgba_unmultiplied(0, 0, 0, 240),
    );
    ui.add_space(40.0);
    ui.heading(format!("fps: {}", timers.fps));
    ui.separator();

    fn column(ui: &mut Ui) {
        ui.painter().rect_filled(
            Rect::from_min_size(
                egui::pos2(ui.max_rect().min.x, ui.next_widget_position().y),
                egui::vec2(ui.available_width(), 24.0),
            ),
            egui::CornerRadius::same(5),
            egui::Color32::from_rgba_unmultiplied(30, 30, 30, 150),
        );
    }
    fn section(ui: &mut Ui, open: &mut bool, name: &str, f: impl FnOnce(&mut Ui)) {
        let label = format!(" {}  {name}", if *open { "-" } else { "+" });
        *open ^= ui.heading(label).clicked();
        if *open {
            f(ui);
        }
    }

    section(ui, &mut state.s2_open, "Player", |ui| {
        column(ui);
        ui.heading(format!(
            "pos: {:.1} {:.1} {:.1}",
            game.player.pos.x, game.player.pos.y, game.player.pos.z
        ));
        ui.heading(format!("on ground: {}", game.player.on_ground));
        column(ui);
        ui.heading(format!("jumped: {}", game.player.jumped));
        ui.heading(format!("flying (Z): {}", game.player.flying));
        ui.separator();
    });
    section(ui, &mut state.s3_open, "World", |ui| {
        let (free, capacity) = game.world.chunk_alloc_status();
        let used = ((capacity - free) as f32 / capacity as f32) * 100.0;
        column(ui);
        ui.heading(format!("width: {} chunks", game.world.size_in_chunks()));
        ui.heading(format!(
            "chunks: {}/{}",
            game.world.populated_count(),
            game.world.chunk_count(),
        ));
        column(ui);
        ui.heading(&format!("memory: %{used:.0}"));
        ui.separator();
    });
}

pub fn show_ui_state(ui: &mut Ui, state: &mut UiState, resources: &Resources, crosshair: &mut Crosshair) -> Option<UiResponse> {
    Some(match state.pages.last()? {
        Page::PauseMenu => show_pause_menu(ui, state),
        Page::Options => show_options(ui, state, resources),
        Page::MyWorlds => show_my_worlds(ui, state, resources),
        Page::CreateWorld => show_create_world(ui, state, resources),
        Page::JoinWorld => show_join_world(ui, state),
        Page::Visuals => show_visuals(ui, state, crosshair),
        Page::Controls => show_controls(ui, state),
    })
}

fn show_pause_menu(ui: &mut Ui, state: &mut UiState) -> UiResponse {
    let mut rs = UiResponse::default();
    if ui.button("Resume").clicked() {
        state.close_page();
    }
    if ui.button("Options").clicked() {
        state.open_page(Page::Options);
    }
    if ui.button("Save & Quit").clicked() {
        rs.quit_game = true;
    }
    rs
}

fn show_options(ui: &mut Ui, state: &mut UiState, _resources: &Resources) -> UiResponse {
    ui.skip_ahead_auto_ids(5);

    ui.add_space(30.0);
    ui.heading("Options");
    ui.add_space(30.0);

    if ui.button("Visuals").clicked() {
        state.open_page(Page::Visuals);
    }
    if ui.button("Controls").clicked() {
        state.open_page(Page::Controls);
    }
    ui.add_space(30.0);
    if ui.button("Back").clicked() {
        state.close_page();
    }
    UiResponse::default()
}

fn show_visuals(ui: &mut Ui, state: &mut UiState, crosshair: &mut Crosshair) -> UiResponse {
    let rs = UiResponse::default();

    ui.add_space(30.0);
    ui.heading("Visuals");
    ui.add_space(30.0);
    ui.heading("Crosshair");
    ui.heading("style: ");
    ui.horizontal(|ui| {
        ui.selectable_value(&mut crosshair.style, 0, "off");
        ui.selectable_value(&mut crosshair.style, 1, "dot");
        ui.selectable_value(&mut crosshair.style, 2, "cross");
    });
    ui.label("size");

    ui.add(egui::Slider::new(&mut crosshair.size, 1.0..=30.0));
    ui.color_edit_button_rgba_unmultiplied(&mut crosshair.color);

    if ui.button("reset").clicked() {
        *crosshair = Crosshair::default();
    }

    ui.add_space(30.0);
    if ui.button("Back").clicked() {
        state.close_page();
    }
    rs
}

fn show_controls(ui: &mut Ui, state: &mut UiState) -> UiResponse {
    let rs = UiResponse::default();
    ui.add_space(30.0);
    ui.heading("Controls");
    ui.add_space(30.0);
    ui.label("Movement - A-W-S-D + mouse");
    ui.label("Sprint - Shift");
    ui.label("Toggle chat - T");
    ui.label("Toggle flying - Z");
    ui.label("Pause - Esc");
    ui.label("Toggle fullscreen - F11");
    ui.label("Toggle debug overlay - F1");
    ui.label("Toggle debug rendering - F2");
    ui.label("Toggle world loading - F9");
    ui.add_space(30.0);

    if ui.button("Back").clicked() {
        state.close_page();
    }
    rs
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
    let mut rs = UiResponse::default();

    ui.text_edit_singleline(&mut state.join_world_address);

    let addr = SocketAddr::from_str(&state.join_world_address);
    if addr.is_err() {
        ui.visuals_mut().override_text_color = Some(egui::Color32::RED);
        ui.label("Invalid address - should be in the format: 127.0.0.1:60000");
    }

    if ui.button("Join").clicked() {
        if let Ok(addr) = addr {
            rs.join_game = Some(addr);
        }
    }
    ui.separator();
    if ui.button("Back").clicked() {
        state.close_page()
    }
    rs
}
