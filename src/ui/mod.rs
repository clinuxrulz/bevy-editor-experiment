mod checkbox;
mod ui_component;

use std::sync::Arc;
use std::sync::Mutex;

use bevy::app::App;
use bevy::app::Startup;
use bevy::app::Update;
use bevy::prelude::World;
pub use checkbox::Checkbox;
pub use checkbox::CheckboxProps;
pub use ui_component::UiComponent;
pub use ui_component::UiComponentMount;

use crate::cloned;
use crate::fgr::FgrCtx;
use crate::fgr::RootScope;
use crate::fgr::FgrExtensionMethods;

pub fn render<'a, R: UiComponentMount + Sync + Send + 'static, CALLBACK: FnOnce(&mut World) -> R>(app: &mut App, callback: CALLBACK) -> RootScope<World> {
    let mount;
    let root_scope;
    app.insert_resource(FgrCtx::<World>::new());
    {
        let world = app.world_mut();
        let ctx = world;
        (mount, root_scope) = ctx.fgr_create_root(|ctx, root_scope| {
            let mount = callback(ctx);
            (mount, root_scope)
        });
    }
    let mount = Arc::new(Mutex::new(mount));
    app
        .add_systems(
            Startup,
            cloned!((mount) => move |world: &mut World| {
                let mut mount2 = mount.lock().unwrap();
                mount2.init(world);
            })
        )
        .add_systems(
            Update,
            cloned!((mount) => move |world: &mut World| {
                let mut mount2 = mount.lock().unwrap();
                mount2.update(world);
            })
        );
    root_scope
}
