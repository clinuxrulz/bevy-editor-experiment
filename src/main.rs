use bevy::{prelude::*, winit::WinitSettings};
use fgr::{FgrCtx, Memo, Signal};
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
        |fgr_ctx| {
            let checked = Signal::new(fgr_ctx, false);
            fgr_ctx.create_effect(cloned!((checked) => move |fgr_ctx| {
                println!("checked = {}", *checked.value(fgr_ctx));
            }));
            ui::Checkbox::execute(
                ui::CheckboxProps {
                    on_changed: Some(Box::new(cloned!((checked) => move |fgr_ctx, value| {
                        checked.update_value(fgr_ctx, |old_value| *old_value = value);
                    }))),
                },
            )
        }
    );
    app.run();
    scope.dispose();
}

fn setup(mut commands: Commands) {
    // ui camera
    commands.spawn(Camera2dBundle::default());
}
