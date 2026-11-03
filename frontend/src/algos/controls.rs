//! Designer and practice controls shared by all three algorithms.
//!
//! The knob ceilings live here too: they bound what the generators are asked
//! to search for, so they are a property of the exercise format rather than of
//! any one rule set.

use dioxus::prelude::*;

pub const MIN_LIMIT: usize = 1;
pub const MAX_VARS_LIMIT: usize = 20;
pub const MAX_CLAUSES_LIMIT: usize = 20;
pub const MAX_STEPS_LIMIT: usize = 20;

#[component]
pub fn Stepper(
    label: &'static str,
    value: usize,
    on_dec: EventHandler<()>,
    on_inc: EventHandler<()>,
) -> Element {
    rsx! {
        div { class: "flex items-center justify-between",
            span { class: "text-[13px] text-ink-2", "{label}" }
            div { class: "flex items-center border border-border rounded-[9px] overflow-hidden bg-surface",
                button {
                    class: "px-3 py-1 text-ink-muted text-[15px] cursor-pointer hover:text-ink hover:bg-surface-2",
                    onclick: move |_| on_dec.call(()),
                    "−"
                }
                div { class: "px-3 py-1 text-[13px] font-bold text-ink border-l border-r border-border-soft min-w-[14px] text-center",
                    "{value}"
                }
                button {
                    class: "px-3 py-1 text-ink-muted text-[15px] cursor-pointer hover:text-ink hover:bg-surface-2",
                    onclick: move |_| on_inc.call(()),
                    "＋"
                }
            }
        }
    }
}

#[component]
pub fn TrailLegend() -> Element {
    rsx! {
        div { class: "flex items-center flex-wrap gap-1.5 px-3 text-[11.5px] text-ink-muted",
            span { class: "px-1.5 rounded bg-accent text-white font-serif text-[12px]", "α" }
            "decision"
            span { class: "ml-2 px-1.5 rounded bg-surface border border-border-soft text-ink font-serif text-[12px]", "α" }
            "propagated"
            span { class: "ml-2 text-ink-faint", "— appended literals start as propagated; click one in the trail to switch." }
        }
    }
}
