mod checkbox;
mod ui_component;

use std::sync::Arc;
use std::sync::Mutex;

use bevy::app::App;
use bevy::app::Startup;
use bevy::app::Update;
use bevy::prelude::World;
pub use checkbox::Checkbox;
pub use ui_component::UiComponent;
pub use ui_component::UiComponentMount;

use crate::cloned;
use crate::fgr::FgrCtx;

pub fn render<'a, R: UiComponentMount + Sync + Send + 'static, CALLBACK: FnOnce() -> R>(app: &mut App, callback: CALLBACK) {
    let mount = callback();
    let mount = Arc::new(Mutex::new(mount));
    app
        .insert_resource(FgrCtx::new())
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
}
