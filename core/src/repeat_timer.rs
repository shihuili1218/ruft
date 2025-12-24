use tokio::time::Duration;

pub struct RepeatTimer {
    name: String,
    delay: Box<dyn Fn() -> Duration + Send + Sync>,
    task: Box<dyn Fn() + Send + Sync>,
}

pub struct RepeatTimerHandle {
    reset_tx: tokio::sync::mpsc::UnboundedSender<()>,
}

impl RepeatTimer {
    pub fn new(
        name: String,
        delay: Box<dyn Fn() -> Duration + Send + Sync>,
        task: Box<dyn Fn() + Send + Sync>,
    ) -> Self {
        RepeatTimer { name, delay, task }
    }

    pub fn spawn(self) -> RepeatTimerHandle {
        let (reset_tx, mut reset_rx) = tokio::sync::mpsc::unbounded_channel();

        tokio::spawn(async move {
            loop {
                let delay = (self.delay)();

                tokio::select! {
                    _ = tokio::time::sleep(delay) => {
                        (self.task)();
                    }
                    Some(_) = reset_rx.recv() => {
                        // 重置：取消当前sleep，重新计算delay
                        continue;
                    }
                }
            }
        });

        RepeatTimerHandle { reset_tx }
    }
}

impl RepeatTimerHandle {
    pub fn reset(&self) {
        let _ = self.reset_tx.send(());
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::{Arc, Mutex};

    #[tokio::test]
    async fn test_repeat_timer() {
        let counter = Arc::new(Mutex::new(0));
        let counter_clone = counter.clone();

        let timer = RepeatTimer::new(
            "test_timer".to_string(),
            Box::new(|| Duration::from_millis(100)),
            Box::new(move || {
                let mut count = counter_clone.lock().unwrap();
                *count += 1;
                println!("Task executed, count: {}", *count);
            }),
        );

        let handle = timer.spawn();

        // 等待第一次执行
        tokio::time::sleep(Duration::from_millis(150)).await;
        assert_eq!(*counter.lock().unwrap(), 1);

        // 等待第二次执行
        tokio::time::sleep(Duration::from_millis(100)).await;
        assert_eq!(*counter.lock().unwrap(), 2);
    }

    #[tokio::test]
    async fn test_reset() {
        let counter = Arc::new(Mutex::new(0));
        let counter_clone = counter.clone();

        let timer = RepeatTimer::new(
            "reset_timer".to_string(),
            Box::new(|| Duration::from_millis(100)),
            Box::new(move || {
                let mut count = counter_clone.lock().unwrap();
                *count += 1;
            }),
        );

        let handle = timer.spawn();

        // 50ms后reset（在100ms超时之前）
        tokio::time::sleep(Duration::from_millis(50)).await;
        handle.reset();

        // 再等60ms，还没到100ms，不应该执行
        tokio::time::sleep(Duration::from_millis(60)).await;
        assert_eq!(*counter.lock().unwrap(), 0);

        // 再等50ms，从reset算起已经110ms，应该执行了
        tokio::time::sleep(Duration::from_millis(50)).await;
        assert_eq!(*counter.lock().unwrap(), 1);
    }

    #[tokio::test]
    async fn test_dynamic_delay() {
        let counter = Arc::new(Mutex::new(0));
        let counter_clone = counter.clone();
        let delay_multiplier = Arc::new(Mutex::new(1));
        let delay_multiplier_clone = delay_multiplier.clone();

        let timer = RepeatTimer::new(
            "dynamic_timer".to_string(),
            Box::new(move || {
                let multiplier = *delay_multiplier_clone.lock().unwrap();
                Duration::from_millis(50 * multiplier)
            }),
            Box::new(move || {
                let mut count = counter_clone.lock().unwrap();
                *count += 1;
            }),
        );

        let handle = timer.spawn();

        // 第一次：50ms
        tokio::time::sleep(Duration::from_millis(60)).await;
        assert_eq!(*counter.lock().unwrap(), 1);

        // 改变delay倍数，重置
        *delay_multiplier.lock().unwrap() = 2;
        handle.reset();

        // 第二次：100ms
        tokio::time::sleep(Duration::from_millis(60)).await;
        assert_eq!(*counter.lock().unwrap(), 1); // 还没到

        tokio::time::sleep(Duration::from_millis(50)).await;
        assert_eq!(*counter.lock().unwrap(), 2); // 现在到了
    }
}
