use egui::*;
use spacetimedb_sdk::*;

use crate::module_bindings::*;

pub fn draw_side_panel_contents(egui_ctx: &Context, client: &DbConnection) -> Rect {
    egui::SidePanel::left("left_panel")
        .show(egui_ctx, |ui| {
            ui.heading("Solarance:Beginnings");
            ui.separator();

            ui.label(format!(
                "Ships: {}",
                client.db().current_sector_ships().count()
            ));

            if let Some(player_state) = client.db().my_player_state().iter().next() {
                ui.label(format!(
                    "Player State: ({} total seen)",
                    client.db().my_player_state().count()
                ));
                ui.code(format!("{:?}", player_state));
            } else {
                ui.label("Player State: Not connected");
            }
            ui.separator();
            ui.heading("Table View Report");

            ui.label(format!(
                "Current Sector Ships: {}",
                client.db().current_sector_ships().count()
            ));

            ui.label(format!(
                "Visible Sectors in Current System: {}",
                client.db().current_system_visible_sectors().count()
            ));

            ui.label(format!(
                "My Visited Systems: {}",
                client.db().my_visited_systems().count()
            ));
            ui.separator();

            ui.collapsing("Reducers", |ui| {
                if ui.button("Spawn Ship").clicked() {
                    let _ = client.reducers().spawn_ship();
                }
            });
        })
        .response
        .rect
}
