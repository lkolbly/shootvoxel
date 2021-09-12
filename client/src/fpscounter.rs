use log::*;
use std::time::{Duration, Instant};

pub struct FpsCounter {
    last_frame: Instant,
    last_print: Instant,
}

impl FpsCounter {
    pub fn new() -> Self {
        Self {
            last_frame: Instant::now(),
            last_print: Instant::now(),
        }
    }

    pub fn frame(&mut self) -> f32 {
        let duration = Instant::now() - self.last_frame;
        self.last_frame = Instant::now();
        let fps = 1.0 / duration.as_secs_f32();
        if (Instant::now() - self.last_print).as_secs_f32() > 3.0 {
            info!("FPS = {}", fps);
            self.last_print = Instant::now();
        }
        fps
    }
}
