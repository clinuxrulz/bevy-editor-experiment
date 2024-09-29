use bevy::prelude::World;

pub trait UiComponentMount {
    fn init(&mut self, world: &mut World);
    fn update(&mut self, world: &mut World);
    fn dispose(&mut self, world: &mut World);
}

pub trait UiComponent<P> {
    fn execute(props: P) -> impl UiComponentMount;
}
