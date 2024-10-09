use bevy::{prelude::*, winit::WinitSettings};

#[cfg(test)]
mod tests;

use bevy_editor_experiment_lib::{
    cloned,
    fgr::{print_graph, FgrExtensionMethods, RootScope, Signal},
    ui::{self, Elements},
};

fn main() {
    let mut app = App::new();
    app.add_plugins(DefaultPlugins)
        .insert_resource(WinitSettings::desktop_app())
        .add_systems(Startup, setup);
    let scope = ui::render(
        &mut app,
        |world| {
            let checked = Signal::new(world, false);
            world.fgr_create_effect(cloned!((checked) => move |world| {
                println!("checked = {}", *checked.value(world));
            }));
            print_graph((&checked).into());
            Elements(vec![
                Box::new(ui::CheckBoxElement::new(
                    ui::CheckBoxProps {
                        on_changed: Some(Box::new(cloned!((checked) => move |world, value| {
                            checked.update_value(world, |old_value| *old_value = value);
                            print_graph((&checked).into());
                        }))),
                    },
                )),
                Box::new(ui::TextBoxElement::new(ui::TextBoxProps {
                    ..default()
                }))
            ])
        }
    );
    app.insert_resource(scope)
        .add_systems(PostUpdate, |world: &mut World| {
            let exit_event = world.get_resource_mut::<Events<AppExit>>().unwrap();
            if !exit_event.is_empty() {
                let scope = world.remove_resource::<RootScope<World>>();
                if let Some(mut scope) = scope {
                    scope.dispose(world);
                }
            }
        })
        .run();
}

fn setup(mut commands: Commands) {
    // ui camera
    commands.spawn(Camera2dBundle::default());
}
