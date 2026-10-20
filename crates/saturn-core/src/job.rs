use serde::{Deserialize, Serialize};
use std::collections::VecDeque;
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::mpsc::{Receiver, Sender, channel};
use std::sync::{Arc, Condvar, Mutex};
use std::thread::{self, JoinHandle};
use std::time::{SystemTime, UNIX_EPOCH};

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct JobId(pub u64);

impl std::fmt::Display for JobId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum JobKind {
    ProxyGeneration {
        media_id: u64,
        source_path: PathBuf,
    },
    WaveformDecoding {
        media_id: u64,
        source_path: PathBuf,
    },
    ExportRendering {
        output_path: PathBuf,
    },
    BatchExport {
        output_path: PathBuf,
        preset: String,
    },
    MotionTracking {
        clip_id: u64,
    },
    MediaProbe {
        path: PathBuf,
    },
    Custom {
        name: String,
    },
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum JobPriority {
    Background = 0,
    Normal = 1,
    High = 2,
    Immediate = 3,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
#[serde(tag = "status", rename_all = "snake_case")]
pub enum JobState {
    Queued,
    Running { progress: f32, phase: String },
    Completed { output: Option<String> },
    Failed { error: String },
    Cancelled,
}

impl JobState {
    pub fn is_active(&self) -> bool {
        matches!(self, JobState::Queued | JobState::Running { .. })
    }

    pub fn is_terminal(&self) -> bool {
        matches!(
            self,
            JobState::Completed { .. } | JobState::Failed { .. } | JobState::Cancelled
        )
    }
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct JobInfo {
    pub id: JobId,
    pub label: String,
    pub kind: JobKind,
    pub priority: JobPriority,
    pub state: JobState,
    pub created_at: u64,
    pub started_at: Option<u64>,
    pub completed_at: Option<u64>,
}

#[derive(Clone, Debug)]
pub struct CancellationToken(Arc<AtomicBool>);

impl Default for CancellationToken {
    fn default() -> Self {
        Self::new()
    }
}

impl CancellationToken {
    pub fn new() -> Self {
        Self(Arc::new(AtomicBool::new(false)))
    }

    pub fn cancel(&self) {
        self.0.store(true, Ordering::SeqCst);
    }

    pub fn is_cancelled(&self) -> bool {
        self.0.load(Ordering::SeqCst)
    }
}

pub struct JobContext {
    pub job_id: JobId,
    cancellation: CancellationToken,
    paused: Arc<AtomicBool>,
    progress_sender: Arc<dyn Fn(f32, String) + Send + Sync>,
}

impl JobContext {
    pub fn is_cancelled(&self) -> bool {
        self.cancellation.is_cancelled()
    }

    pub fn is_paused(&self) -> bool {
        self.paused.load(Ordering::SeqCst)
    }

    pub fn update_progress(&self, fraction: f32, phase: impl Into<String>) {
        (self.progress_sender)(fraction.clamp(0.0, 1.0), phase.into());
    }
}

pub type JobTask = Box<dyn FnOnce(&JobContext) -> Result<Option<String>, String> + Send + 'static>;
pub type JobListener = Arc<dyn Fn(&JobInfo) + Send + Sync + 'static>;

struct QueuedTask {
    info: JobInfo,
    cancellation: CancellationToken,
    result_sender: Option<Sender<Result<Option<String>, String>>>,
    task: JobTask,
}

struct InnerManager {
    next_id: AtomicU64,
    shutdown: AtomicBool,
    paused: Arc<AtomicBool>,
    queue: Mutex<JobQueueState>,
    condvar: Condvar,
    listeners: Mutex<Vec<JobListener>>,
}

struct JobQueueState {
    pending: VecDeque<QueuedTask>,
    running: Vec<(JobInfo, CancellationToken)>,
    history: VecDeque<JobInfo>,
    max_history: usize,
}

pub struct JobManager {
    inner: Arc<InnerManager>,
    _workers: Vec<JoinHandle<()>>,
}

fn now_epoch_ms() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_millis() as u64)
        .unwrap_or(0)
}

impl JobManager {
    pub fn new(num_workers: usize) -> Self {
        let workers_count = num_workers.max(1);
        let inner = Arc::new(InnerManager {
            next_id: AtomicU64::new(1),
            shutdown: AtomicBool::new(false),
            paused: Arc::new(AtomicBool::new(false)),
            queue: Mutex::new(JobQueueState {
                pending: VecDeque::new(),
                running: Vec::new(),
                history: VecDeque::new(),
                max_history: 100,
            }),
            condvar: Condvar::new(),
            listeners: Mutex::new(Vec::new()),
        });

        let mut workers = Vec::with_capacity(workers_count);
        for worker_idx in 0..workers_count {
            let inner_clone = Arc::clone(&inner);
            let handle = thread::Builder::new()
                .name(format!("eep-worker-{worker_idx}"))
                .spawn(move || {
                    Self::worker_loop(inner_clone);
                })
                .expect("Failed to spawn job worker thread");
            workers.push(handle);
        }

        Self {
            inner,
            _workers: workers,
        }
    }

