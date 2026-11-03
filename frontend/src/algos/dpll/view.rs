use crate::algos::dpll::generate::DpllGenerate;
use crate::algos::dpll::practice::DpllPractice;
use crate::algos::dpll::state::{DpllAppState, Module, TrailEntry};
use crate::algos::math::lit_name;
use dioxus::prelude::*;

#[component]
pub fn DpllView() -> Element {
    let state = use_context::<Signal<DpllAppState>>();
    let module = state.read().module;
    rsx! {
        match module {
            Module::Generate => rsx! { DpllGenerate {} },
            Module::Practice => rsx! { DpllPractice {} },
        }
    }
}

#[component]
pub fn TrailChips(trail: Vec<TrailEntry>) -> Element {
    rsx! {
        if trail.is_empty() {
            span { class: "font-serif text-[17px] text-ink-faint", "∅" }
        } else {
            span { class: "flex items-center gap-1.5 flex-wrap",
                for e in trail.iter() {
                    span {
                        class: if e.decision {
                            "px-2 py-0.5 rounded-md bg-accent text-white font-serif text-[16px]"
                        } else {
                            "px-2 py-0.5 rounded-md border border-border text-ink font-serif text-[16px]"
                        },
                        title: if e.decision { "decision" } else { "propagated" },
                        "{lit_name(e.lit)}"
                    }
                }
            }
        }
    }
}
