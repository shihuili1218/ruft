use tokio::time::{self, Duration, Interval};

pub struct RepeatTimer {
    name: String,
    delay: Box<dyn Fn() -> Duration>,
    task: Box<dyn Fn() -> ()>,
    timer: Interval,
}

impl RepeatTimer {
    pub fn new(
        name: String,
        delay: Box<dyn Fn() -> Duration>,
        task: Box<dyn Fn() -> ()>,
    ) -> RepeatTimer {
        let delay_duration = delay();
        let mut timer = time::interval(delay_duration);
        timer.set_missed_tick_behavior(time::MissedTickBehavior::Delay);
        RepeatTimer {
            name,
            delay,
            task,
            timer,
        }
    }
    pub async fn start(&mut self) {
        self.timer.tick().await;
        (self.task)();
    }

    pub fn reset(&mut self) {
        self.timer.reset_after((self.delay)());
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn it_works() {
        let interval = Duration::from_secs(1);
        let mut interval = time::interval(interval);
        let rt = tokio::runtime::Runtime::new().unwrap();
        rt.block_on(async {
            loop {
                interval.tick().await;
                println!("111");
            }
        });

    }
}
