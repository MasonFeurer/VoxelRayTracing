use std::mem::MaybeUninit;
use crate::{GameState, Timers};
use egui::{Align, Align2, Color32, Context, FontFamily, FontId, Id, Layout, Rect, Response, RichText, Sense, Stroke, StrokeKind, Ui, UiBuilder, Vec2, Widget};
use std::net::SocketAddr;
use std::str::FromStr;
use client::common::log::warn;
use client::common::resources::{Resources, WorldInfo, CURRENT_VERSION};
use client::common::world::Voxel;
use crate::graphics::Crosshair;

fn screen_size(ctx: &Context) -> Vec2 {
    ctx.input(|i| i.content_rect()).size()
}

pub type WorldOptions = WorldInfo;

#[derive(Default)]
pub struct UiResponse {
    pub host_world: Option<String>,
    pub host_new_world: Option<WorldOptions>,
    pub join_game: Option<SocketAddr>,
    pub quit_game: bool,
    pub reload_worlds: bool,
    pub exit_app: bool,
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
    selected_world: Option<String>,
    set_world_size: u32,

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
            selected_world: None,
            set_world_size: 0,

            s2_open: true,
            s3_open: true,
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
    let mut rs = UiResponse::default();

    title(ui, "Block World");
    ui.add_space(ROW_H);

    if show_buttons(ui, ["My Worlds"])[0].clicked() {
        state.open_page(Page::MyWorlds);
        rs.reload_worlds = true;
    }
    if show_buttons(ui, ["Join World"])[0].clicked() {
        state.open_page(Page::JoinWorld);
    }
    if show_buttons(ui, ["Options"])[0].clicked() {
        state.open_page(Page::Options);
    }
    if show_buttons(ui, ["Exit"])[0].clicked() {
        rs.exit_app = true;
    }
    rs
}

pub fn show_game_overlay(
    ui: &mut Ui,
    current_voxel: Voxel,
    state: &mut UiState,
    game: &mut GameState,
    timers: &Timers,
) {
    ui.visuals_mut().override_text_color = Some(Color32::WHITE);

    ui.allocate_at_least(egui::vec2(200.0, 10.0), Sense::empty());
    ui.painter().rect_filled(
        ui.max_rect(),
        egui::CornerRadius::same(5),
        Color32::from_rgba_unmultiplied(0, 0, 0, 240),
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
            Color32::from_rgba_unmultiplied(30, 30, 30, 150),
        );
    }
    fn section(ui: &mut Ui, open: &mut bool, name: &str, f: impl FnOnce(&mut Ui)) {
        let label = format!(" {}  {name}", if *open { "-" } else { "+" });
        *open ^= ui.heading(label).clicked();
        if *open {
            f(ui);
        }
    }
    
    let current_voxel = game.voxels.get(current_voxel).map(|data| data.name.as_str()).unwrap_or("Unknown");
    ui.heading(format!("Place: {current_voxel:?}"));
    
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

        let world_size = game.world.size_in_chunks();

        ui.heading(format!("world size: {world_size} chunks"));
        egui::Slider::new(&mut state.set_world_size, 10..=80).ui(ui);
        if ui.button("apply").clicked() && state.set_world_size != world_size {
            game.world.resize(state.set_world_size);
        }

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

const ROW_W: f32 = 500.0;
const ROW_H: f32 = 50.0;
const BUTTON_PAD: f32 = 10.0;

fn title(ui: &mut Ui, text: &str) {
    let y = ui.next_widget_position().y;
    let x = screen_size(ui.ctx()).x * 0.5;

    ui.painter().text(
        egui::pos2(x, y),
        Align2::CENTER_TOP,
        text,
        FontId::new(ROW_H, FontFamily::Proportional),
        Color32::WHITE,
    );
    ui.add_space(ROW_H);
}
fn heading(ui: &mut Ui, text: &str) {
    let y = ui.next_widget_position().y;
    let x = screen_size(ui.ctx()).x * 0.5;

    ui.painter().text(
        egui::pos2(x, y),
        Align2::CENTER_TOP,
        text,
        FontId::new(ROW_H * 0.6, FontFamily::Proportional),
        Color32::WHITE,
    );
    ui.add_space(ROW_H);
}
fn comment(ui: &mut Ui, text: &str, color: Color32) {
    let y = ui.next_widget_position().y;
    let x = screen_size(ui.ctx()).x * 0.5;

    ui.painter().text(
        egui::pos2(x, y),
        Align2::CENTER_TOP,
        text,
        FontId::new(ROW_H * 0.34, FontFamily::Proportional),
        color,
    );
    ui.add_space(ROW_H);
}

