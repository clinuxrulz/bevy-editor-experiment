use std::sync::{Arc, RwLock};

use bevy::{color::{palettes::css::RED, Color}, prelude::{ButtonBundle, Entity, World}, ui::{BackgroundColor, BorderRadius, Interaction, Style, Val}};

use crate::fgr::FgrExtensionMethods;

use super::UiComponent;

pub struct CheckBoxProps {
    pub on_changed: Option<Box<dyn FnMut(&mut World, bool) + Send + Sync>>,
}

impl Default for CheckBoxProps {
    fn default() -> Self {
        Self {
            on_changed: Default::default(),
        }
    }
}

struct CheckBoxState {
    pub props: CheckBoxProps,
    pub last_interaction: Interaction,
    pub checked: bool,
}

impl CheckBoxState {
    fn new(props: CheckBoxProps) -> Self {
        Self {
            props,
            last_interaction: Interaction::None,
            checked: false,
        }
    }
}

pub struct CheckBox;

impl UiComponent<CheckBoxProps> for CheckBox {
    fn run(world: &mut World, props: CheckBoxProps) -> Entity {
        let state = Arc::new(RwLock::new(CheckBoxState::new(props)));
        let checkbox_id = world.spawn(
                ButtonBundle {
                    style: Style {
                        width: Val::Px(10.0),
                        height: Val::Px(10.0),
                        ..Default::default()
                    },
                    border_color: Color::WHITE.into(),
                    border_radius: BorderRadius::all(Val::Px(5.0)),
                    background_color: Color::BLACK.into(),
                    ..Default::default()
                }
            )
            .id();
        world.fgr_on_cleanup(move |world| {
            world.despawn(checkbox_id);
        });
        world.fgr_on_update(move |world| {
            let mut state = state.write().unwrap();
            let entity = checkbox_id;
            let Some(interaction) = world.get::<Interaction>(entity) else { return; };
            if *interaction == state.last_interaction {
                return;
            }
            state.last_interaction = *interaction;
            if *interaction == Interaction::Pressed {
                state.checked = !state.checked;
                world
                    .get_mut::<BackgroundColor>(entity)
                    .unwrap()
                    .0 = if state.checked { RED.into() } else { Color::BLACK };
                let checked = state.checked;
                if let Some(on_changed) = &mut state.props.on_changed {
                    on_changed(world, checked);
                }
            }
        });
        checkbox_id
    }
}
