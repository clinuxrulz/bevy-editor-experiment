use bevy::{color::{palettes::css::RED, Color}, prelude::{ButtonBundle, Entity, World}, ui::{BackgroundColor, BorderRadius, Interaction, Style, Val}};

use super::Element;

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

pub struct CheckBoxElement {
    pub props: CheckBoxProps,
    pub checkbox_entity: Option<Entity>,
    pub last_interaction: Interaction,
    pub checked: bool,
}

impl CheckBoxElement {
    pub fn new(props: CheckBoxProps) -> Self {
        Self {
            props,
            checkbox_entity: None,
            last_interaction: Interaction::None,
            checked: false,
        }
    }
}

impl Element for CheckBoxElement {
    fn mount(&mut self, world: &mut World) {
        self.checkbox_entity = Some(
            world.spawn(
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
            .id()
        );
    }

    fn unmount(&mut self, world: &mut World) {
        if let Some(entity) = self.checkbox_entity {
            world.despawn(entity);
        }
        self.checkbox_entity = None;
    }

    fn update(&mut self, world: &mut World) {
        let entity = self.checkbox_entity;
        let Some(entity) = entity else { return; };
        let Some(interaction) = world.get::<Interaction>(entity) else { return; };
        if *interaction == self.last_interaction {
            return;
        }
        self.last_interaction = *interaction;
        if *interaction == Interaction::Pressed {
            self.checked = !self.checked;
            world
                .get_mut::<BackgroundColor>(entity)
                .unwrap()
                .0 = if self.checked { RED.into() } else { Color::BLACK };
            if let Some(on_changed) = &mut self.props.on_changed {
                on_changed(world, self.checked);
            }
        }
    }
}
