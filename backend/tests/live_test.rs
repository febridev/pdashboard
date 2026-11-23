use pdashboard_backend::models::document::Document;

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
