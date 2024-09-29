use std::sync::{Arc, RwLock};

use bevy::{color::{palettes::css::RED, Color}, prelude::{ButtonBundle, Entity, World}, ui::{BackgroundColor, BorderRadius, Interaction, Style, Val}};

use crate::{cloned, fgr::FgrExtensionMethods};

use super::UiComponent;

pub struct Checkbox;

pub struct CheckboxProps {
    pub on_changed: Option<Box<dyn FnMut(&mut World, bool) + Send + Sync>>,
}

impl Default for CheckboxProps {
    fn default() -> Self {
        Self {
            on_changed: Default::default(),
        }
    }
}

impl UiComponent<CheckboxProps> for Checkbox {
    fn execute(world: &mut World, props: CheckboxProps) -> Box<dyn FnMut(&mut World) + Send + Sync> {
        struct CheckboxMount {
            props: CheckboxProps,
            checkbox_entity: Entity,
            last_interaction: Interaction,
            checked: bool,
        }
        let checkbox_entity = world
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
            .id();
        let mount: Arc<RwLock<CheckboxMount>> = Arc::new(RwLock::new(
            CheckboxMount {
                props,
                checkbox_entity,
                last_interaction: Interaction::None,
                checked: false,
            }
        ));
        world.fgr_on_cleanup(cloned!((mount) => move |world| {
            world.despawn(mount.read().unwrap().checkbox_entity);
        }));
        let update: Box<dyn FnMut(&mut World) + Send + Sync> = Box::new(cloned!((mount) => move |world| {
            let mount = &mut *mount.write().unwrap();
            let entity = mount.checkbox_entity;
            let Some(interaction) = world.get::<Interaction>(entity) else { return; };
            if *interaction == mount.last_interaction {
                return;
            }
            mount.last_interaction = *interaction;
            if *interaction == Interaction::Pressed {
                mount.checked = !mount.checked;
                world
                    .get_mut::<BackgroundColor>(entity)
                    .unwrap()
                    .0 = if mount.checked { RED.into() } else { Color::BLACK };
                if let Some(on_changed) = &mut mount.props.on_changed {
                    on_changed(world, mount.checked);
                }
            }
        }));
        update
    }
}
