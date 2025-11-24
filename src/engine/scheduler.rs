use crate::engine::Priority;
use std::cmp::Ordering;
use std::collections::BinaryHeap;
use std::future::Future;
use tokio::task::JoinHandle;

/// Wrapper for prioritized tasks
struct PrioritizedTask<T> {
    priority: Priority,
    handle: JoinHandle<T>,
    task_id: usize,
}

impl<T> PartialEq for PrioritizedTask<T> {
    fn eq(&self, other: &Self) -> bool {
        self.priority == other.priority && self.task_id == other.task_id
    }
}

impl<T> Eq for PrioritizedTask<T> {}

impl<T> PartialOrd for PrioritizedTask<T> {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl<T> Ord for PrioritizedTask<T> {
    fn cmp(&self, other: &Self) -> Ordering {
        // Higher priority first, then FIFO by task_id
        match self.priority.cmp(&other.priority) {
            Ordering::Equal => other.task_id.cmp(&self.task_id),
            other => other,
        }
    }
}

/// Priority-based task scheduler
pub struct PipelineScheduler<T> {
    max_concurrent: usize,
    active_tasks: Vec<JoinHandle<T>>,
    pending_queue: BinaryHeap<PrioritizedTask<T>>,
    next_task_id: usize,
    completed: Vec<T>,
}

impl<T: Send + 'static> PipelineScheduler<T> {
    pub fn new(max_concurrent: usize) -> Self {
        Self {
            max_concurrent,
            active_tasks: Vec::new(),
            pending_queue: BinaryHeap::new(),
            next_task_id: 0,
            completed: Vec::new(),
        }
    }

    /// Schedule a task with given priority
    /// Returns true if task started immediately, false if queued
    pub async fn schedule_task<F>(&mut self, priority: Priority, future: F) -> bool
    where
        F: Future<Output = T> + Send + 'static,
    {
        let handle = tokio::spawn(future);
        let task = PrioritizedTask {
            priority,
            handle,
            task_id: self.next_task_id,
        };
        self.next_task_id += 1;

        if self.active_tasks.len() < self.max_concurrent {
            self.active_tasks.push(task.handle);
            true
        } else {
            self.pending_queue.push(task);
            false
        }
    }

    /// Get number of currently active tasks
    pub fn active_count(&self) -> usize {
        self.active_tasks.len()
    }

    /// Get number of pending tasks
    pub fn pending_count(&self) -> usize {
        self.pending_queue.len()
    }

    /// Poll for completed tasks and start pending ones
    #[allow(dead_code)]
    async fn poll_completions(&mut self) {
        // Check for completed active tasks
        let mut i = 0;
        while i < self.active_tasks.len() {
            if self.active_tasks[i].is_finished() {
                let handle = self.active_tasks.remove(i);
                if let Ok(result) = handle.await {
                    self.completed.push(result);
                }
            } else {
                i += 1;
            }
        }

        // Start pending tasks if slots available
        while self.active_tasks.len() < self.max_concurrent {
            if let Some(task) = self.pending_queue.pop() {
                self.active_tasks.push(task.handle);
            } else {
                break;
            }
        }
    }

    /// Wait for all tasks to complete and return results
    pub async fn wait_all(mut self) -> Vec<T> {
        // Move all pending to active
        while let Some(task) = self.pending_queue.pop() {
            self.active_tasks.push(task.handle);
        }

        // Wait for all active tasks
        for handle in self.active_tasks {
            if let Ok(result) = handle.await {
                self.completed.push(result);
            }
        }

        self.completed
    }
}
