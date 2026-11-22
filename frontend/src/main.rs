mod components;

use components::kanban::KanbanBoard;
use dioxus::prelude::*;

fn main() {
    dioxus::launch(App);
}

#[component]
fn App() -> Element {
    rsx! {
        div {
            style: "background: #f8f9fa; min-height: 100vh; padding: 20px;",
            h1 {
                style: "color: #1f2937; margin-bottom: 20px;",
                "Personal Dashboard"
            }
            KanbanBoard {}
        }
    }
}
