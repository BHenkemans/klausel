use dioxus::prelude::*;

#[component]
pub fn Footer() -> Element {
    rsx! {
        footer { class: "border-t border-border bg-topbar px-6 py-4 text-center text-[12px] text-ink-muted",
            "Frontend made with AI, research done by "
            span { class: "font-semibold text-ink-2", "Bartjan Henkemans" }
            " at "
            a { href:"https://fsa.win.tue.nl/", "Formal System Analysis"}
        }
    }
}