    pub fn add_listener(&self, listener: impl Fn(&JobInfo) + Send + Sync + 'static) {
        if let Ok(mut listeners) = self.inner.listeners.lock() {
            listeners.push(Arc::new(listener));
        }
    }

    fn notify_listeners(inner: &InnerManager, info: &JobInfo) {
        if let Ok(listeners) = inner.listeners.lock() {
            for listener in listeners.iter() {
                listener(info);
            }
        }
    }

    pub fn submit(
        &self,
        label: impl Into<String>,
        kind: JobKind,
        priority: JobPriority,
        task: impl FnOnce(&JobContext) -> Result<Option<String>, String> + Send + 'static,
    ) -> (
        JobId,
        CancellationToken,
        Receiver<Result<Option<String>, String>>,
    ) {
        let id = JobId(self.inner.next_id.fetch_add(1, Ordering::SeqCst));
        let cancellation = CancellationToken::new();
        let (tx, rx) = channel();

        let info = JobInfo {
            id,
            label: label.into(),
            kind,
            priority,
            state: JobState::Queued,
            created_at: now_epoch_ms(),
            started_at: None,
            completed_at: None,
        };

        let queued_item = QueuedTask {
            info: info.clone(),
            cancellation: cancellation.clone(),
            result_sender: Some(tx),
            task: Box::new(task),
        };

        {
            let mut state = self.inner.queue.lock().unwrap();
            let insert_idx = state
                .pending
                .iter()
                .position(|item| item.info.priority < priority)
                .unwrap_or(state.pending.len());
            state.pending.insert(insert_idx, queued_item);
        }

        Self::notify_listeners(&self.inner, &info);
        self.inner.condvar.notify_one();

        (id, cancellation, rx)
    }

    pub fn submit_detached(
        &self,
        label: impl Into<String>,
        kind: JobKind,
        priority: JobPriority,
        task: impl FnOnce(&JobContext) -> Result<Option<String>, String> + Send + 'static,
    ) -> (JobId, CancellationToken) {
        let (id, cancellation, _rx) = self.submit(label, kind, priority, task);
        (id, cancellation)
    }

    pub fn cancel(&self, id: JobId) -> bool {
        let mut state = self.inner.queue.lock().unwrap();

        // Check if it's pending in the queue
        if let Some(pos) = state.pending.iter().position(|item| item.info.id == id) {
            let mut queued = state.pending.remove(pos).unwrap();
            queued.cancellation.cancel();
            queued.info.state = JobState::Cancelled;
            queued.info.completed_at = Some(now_epoch_ms());

            if let Some(sender) = queued.result_sender {
                let _ = sender.send(Err("Job was cancelled".into()));
            }

            let info_copy = queued.info.clone();
            Self::push_history(&mut state, queued.info);
            drop(state);

            Self::notify_listeners(&self.inner, &info_copy);
            return true;
        }

        // Check if it's currently running
        if let Some((info, cancellation)) = state.running.iter_mut().find(|(item, _)| item.id == id)
        {
            cancellation.cancel();
            info.state = JobState::Cancelled;
            let info_copy = info.clone();
            drop(state);

            Self::notify_listeners(&self.inner, &info_copy);
            return true;
        }

        false
    }

    pub fn cancel_all(&self) {
        let mut cancelled_infos = Vec::new();
        {
            let mut state = self.inner.queue.lock().unwrap();
            while let Some(mut queued) = state.pending.pop_front() {
                queued.cancellation.cancel();
                queued.info.state = JobState::Cancelled;
                queued.info.completed_at = Some(now_epoch_ms());
                if let Some(sender) = queued.result_sender {
                    let _ = sender.send(Err("Job was cancelled".into()));
                }
                cancelled_infos.push(queued.info.clone());
                Self::push_history(&mut state, queued.info);
            }
            for (info, cancellation) in state.running.iter_mut() {
                cancellation.cancel();
                info.state = JobState::Cancelled;
                cancelled_infos.push(info.clone());
            }
        }

        for info in cancelled_infos {
            Self::notify_listeners(&self.inner, &info);
        }
    }

