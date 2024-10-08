use std::sync::{Arc, RwLock};

use bevy::prelude::World;

use crate::{cloned, fgr::{BoxedAccessor, ConstAccessor}};

use super::UiComponent;

pub struct TextBox;

pub struct TextBoxProps {
    contents: BoxedAccessor<World, String>,
}

impl Default for TextBoxProps {
    fn default() -> Self {
        Self {
            contents: ConstAccessor::new("".into()).into(),
        }
    }
}

impl UiComponent<TextBoxProps> for TextBox {
    fn execute(world: &mut World, props: TextBoxProps) -> Box<dyn FnMut(&mut World) + Send + Sync> {
        struct TextBoxState {
            is_selected: bool,
            cursor_position: usize,
        }
        /*
        let state: Arc<RwLock<TextBoxState>> = Arc::new(RwLock::new(
            TextBoxState {}
        ));*/
        let update: Box<dyn FnMut(&mut World) + Send + Sync> = Box::new(cloned!((/*state*/) => move |world| {

        }));
        update
    }
}