//! Cooperative progress/cancellation for blocking traversals and hashing.
use serde::Serialize;
use std::{
    cell::RefCell,
    collections::HashMap,
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc, LazyLock, Mutex,
    },
};
#[derive(Clone, Default, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Status {
    pub running: bool,
    pub visited: u64,
    pub skipped: u64,
    pub current_path: String,
    pub cancelled: bool,
    pub notice: String,
}
struct Task {
    status: Mutex<Status>,
    cancel: AtomicBool,
}
static TASKS: LazyLock<Mutex<HashMap<String, Arc<Task>>>> =
    LazyLock::new(|| Mutex::new(HashMap::new()));
thread_local! { static CURRENT: RefCell<Option<Arc<Task>>> = const { RefCell::new(None) }; }
pub struct Guard(Arc<Task>);
pub fn start(kind: &str) -> Result<Guard, String> {
    let mut tasks = TASKS.lock().unwrap();
    if tasks
        .get(kind)
        .is_some_and(|t| t.status.lock().unwrap().running)
    {
        return Err("同类扫描正在运行，请先取消或等待完成".into());
    }
    let task = Arc::new(Task {
        status: Mutex::new(Status {
            running: true,
            ..Default::default()
        }),
        cancel: AtomicBool::new(false),
    });
    tasks.insert(kind.into(), task.clone());
    Ok(Guard(task))
}
impl Guard {
    pub fn run<T>(self, f: impl FnOnce() -> T) -> Result<T, String> {
        CURRENT.with(|c| *c.borrow_mut() = Some(self.0.clone()));
        let value = f();
        if cancelled() {
            Err("扫描已取消，未使用不完整结果".into())
        } else {
            Ok(value)
        }
    }
}
impl Drop for Guard {
    fn drop(&mut self) {
        self.0.status.lock().unwrap().running = false;
        CURRENT.with(|c| *c.borrow_mut() = None);
    }
}
pub fn status(kind: &str) -> Status {
    TASKS
        .lock()
        .unwrap()
        .get(kind)
        .map(|t| t.status.lock().unwrap().clone())
        .unwrap_or_default()
}
pub fn cancel(kind: &str) {
    if let Some(t) = TASKS.lock().unwrap().get(kind) {
        t.cancel.store(true, Ordering::Relaxed);
        t.status.lock().unwrap().cancelled = true;
    }
}
pub fn cancelled() -> bool {
    CURRENT.with(|c| {
        c.borrow()
            .as_ref()
            .is_some_and(|t| t.cancel.load(Ordering::Relaxed))
    })
}
pub fn visit(path: &std::path::Path) -> bool {
    CURRENT.with(|c| {
        if let Some(t) = c.borrow().as_ref() {
            let mut s = t.status.lock().unwrap();
            s.visited += 1;
            s.current_path = path.to_string_lossy().into_owned();
        }
    });
    !cancelled()
}
pub fn skip() {
    CURRENT.with(|c| {
        if let Some(t) = c.borrow().as_ref() {
            t.status.lock().unwrap().skipped += 1;
        }
    });
}
pub fn notice(text: &str) {
    CURRENT.with(|c| {
        if let Some(t) = c.borrow().as_ref() {
            t.status.lock().unwrap().notice = text.into();
        }
    });
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn cancellation_discards_partial_results_and_allows_restart() {
        let guard = start("test-cancel").unwrap();
        assert!(start("test-cancel").is_err());
        cancel("test-cancel");
        assert!(guard
            .run(|| {
                visit(std::path::Path::new("/fixture"));
                vec!["partial"]
            })
            .is_err());
        assert!(!status("test-cancel").running);
        assert!(status("test-cancel").cancelled);
        assert!(start("test-cancel").is_ok());
    }
}