    pub fn get_job(&self, id: JobId) -> Option<JobInfo> {
        let state = self.inner.queue.lock().unwrap();
        if let Some(item) = state.pending.iter().find(|i| i.info.id == id) {
            return Some(item.info.clone());
        }
        if let Some((info, _)) = state.running.iter().find(|(i, _)| i.id == id) {
            return Some(info.clone());
        }
        state.history.iter().find(|i| i.id == id).cloned()
    }

    pub fn list_active(&self) -> Vec<JobInfo> {
        let state = self.inner.queue.lock().unwrap();
        let mut list = Vec::new();
        for (info, _) in &state.running {
            list.push(info.clone());
        }
        for item in &state.pending {
            list.push(item.info.clone());
        }
        list
    }

    pub fn list_all(&self) -> Vec<JobInfo> {
        let state = self.inner.queue.lock().unwrap();
        let mut list = Vec::new();
        for (info, _) in &state.running {
            list.push(info.clone());
        }
        for item in &state.pending {
            list.push(item.info.clone());
        }
        for info in &state.history {
            list.push(info.clone());
        }
        list
    }

    pub fn clear_history(&self) {
        let mut state = self.inner.queue.lock().unwrap();
        state.history.clear();
    }

    pub fn pause(&self) {
        self.inner.paused.store(true, Ordering::SeqCst);
    }

    pub fn resume(&self) {
        self.inner.paused.store(false, Ordering::SeqCst);
        self.inner.condvar.notify_all();
    }

    pub fn is_paused(&self) -> bool {
        self.inner.paused.load(Ordering::SeqCst)
    }

    fn push_history(state: &mut JobQueueState, info: JobInfo) {
        if state.history.len() >= state.max_history {
            state.history.pop_back();
        }
        state.history.push_front(info);
    }

    fn worker_loop(inner: Arc<InnerManager>) {
        loop {
            let task_to_run = {
                let mut state = inner.queue.lock().unwrap();
                loop {
                    if inner.shutdown.load(Ordering::SeqCst) {
                        return;
                    }
                    if inner.paused.load(Ordering::SeqCst) {
                        state = inner.condvar.wait(state).unwrap();
                        continue;
                    }
                    if let Some(queued) = state.pending.pop_front() {
                        let mut info = queued.info;
                        info.state = JobState::Running {
                            progress: 0.0,
                            phase: "Starting".into(),
                        };
                        info.started_at = Some(now_epoch_ms());
                        state
                            .running
                            .push((info.clone(), queued.cancellation.clone()));

                        break Some((info, queued.cancellation, queued.result_sender, queued.task));
                    }
                    state = inner.condvar.wait(state).unwrap();
                }
            };

            let Some((mut info, cancellation, result_sender, task)) = task_to_run else {
                continue;
            };

            Self::notify_listeners(&inner, &info);

            let inner_for_progress = Arc::clone(&inner);
            let job_id = info.id;
            let progress_sender: Arc<dyn Fn(f32, String) + Send + Sync> =
                Arc::new(move |fraction, phase| {
                    let updated_info = {
                        let mut state = inner_for_progress.queue.lock().unwrap();
                        if let Some((run_info, _)) =
                            state.running.iter_mut().find(|(i, _)| i.id == job_id)
                        {
                            run_info.state = JobState::Running {
                                progress: fraction,
                                phase: phase.clone(),
                            };
                            Some(run_info.clone())
                        } else {
                            None
                        }
                    };
                    if let Some(info) = updated_info {
                        Self::notify_listeners(&inner_for_progress, &info);
                    }
                });

            let ctx = JobContext {
                job_id,
                cancellation: cancellation.clone(),
                paused: Arc::clone(&inner.paused),
                progress_sender,
            };

            let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| task(&ctx)));

            let final_state = match result {
                Ok(Ok(output)) => {
                    if cancellation.is_cancelled() {
                        JobState::Cancelled
                    } else {
                        JobState::Completed { output }
                    }
                }
                Ok(Err(err)) => {
                    if cancellation.is_cancelled() {
                        JobState::Cancelled
                    } else {
                        JobState::Failed { error: err }
                    }
                }
                Err(_) => JobState::Failed {
                    error: "Task panicked unexpectedly".into(),
                },
            };

