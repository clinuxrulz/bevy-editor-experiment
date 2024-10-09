use bevy::{asset::Assets, color::{palettes::css::BLUE, Color}, prelude::{default, Entity, Mesh, Rectangle, World}, sprite::{ColorMaterial, MaterialMesh2dBundle, Mesh2dHandle}};

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
        let cursor;
        {
            let Some(mut meshes) = world.get_resource_mut::<Assets<Mesh>>() else { return; };
            cursor = Mesh2dHandle(meshes.add(Rectangle::new(5.0, 50.0)));
        }
        let cursor_material;
        {
            let Some(mut materials) = world.get_resource_mut::<Assets<ColorMaterial>>() else { return; };
            cursor_material = materials.add(Into::<Color>::into(BLUE));
        }
        let cursor_entity = world.spawn(MaterialMesh2dBundle {
            mesh: cursor,
            material: cursor_material,
            ..default()
        }).id();
        self.cursor_entity = Some(cursor_entity);
    }

    fn unmount(&mut self, world: &mut World) {
        if let Some(cursor_entity) = self.cursor_entity {
            world.despawn(cursor_entity);
        }
    }

    fn update(&mut self, world: &mut World) {
    }
}
