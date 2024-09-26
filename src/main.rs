use bevy::{prelude::*, winit::WinitSettings};

pub mod fgr;

#[cfg(test)]
mod tests;

fn main() {
    let mut app = App::new();
    app.add_plugins(DefaultPlugins)
        .insert_resource(WinitSettings::desktop_app())
        .add_systems(Startup, setup);
    app.run();
}

fn setup(mut commands: Commands, _asset_server: Res<AssetServer>) {

}