            info.state = final_state.clone();
            info.completed_at = Some(now_epoch_ms());

            {
                let mut state = inner.queue.lock().unwrap();
                state.running.retain(|(item, _)| item.id != job_id);
                Self::push_history(&mut state, info.clone());
            }

            Self::notify_listeners(&inner, &info);

            if let Some(sender) = result_sender {
                let res = match &final_state {
                    JobState::Completed { output } => Ok(output.clone()),
                    JobState::Failed { error } => Err(error.clone()),
                    JobState::Cancelled => Err("Job was cancelled".into()),
                    _ => Err("Job ended in non-terminal state".into()),
                };
                let _ = sender.send(res);
            }
        }
    }
}

impl Drop for JobManager {
    fn drop(&mut self) {
        self.inner.shutdown.store(true, Ordering::SeqCst);
        self.inner.condvar.notify_all();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::AtomicUsize;
    use std::time::Duration;

    #[test]
    fn test_job_submission_and_completion() {
        let manager = JobManager::new(2);
        let completed_count = Arc::new(AtomicUsize::new(0));
        let count_clone = Arc::clone(&completed_count);

        manager.add_listener(move |info| {
            if matches!(info.state, JobState::Completed { .. }) {
                count_clone.fetch_add(1, Ordering::SeqCst);
            }
        });

        let (id, _cancel, rx) = manager.submit(
            "Test Task",
            JobKind::Custom {
                name: "test".into(),
            },
            JobPriority::Normal,
            |ctx| {
                ctx.update_progress(0.5, "Halfway");
                Ok(Some("All done".into()))
            },
        );

        let result = rx
            .recv_timeout(Duration::from_secs(2))
            .expect("Did not receive job result");
        assert_eq!(result, Ok(Some("All done".into())));

        if let Some(info) = manager.get_job(id) {
            assert!(
                matches!(info.state, JobState::Completed { output: Some(ref s) } if s == "All done")
            );
        } else {
            panic!("Job not found");
        }

        assert_eq!(completed_count.load(Ordering::SeqCst), 1);
    }

    #[test]
    fn test_job_cancellation_while_running() {
        let manager = JobManager::new(1);
        let (id, _cancel, rx) = manager.submit(
            "Long running task",
            JobKind::Custom {
                name: "sleep".into(),
            },
            JobPriority::Normal,
            |ctx| {
                for _ in 0..100 {
                    if ctx.is_cancelled() {
                        return Err("Cancelled by request".into());
                    }
                    thread::sleep(Duration::from_millis(10));
                }
                Ok(None)
            },
        );

        thread::sleep(Duration::from_millis(25));
        let cancelled = manager.cancel(id);
        assert!(cancelled);

        let result = rx
            .recv_timeout(Duration::from_secs(2))
            .expect("Did not receive result after cancel");
        assert!(result.is_err());

        if let Some(info) = manager.get_job(id) {
            assert_eq!(info.state, JobState::Cancelled);
        }
    }

    #[test]
    fn test_job_priority_ordering() {
        let manager = JobManager::new(1);
        let blocker_done = Arc::new(AtomicBool::new(false));
        let blocker_done_clone = Arc::clone(&blocker_done);
        manager.submit_detached(
            "Blocker",
            JobKind::Custom {
                name: "blocker".into(),
            },
            JobPriority::Immediate,
            move |_ctx| {
                while !blocker_done_clone.load(Ordering::SeqCst) {
                    thread::sleep(Duration::from_millis(5));
                }
                Ok(None)
            },
        );

        let execution_order = Arc::new(Mutex::new(Vec::new()));
        let order_clone1 = Arc::clone(&execution_order);
        let order_clone2 = Arc::clone(&execution_order);

        manager.submit_detached(
            "Low priority",
            JobKind::Custom { name: "low".into() },
            JobPriority::Background,
            move |_ctx| {
                order_clone1.lock().unwrap().push("low");
                Ok(None)
            },
        );

        manager.submit_detached(
            "High priority",
            JobKind::Custom {
                name: "high".into(),
            },
            JobPriority::High,
            move |_ctx| {
                order_clone2.lock().unwrap().push("high");
                Ok(None)
            },
        );

        blocker_done.store(true, Ordering::SeqCst);

        for _ in 0..100 {
            let active = manager.list_active();
            if active.is_empty() {
                break;
            }
            thread::sleep(Duration::from_millis(10));
        }

        let order = execution_order.lock().unwrap().clone();
        assert_eq!(order, vec!["high", "low"]);
    }
}
