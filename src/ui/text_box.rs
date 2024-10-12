use bevy::{color::palettes::css::BLUE, prelude::{default, BuildWorldChildren, Entity, NodeBundle, World}, ui::{Style, Val}};
use std::sync::Arc;
use std::sync::RwLock;

use crate::fgr::{BoxedAccessor, ConstAccessor};

use super::UiComponent;

pub struct TextBoxProps {
    pub width: BoxedAccessor<World, f32>,
    pub height: BoxedAccessor<World, f32>,
    pub contents: BoxedAccessor<World, String>,
}

impl Default for TextBoxProps {
    fn default() -> Self {
        Self {
            width: ConstAccessor::new(100.0).into(),
            height: ConstAccessor::new(50.0).into(),
            contents: ConstAccessor::new("".into()).into(),
        }
    }
}

pub struct TextBox;

struct TextBoxState {
}

impl UiComponent<TextBoxProps> for TextBox {
    fn run(world: &mut World, props: TextBoxProps) -> Entity {
        let state = Arc::new(RwLock::new(TextBoxState {}));
        let textbox_id = world
            .spawn(
                NodeBundle {
                    ..Default::default()
                }
            )
            .with_children(|parent| {
                parent.spawn(NodeBundle {
                    style: Style {
                        left: Val::Px(100.0),
                        top: Val::Px(100.0),
                        width: Val::Px(5.0),
                        height: Val::Px(50.0),
                        ..default()
                    },
                    background_color: BLUE.into(),
                    ..default()
                });
            })
            .id();
        return textbox_id;
    }
}
