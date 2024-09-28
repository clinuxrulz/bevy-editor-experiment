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

pub fn render<'a, R: UiComponentMount + Sync + Send + 'static, CALLBACK: FnOnce(&mut FgrCtx) -> R>(app: &mut App, callback: CALLBACK) -> RootScope {
    let mut fgr_ctx = FgrCtx::new();
    let (mount, root_scope) = fgr_ctx.create_root(|fgr_ctx, root_scope| {
        let mount = callback(fgr_ctx);
        (mount, root_scope)
    });
    let mount = Arc::new(Mutex::new(mount));
    app
        .insert_resource(fgr_ctx)
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
