use bevy::prelude::World;

use super::Element;

pub trait UiComponent<P> {
    fn run(world: &mut World, props: P) -> Box<dyn Element + Send + Sync>;
}
