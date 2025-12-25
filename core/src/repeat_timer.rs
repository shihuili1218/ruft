use tokio::time::Duration;

pub(crate) struct RepeatTimer {
    name: String,
    delay: Box<dyn Fn() -> Duration + Send + Sync>,
    task: Box<dyn Fn() + Send + Sync>,
}

pub(crate) struct RepeatTimerHandle {
    restart_tx: tokio::sync::mpsc::UnboundedSender<()>,
    stop_tx: tokio::sync::mpsc::UnboundedSender<()>,
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
        let (restart_tx, mut restart_rx) = tokio::sync::mpsc::unbounded_channel();
        let (stop_tx, mut stop_rx) = tokio::sync::mpsc::unbounded_channel();

        tokio::spawn(async move {
            loop {
                let delay = (self.delay)();

                tokio::select! {
                    _ = tokio::time::sleep(delay) => {
                        (self.task)();
                    }
                    Some(_) = restart_rx.recv() => {
                        // 重启：取消当前sleep，重新计算delay
                        continue;
                    }
                    Some(_) = stop_rx.recv() => {
                        // 停止：退出loop
                        break;
                    }
                }
            }
        });

        RepeatTimerHandle { restart_tx, stop_tx }
    }
}

impl RepeatTimerHandle {
    pub fn restart(&self) {
        let _ = self.restart_tx.send(());
    }

    pub fn stop(&self) {
        let _ = self.stop_tx.send(());
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
    async fn test_restart() {
        let counter = Arc::new(Mutex::new(0));
        let counter_clone = counter.clone();

        let timer = RepeatTimer::new(
            "restart_timer".to_string(),
            Box::new(|| Duration::from_millis(100)),
            Box::new(move || {
                let mut count = counter_clone.lock().unwrap();
                *count += 1;
            }),
        );

        let handle = timer.spawn();

        // 50ms后restart（在100ms超时之前）
        tokio::time::sleep(Duration::from_millis(50)).await;
        handle.restart();

        // 再等60ms，还没到100ms，不应该执行
        tokio::time::sleep(Duration::from_millis(60)).await;
        assert_eq!(*counter.lock().unwrap(), 0);

        // 再等50ms，从restart算起已经110ms，应该执行了
        tokio::time::sleep(Duration::from_millis(50)).await;
        assert_eq!(*counter.lock().unwrap(), 1);
    }

    #[tokio::test]
    async fn test_stop() {
        let counter = Arc::new(Mutex::new(0));
        let counter_clone = counter.clone();

        let timer = RepeatTimer::new(
            "stop_timer".to_string(),
            Box::new(|| Duration::from_millis(100)),
            Box::new(move || {
                let mut count = counter_clone.lock().unwrap();
                *count += 1;
            }),
        );

        let handle = timer.spawn();

        tokio::time::sleep(Duration::from_millis(50)).await;
        handle.stop();
        handle.stop();

        tokio::time::sleep(Duration::from_millis(100)).await;
        assert_eq!(*counter.lock().unwrap(), 0);
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

        // 改变delay倍数，重启
        *delay_multiplier.lock().unwrap() = 2;
        handle.restart();

        // 第二次：100ms
        tokio::time::sleep(Duration::from_millis(60)).await;
        assert_eq!(*counter.lock().unwrap(), 1); // 还没到

        tokio::time::sleep(Duration::from_millis(50)).await;
        assert_eq!(*counter.lock().unwrap(), 2); // 现在到了
    }
}
