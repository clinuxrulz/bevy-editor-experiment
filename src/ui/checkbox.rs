use bevy::{color::{palettes::css::RED, Color}, prelude::{ButtonBundle, Entity, World}, ui::{BackgroundColor, BorderRadius, Interaction, Style, Val}};

use crate::fgr::{FgrCtx, Memo};

use super::{ui_component::UiComponentMount, UiComponent};

pub struct Checkbox;

pub struct CheckboxProps {
    pub on_changed: Option<Box<dyn FnMut(&mut FgrCtx, bool)>>,
}

impl Default for CheckboxProps {
    fn default() -> Self {
        Self {
            on_changed: Default::default(),
        }
    }
}

impl UiComponent<CheckboxProps> for Checkbox {
    fn execute(props: CheckboxProps) -> impl UiComponentMount {
        struct CheckboxMount {
            checkbox_entity: Option<Entity>,
            last_interaction: Interaction,
            checked: bool,
        }
        impl UiComponentMount for CheckboxMount {
            fn init(&mut self, world: &mut World) {
                self.checkbox_entity = Some(
                    world
                        .spawn(
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
            fn update(&mut self, world: &mut World) {
                let Some(entity) = self.checkbox_entity else { return; };
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
                }
            }
            fn dispose(&mut self, world: &mut World) {
                let Some(entity) = self.checkbox_entity else { return; };
                world.despawn(entity);
            }
        }
        CheckboxMount {
            checkbox_entity: None,
            last_interaction: Interaction::None,
            checked: false,
        }
    }
}
