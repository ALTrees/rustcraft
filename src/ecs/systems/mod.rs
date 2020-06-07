use specs::{System, Write};

pub use input::*;
pub use physics::*;
pub use player::*;
pub use fps_counter::*;
pub use hand::*;
pub use inventory::*;

use crate::timer::Timer;

pub mod input;
pub mod physics;
pub mod player;
pub mod fps_counter;
pub mod hand;
pub mod inventory;

pub struct AdvanceGlobalTime;

impl<'a> System<'a> for AdvanceGlobalTime {
    type SystemData = (
        Write<'a, Timer>,
    );

    fn run(&mut self, (mut global_timer, ): Self::SystemData) {
        global_timer.tick();
    }
}

