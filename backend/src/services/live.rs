//! Live updates: change detection against a document cache, snapshot
//! rebuild, and the watch channel that carries snapshots to SSE clients.

use crate::models::document::Document;
use crate::models::response::TasksResponse;
use crate::models::task::{DocumentTasks, TaskGroup};
use std::collections::{HashMap, HashSet};
use tokio::sync::watch;

/// Broadcast channel for task snapshots. Holds the latest snapshot so a
/// subscriber that connects late immediately sees the current state.
pub struct TaskEvents {
    tx: watch::Sender<Option<TasksResponse>>,
}

impl TaskEvents {
    pub fn new() -> Self {
        let (tx, _rx) = watch::channel(None);
        Self { tx }
    }

    /// Replace the current snapshot. Works with zero receivers: the value is
    /// stored for the next subscriber.
    pub fn publish(&self, snapshot: TasksResponse) {
        self.tx.send_replace(Some(snapshot));
    }

    pub fn subscribe(&self) -> watch::Receiver<Option<TasksResponse>> {
        self.tx.subscribe()
    }
}

/// A parsed document held in the poller's cache, keyed by document id.
pub struct CachedDoc {
    pub updated_at: Option<String>,
    pub title: String,
    pub groups: Vec<TaskGroup>,
}

pub type DocumentCache = HashMap<String, CachedDoc>;

/// True when the document has never been cached or its `updatedAt` moved.
pub fn needs_refetch(cache: &DocumentCache, doc: &Document) -> bool {
    match cache.get(&doc.id) {
        Some(cached) => cached.updated_at != doc.updated_at,
        None => true,
    }
}

/// Rebuild the full snapshot from the cache, following the order documents
/// were listed in. Drops cache entries whose id is no longer listed (the
/// document was deleted or no longer has tasks), and skips docs that are
/// not cached yet or whose parsed groups are empty — mirroring `get_tasks`.
pub fn rebuild_snapshot(cache: &mut DocumentCache, listed: &[Document]) -> TasksResponse {
    let listed_ids: HashSet<&String> = listed.iter().map(|d| &d.id).collect();
    cache.retain(|id, _| listed_ids.contains(id));

    let tasks = listed
        .iter()
        .filter_map(|d| {
            let cached = cache.get(&d.id)?;
            if cached.groups.is_empty() {
                return None;
            }
            Some(DocumentTasks {
                document_title: cached.title.clone(),
                groups: cached.groups.clone(),
            })
        })
        .collect();

    TasksResponse { tasks }
}
