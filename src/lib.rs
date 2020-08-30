use async_trait::async_trait;
use bevy_hecs::World;
use std::{
    future::Future,
    pin::Pin,
    sync::{Arc, Mutex},
    task::{Context, RawWaker, RawWakerVTable, Waker},
};

pub struct Resources;

// Async systems have an async run() method. run() is only meant to be called
// *once*. The future it returns can be stored and repeatedly polled to run
// the system. For foreach systems, the run method can wrap the synchronous
// ForEachSystemFn in an infinite loop. For an asset loading system, the
// run method could be a typical async fn, which awaits IO calls and so
// on. Startup systems can be systems where the run method terminates.
#[async_trait]
pub trait System {
    async fn run(&mut self, world: Arc<World>, resources: Arc<Resources>);
    fn run_thread_local(&mut self, world: &mut World, resources: &mut Resources);
}

type SystemFuture<'a> = Pin<Box<dyn Future<Output = ()> + Send + 'a>>;

// Waker does nothing for demo, but could set a "ready" flag for the woken
// system in the Schedule.
fn do_nothing(_: *const ()) {}
fn clone(data: *const ()) -> RawWaker {
    RawWaker::new(data, &VTABLE)
}
const VTABLE: RawWakerVTable = RawWakerVTable::new(clone, do_nothing, do_nothing, do_nothing);

// Simple demo schedule just loops through all the systems and advances
// their futures by one step. Futures could be advanced in parallel using
// multiple threads. Could use bevy_tasks?
//
// After advancing all futures, runs the thread local parts of the system.
// The thread local parts of the system could, for instance, apply queued
// writes to the World.
pub struct Schedule<'a> {
    systems: Vec<(Arc<Mutex<Box<dyn System + 'a>>>, Option<SystemFuture<'a>>)>,
    waker: Waker,
}

impl<'a> Schedule<'a> {
    pub fn new() -> Self {
        let raw_waker = clone(&());
        let waker = unsafe { Waker::from_raw(raw_waker) };
        Schedule {
            systems: Vec::new(),
            waker,
        }
    }

    pub fn add_system(&mut self, system: Box<dyn System>) {
        self.systems.push((Arc::new(Mutex::new(system)), None));
    }

    pub fn run(&'a mut self, world: Arc<World>, resources: Arc<Resources>) {
        let mut context = Context::from_waker(&self.waker);
        let mut finished_systems = Vec::new();
        for (i, (system, state)) in self.systems.iter_mut().enumerate() {
            match state {
                Some(future) => {
                    use std::task::Poll::*;
                    match Future::poll(future.as_mut(), &mut context) {
                        Ready(()) => {
                            finished_systems.push(i);
                        }
                        Pending => (),
                    }
                }
                None => {
                    let future = system.run(Arc::clone(&world), Arc::clone(&resources));
                    *state = Some(Box::pin(future));
                }
            }
        }
        // sync barrier here
        for (system, _) in self.systems.iter_mut() {
            system.run_thread_local(&mut world, &mut resources);
        }
        for i in finished_systems {
            self.systems.remove(i);
        }
    }
}

pub struct PrintU32System;

#[async_trait]
impl System for PrintU32System {
    async fn run(&mut self, world: Arc<World>, _: Arc<Resources>) {
        loop {
            for n in world.query::<&u32>().iter() {
                print!("{} ", n);
            }
            println!();
            // This is used to yield in this demo, but could be replaced by a
            // Timer::from_sec(1.0).await or something.
            async { () }.await;
        }
    }

    fn run_thread_local(&mut self, _: &mut World, _: &mut Resources) {}
}

// This system demonstrates a simplified version of how you might queue
// up world mutations and then resolve them in the thread local part of
// the system.
//
// Also note that this system terminates after 5 iterations.
#[derive(Default)]
pub struct AddInitialU32sSystem {
    u32s_to_add: Vec<u32>,
}

#[async_trait]
impl System for AddInitialU32sSystem {
    async fn run(&mut self, _: Arc<World>, _: Arc<Resources>) {
        for i in 0..5 {
            self.u32s_to_add.push(i);
            self.u32s_to_add.push(i + 5);
            async { () }.await;
        }
    }

    fn run_thread_local(&mut self, world: &mut World, _: &mut Resources) {
        for i in self.u32s_to_add.drain(0..) {
            world.spawn((i,));
        }
    }
}

pub struct IncrementU32sSystem;

#[async_trait]
impl System for IncrementU32sSystem {
    async fn run(&mut self, world: Arc<World>, _: Arc<Resources>) {
        loop {
            for mut n in world.query::<&mut u32>().iter() {
                *n += 1;
            }
            async { () }.await;
        }
    }

    fn run_thread_local(&mut self, _: &mut World, _: &mut Resources) {}
}

#[cfg(test)]
mod tests {
    #[test]
    fn it_works() {
        assert_eq!(2 + 2, 4);
    }
}
