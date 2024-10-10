use bevy::prelude::{Entity, World};

pub trait UiComponent<P> {
    fn run(world: &mut World, props: P) -> Entity;
}
