use bevy::{prelude::*, winit::WinitSettings};
use fgr::{print_graph, Signal, FgrExtensionMethods};
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
    let mut scope = ui::render(
        &mut app,
        |world| {
            let checked = Signal::new(world, false);
            world.fgr_create_effect(cloned!((checked) => move |world| {
                println!("checked = {}", *checked.value(world));
            }));
            print_graph((&checked).into());
            ui::Checkbox::execute(
                ui::CheckboxProps {
                    on_changed: Some(Box::new(cloned!((checked) => move |world, value| {
                        checked.update_value(world, |old_value| *old_value = value);
                        print_graph((&checked).into());
                    }))),
                },
            )
        }
    );
    app.run();
    scope.dispose(app.world_mut());
}

fn setup(mut commands: Commands) {
    // ui camera
    commands.spawn(Camera2dBundle::default());
}
