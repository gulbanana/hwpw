use core::future::Future;
use embassy_time::{Duration, Instant};

pub struct Debounced<IO: Debouncy> {
    io: IO,
    min_delay: Duration,
    deadline: Instant,
}

impl<'a, IO: Debouncy> Debounced<IO> {
    pub fn new(io: IO, min_delay_ms: u64) -> Self {
        Debounced {
            io,
            min_delay: Duration::from_millis(min_delay_ms),
            deadline: Instant::now(),
        }
    }

    pub async fn debounce(&mut self) -> IO::Output {
        loop {
            let output = self.io.read().await;
            if Instant::now() >= self.deadline {
                self.deadline = Instant::now() + self.min_delay;
                return output;
            }
        }
    }
}

pub trait Debouncy {
    type Output;
    fn read(&mut self) -> impl Future<Output = Self::Output>;
}
