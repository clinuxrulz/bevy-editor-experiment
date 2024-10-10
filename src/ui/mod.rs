mod check_box;
//mod text_box;
mod ui_component;

use bevy::app::App;
use bevy::app::Update;
use bevy::prelude::Entity;
use bevy::prelude::World;
pub use check_box::CheckBox;
pub use check_box::CheckBoxProps;
//pub use text_box::TextBoxElement;
//pub use text_box::TextBoxProps;
pub use ui_component::UiComponent;

use crate::fgr::FgrCtx;
use crate::fgr::RootScope;
use crate::fgr::FgrExtensionMethods;

pub fn render<'a, CALLBACK: FnOnce(&mut World) -> Entity>(app: &mut App, callback: CALLBACK) -> RootScope<World> {
    let element;
    let root_scope;
    app.insert_resource(FgrCtx::<World>::new());
    {
        let world = app.world_mut();
        let ctx = world;
        (element, root_scope) = ctx.fgr_create_root(|ctx, root_scope| {
            let element = callback(ctx);
            (element, root_scope)
        });
    }
    app
        .add_systems(
            Update,
            move |world: &mut World| {
                world.fgr_update();
            }
        );
    root_scope
}
