use bevy::prelude::World;

use crate::fgr::{BoxedAccessor, ConstAccessor};
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

pub struct TextBoxElement {}

// TODO