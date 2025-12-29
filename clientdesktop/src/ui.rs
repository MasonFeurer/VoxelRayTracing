use crate::{Crosshair, GameState, Timers};
use egui::Ui;

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
