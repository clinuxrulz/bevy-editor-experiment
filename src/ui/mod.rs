mod check_box_element;
mod element;
mod text_box_element;
mod ui_component;

use std::sync::Arc;
use std::sync::Mutex;

use bevy::app::App;
use bevy::app::Startup;
use bevy::app::Update;
use bevy::prelude::World;
pub use check_box_element::CheckBoxElement;
pub use check_box_element::CheckBoxProps;
pub use element::{Element, Elements};
pub use text_box_element::TextBoxElement;
pub use text_box_element::TextBoxProps;
pub use ui_component::UiComponent;

use crate::cloned;
use crate::fgr::FgrCtx;
use crate::fgr::RootScope;
use crate::fgr::FgrExtensionMethods;

pub fn render<'a, R: Element + Send + Sync + 'static, CALLBACK: FnOnce(&mut World) -> R>(app: &mut App, callback: CALLBACK) -> RootScope<World> {
    let element;
    let root_scope;
    app.insert_resource(FgrCtx::<World>::new());
    {
        let world = app.world_mut();
        let ctx = world;
        (element, root_scope) = ctx.fgr_create_root(|ctx, root_scope| {
            let element = callback(ctx);
            let element: Arc<Mutex<dyn Element + Send + Sync>> = Arc::new(Mutex::new(element));
            ctx.fgr_on_cleanup(cloned!((element) => move |ctx| {
                element.lock().unwrap().unmount(ctx);
            }));
            (element, root_scope)
        });
    }
    app
        .add_systems(
            Startup,
            cloned!((element) => move |world: &mut World| {
                element.lock().unwrap().mount(world);
            })
        )
        .add_systems(
            Update,
            move |world: &mut World| {
                element.lock().unwrap().update(world);
            }
        );
    root_scope
}
