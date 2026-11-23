//! Live updates: change detection against a document cache, snapshot
//! rebuild, and the watch channel that carries snapshots to SSE clients.

use crate::models::document::Document;
use crate::models::response::TasksResponse;
use crate::models::task::{DocumentTasks, TaskGroup};
use crate::services::outline::OutlineClient;
use crate::services::parser::parse_tasks_from_markdown;
use std::collections::{HashMap, HashSet};
use std::time::Duration;
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

/// Spawn the background poller. The first refresh runs immediately at
/// startup (warming the snapshot before any browser connects), then
/// every `interval`. An Outline failure keeps the last published snapshot.
pub fn spawn_poller(outline: OutlineClient, events: std::sync::Arc<TaskEvents>, interval: Duration) {
    tokio::spawn(async move {
        let mut cache: DocumentCache = HashMap::new();
        loop {
            if let Err(e) = refresh_once(&outline, &events, &mut cache).await {
                tracing::warn!("live: refresh failed, keeping last snapshot: {}", e);
            }
            tokio::time::sleep(interval).await;
        }
    });
}

async fn refresh_once(
    outline: &OutlineClient,
    events: &TaskEvents,
    cache: &mut DocumentCache,
) -> Result<(), reqwest::Error> {
    if let Some(doc_id) = outline.config.outline_task_document_id.clone() {
        let doc = outline.get_document(&doc_id).await?;
        if needs_refetch(cache, &doc) {
            let groups = doc
                .text
                .as_deref()
                .map(parse_tasks_from_markdown)
                .unwrap_or_default();
            cache.insert(
                doc.id.clone(),
                CachedDoc {
                    updated_at: doc.updated_at.clone(),
                    title: doc.title.clone(),
                    groups,
                },
            );
            events.publish(rebuild_snapshot(cache, std::slice::from_ref(&doc)));
            tracing::info!("live: published snapshot for document {}", doc_id);
        }
    } else {
        let docs = outline.list_documents().await?;
        let relevant: Vec<Document> = docs
            .into_iter()
            .filter(|d| d.tasks.as_ref().map_or(false, |t| t.total > 0))
            .collect();
        let listed_ids: HashSet<&String> = relevant.iter().map(|d| &d.id).collect();
        let cached_ids: HashSet<&String> = cache.keys().collect();
        if listed_ids != cached_ids || relevant.iter().any(|d| needs_refetch(cache, d)) {
            for d in &relevant {
                if !needs_refetch(cache, d) {
                    continue;
                }
                match outline.get_document(&d.id).await {
                    Ok(full) => {
                        let groups = full
                            .text
                            .as_deref()
                            .map(parse_tasks_from_markdown)
                            .unwrap_or_default();
                        cache.insert(
                            full.id.clone(),
                            CachedDoc {
                                updated_at: full.updated_at.clone(),
                                title: full.title.clone(),
                                groups,
                            },
                        );
                    }
                    Err(e) => tracing::warn!("live: failed to fetch {}: {}", d.id, e), // keep prior cache entry
                }
            }
            events.publish(rebuild_snapshot(cache, &relevant));
            tracing::info!("live: published snapshot ({} documents)", relevant.len());
        }
    }
    Ok(())
}
