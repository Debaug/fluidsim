use std::{time::{Instant, Duration}, collections::VecDeque};

#[derive(Debug, Clone, Copy)]
pub struct Timer {
    pub last_tick: Instant,
}

impl Timer {
    pub fn new() -> Self {
        Self { last_tick: Instant::now() }
    }

    pub fn tick(&mut self) {
        self.last_tick = Instant::now();
    }

    pub fn delta(&self) -> Duration {
        Instant::now() - self.last_tick
    }
}

#[derive(Debug, Clone, Default)]
pub struct FpsCounter {
    frames: VecDeque<Instant>,
}

impl FpsCounter {
    pub fn new() -> Self {
        Default::default()
    }

    pub fn add_frame(&mut self) {
        let now = Instant::now();
        self.frames.push_back(now);
    }

    pub fn fps(&mut self) -> usize {
        let now = Instant::now();
        while let Some(&first) = self.frames.front() {
            if first + Duration::new(1, 0) >= now {
                break;
            }
            self.frames.pop_front();
        }
        self.frames.len()
    }
}
