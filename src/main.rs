use bevy_hecs::World;
use poc_async_ecs::{
    AddInitialU32sSystem, IncrementU32sSystem, PrintU32System, Resources, Schedule,
};

fn main() {
    let mut world = World::new();
    let mut resources = Resources;
    let mut schedule = Schedule::new();
    schedule.add_system(Box::new(IncrementU32sSystem));
    schedule.add_system(Box::new(PrintU32System));
    schedule.add_system(Box::new(AddInitialU32sSystem::default()));
    schedule.run(&mut world, &mut resources);
}
