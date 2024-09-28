use bevy::{prelude::*, winit::WinitSettings};
use ui::UiComponent;

pub mod fgr;
pub mod ui;

#[cfg(test)]
mod tests;

fn main() {
    let mut app = App::new();
    app.add_plugins(DefaultPlugins)
        .insert_resource(WinitSettings::desktop_app())
        .add_systems(Startup, setup);
    ui::render(
        &mut app,
        || {
            ui::Checkbox::execute(())
        }
    );
    app.run();
}

fn setup(mut commands: Commands) {
    // ui camera
    commands.spawn(Camera2dBundle::default());
}
