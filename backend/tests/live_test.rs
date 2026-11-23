use pdashboard_backend::models::document::Document;

use std::collections::HashMap;
use pdashboard_backend::models::task::{Task, TaskGroup, TaskStatus};
use pdashboard_backend::models::response::TasksResponse;
use pdashboard_backend::services::live::{CachedDoc, DocumentCache, TaskEvents, needs_refetch, rebuild_snapshot};

fn doc(id: &str, updated: Option<&str>) -> Document {
    Document {
        id: id.to_string(),
        title: format!("Doc {id}"),
        text: None,
        tasks: Some(pdashboard_backend::models::document::DocumentTasks { completed: 0, total: 1 }),
        updated_at: updated.map(str::to_string),
    }
}

fn group() -> Vec<TaskGroup> {
    vec![TaskGroup { header: "g".into(), path: vec![], tasks: vec![Task { text: "t".into(), status: TaskStatus::Pending }] }]
}

#[test]
fn needs_refetch_is_true_for_unknown_or_changed_and_false_when_unchanged() {
    let mut cache: DocumentCache = HashMap::new();
    assert!(needs_refetch(&cache, &doc("a", Some("1"))));            // not cached yet
    cache.insert("a".into(), CachedDoc { updated_at: Some("1".into()), title: "A".into(), groups: group() });
    assert!(!needs_refetch(&cache, &doc("a", Some("1"))));          // unchanged
    assert!(needs_refetch(&cache, &doc("a", Some("2"))));            // updatedAt moved
}

#[test]
fn rebuild_follows_listed_order_and_drops_removed_docs() {
    let mut cache: DocumentCache = HashMap::new();
    cache.insert("a".into(), CachedDoc { updated_at: Some("1".into()), title: "A".into(), groups: group() });
    cache.insert("b".into(), CachedDoc { updated_at: Some("1".into()), title: "B".into(), groups: group() });
    // listed order is b then a — snapshot must follow the list, not the cache
    let snap = rebuild_snapshot(&mut cache, &[doc("b", Some("1")), doc("a", Some("1"))]);
    let titles: Vec<_> = snap.tasks.iter().map(|d| d.document_title.as_str()).collect();
    assert_eq!(titles, ["B", "A"]);
    // "a" disappears from the list -> dropped from cache and snapshot
    let snap2 = rebuild_snapshot(&mut cache, &[doc("b", Some("1"))]);
    assert_eq!(snap2.tasks.len(), 1);
    assert!(!cache.contains_key("a"));
}

#[test]
fn rebuild_skips_docs_with_empty_groups() {
    let mut cache: DocumentCache = HashMap::new();
    cache.insert("a".into(), CachedDoc { updated_at: Some("1".into()), title: "A".into(), groups: vec![] });
    let snap = rebuild_snapshot(&mut cache, &[doc("a", Some("1"))]);
    assert!(snap.tasks.is_empty());   // mirrors get_tasks' !groups.is_empty() filter
    assert!(cache.contains_key("a")); // cached state kept; snapshot exclusion is enough
}

#[test]
fn task_events_publish_makes_snapshot_visible_to_subscribers() {
    let events = TaskEvents::new();
    let mut rx = events.subscribe();
    assert!(rx.borrow_and_update().is_none());       // no snapshot yet
    events.publish(TasksResponse { tasks: vec![] });
    assert!(rx.borrow_and_update().is_some());       // latest value, even with zero prior receivers
}

#[test]
fn document_parses_camel_case_updated_at() {
    let raw = serde_json::json!({
        "id": "d1", "title": "Tasks", "text": "- [ ] a",
        "updatedAt": "2026-11-23T09:00:00.000Z"
    });
    let doc: Document = serde_json::from_value(raw).unwrap();
    assert_eq!(doc.updated_at.as_deref(), Some("2026-11-23T09:00:00.000Z"));
}

#[test]
fn document_without_updated_at_still_parses() {
    let raw = serde_json::json!({ "id": "d1", "title": "Tasks", "text": "- [ ] a" });
    let doc: Document = serde_json::from_value(raw).unwrap();
    assert!(doc.updated_at.is_none());
}
