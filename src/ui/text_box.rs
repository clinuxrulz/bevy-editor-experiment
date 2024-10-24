use bevy::{asset::AssetServer, color::{palettes::css::BLUE, Color}, input::{keyboard::KeyboardInput, ButtonInput}, prelude::{default, BuildWorldChildren, DespawnRecursiveExt, Entity, EventReader, Events, KeyCode, NodeBundle, TextBundle, World}, text::{Text, TextStyle}, ui::{Overflow, Style, Val}};
use std::{borrow::{Borrow, BorrowMut}, str::FromStr, sync::Arc};
use std::sync::RwLock;

use crate::{cloned, fgr::{Accessor, BoxedAccessor, ConstAccessor, FgrExtensionMethods, Memo, Signal}};

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
        let cursor_pos = Signal::new(world, 0);
        let contents = props.contents;
        let contents_length = Memo::new(world, cloned!((contents) => move |world| contents.value(world).len()));
        let contents_before_after_cursor = Memo::new(world, cloned!((cursor_pos, contents) => move |world| {
            let cursor_pos = *cursor_pos.value(world);
            let contents = &*contents.value(world);
            let before = Arc::new(String::from_str(&contents[0..cursor_pos]).unwrap());
            let after = Arc::new(String::from_str(&contents[cursor_pos..]).unwrap());
            return (before, after);
        }));
        let contents_before = Memo::new(world, cloned!((contents_before_after_cursor) => move |world| {
            Arc::clone(&contents_before_after_cursor.value(world).0)
        }));
        let contents_after = Memo::new(world, cloned!((contents_before_after_cursor) => move |world| {
            Arc::clone(&contents_before_after_cursor.value(world).1)
        }));
        let font;
        {
            let asset_server = world.get_resource::<AssetServer>().unwrap();
            font = asset_server.load("fonts/FiraSans-Bold.ttf");
        }
        let contents_before_id = world.spawn(
            TextBundle::from_section(
                "",
                TextStyle {
                    font: font.clone(),
                    font_size: 20.0,
                    color: Color::WHITE,
                },
            ),
        ).id();
        world.fgr_create_effect(cloned!((contents_before) => move |world| {
            let contents_before = (**contents_before.value(world)).clone();
            let mut entity = world.entity_mut(contents_before_id);
            let mut text = entity.get_mut::<Text>().unwrap();
            let section = &mut text.sections[0];
            section.value = contents_before;
        }));
        let contents_after_id = world.spawn(
            TextBundle::from_section(
                "",
                TextStyle {
                    font,
                    font_size: 20.0,
                    color: Color::WHITE,
                },
            ),
        ).id();
        world.fgr_create_effect(cloned!((contents_after) => move |world| {
            let contents_after = (**contents_after.value(world)).clone();
            let mut entity = world.entity_mut(contents_after_id);
            let mut text = entity.get_mut::<Text>().unwrap();
            let section = &mut text.sections[0];
            section.value = contents_after;
        }));
        let cursor_id = world
            .spawn(NodeBundle {
                style: Style {
                    width: Val::Px(2.0),
                    height: Val::Percent(100.0),
                    overflow: Overflow::visible(),
                    ..default()
                },
                ..default()
            })
            .with_children(|parent| {
                parent
                    .spawn(NodeBundle {
                        style: Style {
                            width: Val::Px(2.0),
                            height: Val::Percent(100.0),
                            ..default()
                        },
                        background_color: BLUE.into(),
                        ..default()
                    });
                })
            .id();
        let textbox_id = world
            .spawn(
                NodeBundle {
                    ..Default::default()
                }
            )
            .push_children(&[
                contents_before_id,
                cursor_id,
                contents_after_id,
            ])
            .id();
        world.fgr_on_cleanup(cloned!((textbox_id) => move |world| {
            world.entity_mut(textbox_id).despawn_recursive();
        }));
        world.fgr_on_update(cloned!((cursor_pos, contents_length) => move |world| {
            let cursor_pos_2 = *cursor_pos.value(world);
            let contents_length = *contents_length.value(world);
            //let keyboard_input = world.get_resource::<Events<KeyboardInput>>().unwrap();
            //let mut reader = keyboard_input.get_reader();
            let mut new_cursor_pos = cursor_pos_2;
            enum LeftOrRight {
                Left,
                Right
            }
            let mut move_cursor: Option<LeftOrRight> = None;
            {
                let keys = world.resource::<ButtonInput<KeyCode>>();
                if keys.just_pressed(KeyCode::ArrowLeft) {
                    move_cursor = Some(LeftOrRight::Left);
                }
                if keys.just_pressed(KeyCode::ArrowRight) {
                    move_cursor = Some(LeftOrRight::Right);
                }
            }
            match move_cursor {
                Some(LeftOrRight::Left) => {
                    if cursor_pos_2 > 0 {
                        new_cursor_pos = cursor_pos_2 - 1;
                    }
                }
                Some(LeftOrRight::Right) => {
                    if cursor_pos_2 < contents_length {
                        new_cursor_pos = cursor_pos_2 + 1;
                    }
                }
                None => {}
            }
            if new_cursor_pos != cursor_pos_2 {
                cursor_pos.update_value(world, |x| *x = new_cursor_pos);
            }
        }));
        return textbox_id;
    }
}
