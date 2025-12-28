use std::future::Future;
use std::pin::Pin;
use tokio::time::Duration;

/// Trait for tasks that need to be executed repeatedly
pub(crate) trait RepeatTask: Send + Sync {
    /// Calculate the delay before next execution
    fn delay(&self) -> Pin<Box<dyn Future<Output = Duration> + Send + '_>>;

    /// Execute the task
    fn run(&self) -> Pin<Box<dyn Future<Output = ()> + Send + '_>>;
}

pub(crate) struct RepeatTimer {
    name: String,
    task: Box<dyn RepeatTask>,
}

pub(crate) struct RepeatTimerHandle {
    restart_tx: tokio::sync::mpsc::UnboundedSender<()>,
    stop_tx: tokio::sync::mpsc::UnboundedSender<()>,
}

impl RepeatTimer {
    pub fn new(name: String, task: Box<dyn RepeatTask>) -> Self {
        RepeatTimer { name, task }
    }

    /// Create a timer from closures (for simple cases)
    pub fn from_fns<D, R>(name: String, delay_fn: D, run_fn: R) -> Self
    where
        D: Fn() -> Pin<Box<dyn Future<Output = Duration> + Send>> + Send + Sync + 'static,
        R: Fn() -> Pin<Box<dyn Future<Output = ()> + Send>> + Send + Sync + 'static,
    {
        RepeatTimer {
            name,
            task: Box::new(FnTask { delay_fn, run_fn }),
        }
    }

    pub fn spawn(self) -> RepeatTimerHandle {
        let (restart_tx, mut restart_rx) = tokio::sync::mpsc::unbounded_channel();
        let (stop_tx, mut stop_rx) = tokio::sync::mpsc::unbounded_channel();

        tokio::spawn(async move {
            loop {
                let delay = self.task.delay().await;

                tokio::select! {
                    _ = tokio::time::sleep(delay) => {
                        self.task.run().await;
                    }
                    Some(_) = restart_rx.recv() => {
                        continue;
                    }
                    Some(_) = stop_rx.recv() => {
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

/// Internal task implementation using closures
struct FnTask<D, R> {
    delay_fn: D,
    run_fn: R,
}

impl<D, R> RepeatTask for FnTask<D, R>
where
    D: Fn() -> Pin<Box<dyn Future<Output = Duration> + Send>> + Send + Sync,
    R: Fn() -> Pin<Box<dyn Future<Output = ()> + Send>> + Send + Sync,
{
    fn delay(&self) -> Pin<Box<dyn Future<Output = Duration> + Send + '_>> {
        (self.delay_fn)()
    }

    fn run(&self) -> Pin<Box<dyn Future<Output = ()> + Send + '_>> {
        (self.run_fn)()
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

        let timer = RepeatTimer::from_fns(
            "test_timer".to_string(),
            || Box::pin(async { Duration::from_millis(100) }),
            move || {
                let counter_clone = counter_clone.clone();
                Box::pin(async move {
                    let mut count = counter_clone.lock().unwrap();
                    *count += 1;
                    println!("Task executed, count: {}", *count);
                })
            },
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

        let timer = RepeatTimer::from_fns(
            "restart_timer".to_string(),
            || Box::pin(async { Duration::from_millis(100) }),
            move || {
                let counter_clone = counter_clone.clone();
                Box::pin(async move {
                    let mut count = counter_clone.lock().unwrap();
                    *count += 1;
                    println!("Task executed, count: {}", *count);
                })
            },
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

        let timer = RepeatTimer::from_fns(
            "stop_timer".to_string(),
            || Box::pin(async { Duration::from_millis(100) }),
            move || {
                let counter_clone = counter_clone.clone();
                Box::pin(async move {
                    let mut count = counter_clone.lock().unwrap();
                    *count += 1;
                    println!("Task executed, count: {}", *count);
                })
            },
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

        let timer = RepeatTimer::from_fns(
            "dynamic_timer".to_string(),
            move || {
                let delay_multiplier_clone = delay_multiplier_clone.clone();
                Box::pin(async move {
                    let multiplier = *delay_multiplier_clone.lock().unwrap();
                    Duration::from_millis(50 * multiplier)
                })
            },
            move || {
                let counter_clone = counter_clone.clone();
                Box::pin(async move {
                    let mut count = counter_clone.lock().unwrap();
                    *count += 1;
                    println!("Task executed, count: {}", *count);
                })
            },
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
