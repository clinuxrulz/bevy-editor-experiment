mod checkbox;
mod ui_component;

use std::sync::Arc;
use std::sync::Mutex;

use bevy::app::App;
use bevy::app::Update;
use bevy::prelude::World;
pub use checkbox::Checkbox;
pub use checkbox::CheckboxProps;
pub use ui_component::UiComponent;

use crate::fgr::FgrCtx;
use crate::fgr::RootScope;
use crate::fgr::FgrExtensionMethods;

pub fn render<'a, R: FnMut(&mut World) + Send + Sync + 'static, CALLBACK: FnOnce(&mut World) -> R>(app: &mut App, callback: CALLBACK) -> RootScope<World> {
    let update;
    let root_scope;
    app.insert_resource(FgrCtx::<World>::new());
    {
        let world = app.world_mut();
        let ctx = world;
        (update, root_scope) = ctx.fgr_create_root(|ctx, root_scope| {
            let update = callback(ctx);
            (update, root_scope)
        });
    }
    let update: Arc<Mutex<dyn FnMut(&mut World) + Send + Sync>> = Arc::new(Mutex::new(update));
    app
        .add_systems(
            Update,
            move |world: &mut World| {
                update.lock().unwrap()(world);
            }
        );
    root_scope
}