fn show_buttons<const N: usize>(ui: &mut Ui, titles: [&str; N]) -> [Response; N] {
    show_buttons_enabled(ui, titles, [true; N])
}

fn show_buttons_enabled<const N: usize>(ui: &mut Ui, titles: [&str; N], enabled: [bool; N]) -> [Response; N] {
    let mut responses: [Response; N] = unsafe { MaybeUninit::zeroed().assume_init() };

    let y = ui.next_widget_position().y;
    let mut x = screen_size(ui.ctx()).x * 0.5 - ROW_W * 0.5 + BUTTON_PAD * 0.5;
    let button_w = (ROW_W - BUTTON_PAD) / N as f32 - BUTTON_PAD;

    for i in 0..N {
        let pos = egui::pos2(x, y);
        let size = egui::vec2(button_w, ROW_H);
        let font = FontId::new(ROW_H * 0.8, FontFamily::Proportional);
        let button = egui::Button::new(RichText::new(titles[i]).font(font));
        let mut rs = ui.add_enabled_ui(enabled[i], |ui| {
            ui.place(Rect::from_min_size(pos, size), button)
        }).inner;

        std::mem::swap(&mut rs, &mut responses[i]);
        std::mem::forget(rs);

        x += button_w + BUTTON_PAD;
    }
    ui.add_space(ROW_H + BUTTON_PAD);

    responses
}

fn group<T>(ui: &mut Ui, id: impl Into<Id>, h: f32, align: Align, selected: bool, f: impl FnMut(&mut Ui) -> T) -> egui::InnerResponse<T> {
    let min = egui::pos2(screen_size(ui.ctx()).x * 0.5 - ROW_W * 0.5, ui.next_widget_position().y);
    let size = egui::vec2(ROW_W, ROW_H * h);

    let stroke = match selected {
        true => Stroke::new(5.0, Color32::WHITE),
        false => Stroke::new(3.0, Color32::GRAY),
    };

    ui.painter().rect_stroke(Rect::from_min_size(min, size).expand(4.0), 3.0, stroke, StrokeKind::Outside);
    let mut new_ui = ui.new_child(UiBuilder::new().id(id.into()).max_rect(Rect::from_min_size(min, size)));
    new_ui.set_min_size(size);
    let mut val = new_ui.with_layout(Layout::top_down(align), f);

    ui.add_space(ROW_H * h + BUTTON_PAD + 4.0);
    val.response.rect = Rect::from_min_size(min, size);
    val.response.interact_rect = val.response.rect;
    val
}

pub fn show_ui_state(ctx: &Context, state: &mut UiState, resources: &Resources, crosshair: &mut Crosshair, join_game_err: Option<&str>) -> Option<UiResponse> {
    egui::Area::new("some".into()).movable(false).show(ctx, |ui| {
        let rect = Rect::from_center_size(
            (screen_size(ui.ctx()) * 0.5).to_pos2(),
            egui::vec2(ROW_W + BUTTON_PAD * 2.0, screen_size(ui.ctx()).y * 0.9)
        );
        ui.painter().rect_filled(rect, 5.0, Color32::from_rgba_unmultiplied(0, 0, 0, 100));
        ui.allocate_at_least(egui::vec2(ROW_W, 10.0), Sense::empty());

        ui.with_layout(Layout::top_down(Align::Center), |ui| {
            Some(match state.pages.last()? {
                Page::PauseMenu => show_pause_menu(ui, state),
                Page::Options => show_options(ui, state, resources),
                Page::MyWorlds => show_my_worlds(ui, state, resources),
                Page::CreateWorld => show_create_world(ui, state, resources),
                Page::JoinWorld => show_join_world(ui, state, join_game_err),
                Page::Visuals => show_visuals(ui, state, crosshair),
                Page::Controls => show_controls(ui, state),
            })
        }).inner
    }).inner
}

