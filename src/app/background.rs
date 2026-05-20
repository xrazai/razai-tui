use std::{
    sync::mpsc::{self, Receiver, TryRecvError},
    thread,
    time::Instant,
};

pub(super) struct BackgroundTask<T> {
    started_at: Option<Instant>,
    rx: Option<Receiver<T>>,
}

impl<T> BackgroundTask<T> {
    pub(super) fn is_running(&self) -> bool {
        self.rx.is_some()
    }

    pub(super) fn started_at(&self) -> Option<Instant> {
        self.started_at
    }

    pub(super) fn try_recv(&mut self) -> Option<T> {
        let result = self.rx.as_ref().map(|receiver| receiver.try_recv())?;
        match result {
            Ok(value) => {
                self.clear();
                Some(value)
            }
            Err(TryRecvError::Empty) => None,
            Err(TryRecvError::Disconnected) => {
                self.clear();
                None
            }
        }
    }

    fn clear(&mut self) {
        self.started_at = None;
        self.rx = None;
    }
}

impl<T> BackgroundTask<T>
where
    T: Send + 'static,
{
    pub(super) fn start<F>(&mut self, task: F)
    where
        F: FnOnce() -> T + Send + 'static,
    {
        let (tx, rx) = mpsc::channel();
        self.started_at = Some(Instant::now());
        self.rx = Some(rx);
        thread::spawn(move || {
            let _ = tx.send(task());
        });
    }
}

impl<T> Default for BackgroundTask<T> {
    fn default() -> Self {
        Self {
            started_at: None,
            rx: None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn clears_when_task_result_is_received() {
        let mut task = BackgroundTask::default();
        task.start(|| String::from("ok"));

        let mut result = None;
        for _ in 0..20 {
            result = task.try_recv();
            if result.is_some() {
                break;
            }
            thread::sleep(std::time::Duration::from_millis(10));
        }

        assert_eq!(result.as_deref(), Some("ok"));
        assert!(!task.is_running());
        assert!(task.started_at().is_none());
    }

    #[test]
    fn clears_when_sender_disconnects_without_result() {
        let (tx, rx) = mpsc::channel::<String>();
        drop(tx);
        let mut task = BackgroundTask {
            started_at: Some(Instant::now()),
            rx: Some(rx),
        };

        assert!(task.try_recv().is_none());
        assert!(!task.is_running());
        assert!(task.started_at().is_none());
    }
}
