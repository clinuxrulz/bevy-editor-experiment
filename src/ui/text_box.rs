use bevy::{asset::AssetServer, color::{palettes::css::BLUE, Color}, ecs::event::ManualEventReader, input::{keyboard::{Key, KeyboardInput}, ButtonInput}, prelude::{default, BuildWorldChildren, DespawnRecursiveExt, Entity, EventReader, Events, KeyCode, NodeBundle, TextBundle, World}, text::{Text, TextStyle}, ui::{Overflow, Style, Val}, utils::tracing::Event, window::ReceivedCharacter};
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
    event_reader: ManualEventReader<KeyboardInput>,
}

impl UiComponent<TextBoxProps> for TextBox {
    fn run(world: &mut World, props: TextBoxProps) -> Entity {
        let state = Arc::new(RwLock::new(TextBoxState {
            event_reader: ManualEventReader::default(),
        }));
        let init_cursor_pos = world.fgr_untrack(|world| props.contents.value(world).len());
        let cursor_pos = Signal::new(world, init_cursor_pos);
        let init_contents = world.fgr_untrack(|world| props.contents.value(world).clone());
        let contents = Signal::new(world, init_contents);
        let props_contents = props.contents;
        Memo::new(world, cloned!((props_contents, contents) => move |world| {
            let props_contents = props_contents.value(world).clone();
            contents.update_value(world, |x| *x = props_contents);
        }));
        let contents_length = Memo::new(world, cloned!((contents) => move |world| contents.value(world).len()));
        let cursor_pos_clamped = Memo::new(world, cloned!((cursor_pos, contents_length) => move |world| {
            let cursor_pos = *cursor_pos.value(world);
            let contents_length = *contents_length.value(world);
            return cursor_pos.clamp(0, contents_length);
        }));
        let contents_before_after_cursor = Memo::new(world, cloned!((cursor_pos_clamped, contents) => move |world| {
            let cursor_pos = *cursor_pos_clamped.value(world);
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
        world.fgr_on_update(cloned!((cursor_pos, cursor_pos_clamped, contents_length, contents) => move |world| {
            let cursor_pos_2 = *cursor_pos_clamped.value(world);
            let contents_length = *contents_length.value(world);
            let mut new_cursor_pos = cursor_pos_2;
            let mut state = state.write().unwrap();
            let keyboard_input_events = world.get_resource::<Events<KeyboardInput>>().unwrap();
            for event in state.event_reader.read(&keyboard_input_events) {
                if !event.state.is_pressed() {
                    continue;
                }
                match &event.logical_key {
                    Key::ArrowLeft => {
                        if new_cursor_pos > 0 {
                            new_cursor_pos -= 1;
                            cursor_pos.update_value(world, |x| *x = new_cursor_pos);
                        }
                        break;
                    }
                    Key::ArrowRight => {
                        if new_cursor_pos < contents_length {
                            new_cursor_pos += 1;
                            cursor_pos.update_value(world, |x| *x = new_cursor_pos);
                        }
                        break;
                    }
                    Key::Character(c) => {
                        if c.len() == 1 {
                            let c = c.chars().nth(0).unwrap();
                            contents.update_value(world, |x| {
                                *x = x[0..new_cursor_pos].to_string() + &c.to_string() + &x[new_cursor_pos..];
                            });
                            new_cursor_pos += 1;
                            cursor_pos.update_value(world, |x| *x = new_cursor_pos);
                            println!("char: {}", c);
                        }
                        break;
                    }
                    Key::Backspace => {
                        if new_cursor_pos > 0 {
                            contents.update_value(world, |x| {
                                *x = x[0..new_cursor_pos-1].to_string() + &x[new_cursor_pos..];
                            });
                            new_cursor_pos -= 1;
                            cursor_pos.update_value(world, |x| *x = new_cursor_pos);
                            println!("backspace");
                        }
                        break;
                    }
                    _ => {}
                }
            }
        }));
        return textbox_id;
    }
}
