use bevy::{color::palettes::css::BLUE, prelude::{default, BuildWorldChildren, Entity, NodeBundle, World}, ui::{Style, Val}};

use crate::fgr::{BoxedAccessor, ConstAccessor};

use super::Element;
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

pub struct TextBoxElement {
    props: TextBoxProps,
    textbox_entity: Option<Entity>,
    cursor_entity: Option<Entity>,
}

impl TextBoxElement {
    pub fn new(props: TextBoxProps) -> Self {
        Self {
            props,
            textbox_entity: None,
            cursor_entity: None,
        }
    }
}

impl Element for TextBoxElement {
    fn mount(&mut self, world: &mut World) {
        let mut cursor_entity: Option<Entity> = None;
        let textbox_entity = world.spawn(NodeBundle { ..default() }).with_children(|parent| {
            cursor_entity = Some(parent.spawn(NodeBundle {
                style: Style {
                    left: Val::Px(100.0),
                    top: Val::Px(100.0),
                    width: Val::Px(5.0),
                    height: Val::Px(50.0),
                    ..default()
                },
                background_color: BLUE.into(),
                ..default()
            }).id());
        }).id();
        self.textbox_entity = Some(textbox_entity);
        self.cursor_entity = cursor_entity;
    }

    fn unmount(&mut self, world: &mut World) {
        if let Some(cursor_entity) = self.cursor_entity {
            world.despawn(cursor_entity);
        }
    }

    fn update(&mut self, world: &mut World) {
    }
}
