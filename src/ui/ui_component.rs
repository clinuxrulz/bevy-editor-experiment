use bevy::prelude::World;

pub trait UiComponent<P> {
    fn execute(world: &mut World, props: P) -> Box<dyn FnMut(&mut World) + Send + Sync>;
}