fn show_pause_menu(ui: &mut Ui, state: &mut UiState) -> UiResponse {
    let mut rs = UiResponse::default();

    ui.add_space(ROW_H * 2.0);
    if show_buttons(ui, ["Resume"])[0].clicked() {
        state.close_page();
    }
    if show_buttons(ui, ["Options"])[0].clicked() {
        state.open_page(Page::Options);
    }
    if show_buttons(ui, ["Save & Quit"])[0].clicked() {
        rs.quit_game = true;
    }
    rs
}

fn show_options(ui: &mut Ui, state: &mut UiState, _resources: &Resources) -> UiResponse {
    ui.skip_ahead_auto_ids(5);

    ui.add_space(30.0);
    title(ui, "Options");
    ui.add_space(30.0);

    let [visuals, controls] = show_buttons(ui, ["Visuals", "Controls"]);
    if visuals.clicked() {
        state.open_page(Page::Visuals);
    }
    if controls.clicked() {
        state.open_page(Page::Controls);
    }
    if show_buttons(ui, ["Back"])[0].clicked() {
        state.close_page();
    }
    UiResponse::default()
}

fn show_visuals(ui: &mut Ui, state: &mut UiState, crosshair: &mut Crosshair) -> UiResponse {
    let rs = UiResponse::default();

    ui.add_space(30.0);
    title(ui, "Visuals");
    ui.add_space(30.0);
    heading(ui, "Crosshair");

    group(ui, "visuals", 5.0, Align::Min, false, |ui| {
        ui.heading("style: ");

        ui.horizontal(|ui| {
            ui.selectable_value(&mut crosshair.style, 0, "off");
            ui.selectable_value(&mut crosshair.style, 1, "dot");
            ui.selectable_value(&mut crosshair.style, 2, "cross");
        });
        ui.label("size");

        ui.add(egui::Slider::new(&mut crosshair.size, 1.0..=30.0));
        ui.color_edit_button_rgba_unmultiplied(&mut crosshair.color);
    });

    let [back, reset] = show_buttons(ui, ["Back", "Reset"]);
    if reset.clicked() {
        *crosshair = Crosshair::default();
    }
    if back.clicked() {
        state.close_page();
    }
    rs
}

fn show_controls(ui: &mut Ui, state: &mut UiState) -> UiResponse {
    let rs = UiResponse::default();
    ui.add_space(30.0);
    title(ui, "Controls");

    ui.add_space(30.0);
    ui.heading("Movement - A-W-S-D + mouse");
    ui.heading("Sprint - Shift");
    ui.heading("Toggle chat - T");
    ui.heading("Toggle flying - Z");
    ui.heading("Pause - Esc");
    ui.heading("Toggle fullscreen - F11");
    ui.heading("Toggle debug overlay - F1");
    ui.heading("Toggle debug rendering - F2");
    ui.heading("Toggle world loading - F9");
    ui.add_space(30.0);

    if show_buttons(ui, ["Back"])[0].clicked() {
        state.close_page();
    }
    rs
}

fn show_my_worlds(ui: &mut Ui, state: &mut UiState, resources: &Resources) -> UiResponse {
    let mut rs = UiResponse::default();
    for world in &resources.worlds {
        let selected = state.selected_world == Some(world.name.clone());
        let id = Id::from(format!("world-{}", world.name));
        let rs = group(ui, id, 1.0, Align::Center, selected, |ui| {
            let label = RichText::new(world.name.clone())
                .font(FontId::new(ROW_H * 0.6, FontFamily::Proportional));
            ui.add(egui::Label::new(label).selectable(false))
        }).response;

        if rs.interact(Sense::click_and_drag()).clicked() {
            state.selected_world = Some(world.name.clone());
        }
    }
    if let Some(world) = &state.selected_world {
        let [play, delete, edit] = show_buttons(ui, ["Play", "Delete", "Edit"]);
        if play.clicked() {
            rs.host_world = Some(world.clone());
        }
        if delete.clicked() {
            warn!("Not implemented");
        }
        if edit.clicked() {
            warn!("Not implemented");
        }
    }
    let [back, create] = show_buttons(ui, ["Back", "Create"]);
    if create.clicked() {
        state.open_page(Page::CreateWorld);
    }
    if back.clicked() {
        state.close_page();
    }
    rs
}

