use bevy::prelude::World;

pub trait Element {
    fn mount(&mut self, world: &mut World);
    fn unmount(&mut self, world: &mut World);
    fn update(&mut self, world: &mut World);
}

pub struct Elements(pub Vec<Box<dyn Element>>);

impl Element for Elements {
    fn mount(&mut self, world: &mut World) {
        for element in self.0.iter_mut() {
            element.mount(world);
        }
    }
    fn unmount(&mut self, world: &mut World) {
        for element in self.0.iter_mut() {
            element.unmount(world);
        }
    }
    fn update(&mut self, world: &mut World) {
        for element in self.0.iter_mut() {
            element.update(world);
        }
    }
}
