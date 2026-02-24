use egui::*;

pub fn draw_side_panel_contents(egui_ctx: &Context) -> Rect {
    egui::SidePanel::left("left_panel")
        .show(egui_ctx, |ui| {
            ui.heading("Solarance:Beginnings");
        })
        .response
        .rect
}