fn show_create_world(ui: &mut Ui, state: &mut UiState, resources: &Resources) -> UiResponse {
    let mut rs = UiResponse::default();

    let path = resources.path.join("worlds").join(&state.create_world_name);
    let is_valid = !path.exists();

    let font = FontId::new(ROW_H * 0.8, FontFamily::Proportional);
    let edit = egui::TextEdit::singleline(&mut state.create_world_name).font(font).min_size(egui::vec2(ROW_W, ROW_H));
    let rect = Rect::from_min_size(
        egui::pos2(screen_size(ui.ctx()).x * 0.5 - ROW_W  * 0.5, ui.next_widget_position().y),
        egui::vec2(ROW_W, ROW_H)
    );
    ui.place(rect, edit);
    ui.add_space(ROW_H * 1.5);

    comment(ui, &format!("Stored at {}", path.display()), Color32::WHITE);
    if !is_valid {
        comment(ui, "ERROR: directory already exists", Color32::RED);
    }

    let font = FontId::new(ROW_H * 0.5, FontFamily::Proportional);
    let edit = egui::TextEdit::singleline(&mut state.create_world_seed)
        .font(font)
        .text_color(Color32::GRAY)
        .min_size(egui::vec2(ROW_W, ROW_H))
        .hint_text("leave blank for random seed");
    let rect = Rect::from_min_size(
        egui::pos2(screen_size(ui.ctx()).x * 0.5 - ROW_W  * 0.5, ui.next_widget_position().y),
        egui::vec2(ROW_W, ROW_H)
    );
    ui.place(rect, edit);
    ui.add_space(ROW_H * 2.0);

    let [back, create] =  show_buttons_enabled(ui, ["Back", "Create"], [true, is_valid]);
    if create.clicked() {
        state.pages.clear();
        rs.host_new_world = Some(WorldInfo {
            name: state.create_world_name.clone(),
            version: CURRENT_VERSION,
            datapack: String::from("blockworld.vanilla"),
            stylepack: String::from("blockworld.vanilla"),
        });
    }
    if back.clicked() {
        state.close_page();
    }
    rs
}

fn show_join_world(ui: &mut Ui, state: &mut UiState, join_game_err: Option<&str>) -> UiResponse {
    let mut rs = UiResponse::default();

    let font = FontId::new(ROW_H * 0.8, FontFamily::Proportional);
    let edit = egui::TextEdit::singleline(&mut state.join_world_address).font(font).min_size(egui::vec2(ROW_W, ROW_H));
    let rect = Rect::from_min_size(
        egui::pos2(screen_size(ui.ctx()).x * 0.5 - ROW_W  * 0.5, ui.next_widget_position().y),
        egui::vec2(ROW_W, ROW_H)
    );
    ui.place(rect, edit);
    ui.add_space(ROW_H * 1.5);

    let addr = SocketAddr::from_str(&state.join_world_address);
    if addr.is_err() {
        comment(ui, "Invalid address - should be in the format: 127.0.0.1:60000", Color32::RED);
    }
    if let Some(err) = join_game_err {
        let mut lines = err.lines();
        comment(ui, &format!("Failed to connect : {}", lines.next().unwrap()), Color32::RED);
        for line in lines {
            if !line.is_empty() {
                comment(ui, &line, Color32::RED);
            }
        }
    }

    let [back, join] = show_buttons_enabled(ui, ["Back", "Join"], [true, addr.is_ok()]);

    if join.clicked() {
        if let Ok(addr) = addr {
            rs.join_game = Some(addr);
        }
    }
    if back.clicked() {
        state.close_page()
    }
    rs
}
