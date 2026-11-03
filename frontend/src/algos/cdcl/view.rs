use crate::algos::cdcl::generate::CdclGenerate;
use crate::algos::cdcl::practice::CdclPractice;
use crate::algos::cdcl::state::{CdclAppState, Module, TrailEntry};
use crate::algos::math::lit_name;
use dioxus::prelude::*;

#[component]
pub fn CdclView() -> Element {
    let state = use_context::<Signal<CdclAppState>>();
    let module = state.read().module;
    rsx! {
        match module {
            Module::Generate => rsx! { CdclGenerate {} },
            Module::Practice => rsx! { CdclPractice {} },
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
                for slot in trail.iter() {
                    span {
                        class: if slot.decision {
                            "px-2 py-0.5 rounded-md bg-accent text-white font-serif text-[16px]"
                        } else {
                            "px-2 py-0.5 rounded-md border border-border text-ink font-serif text-[16px]"
                        },
                        title: match slot.antecedent {
                            Some(c) => format!("propagated by c{c}"),
                            None if slot.decision => "decision".to_string(),
                            None => "propagated".to_string(),
                        },
                        "{lit_name(slot.lit)}"
                        if let Some(c) = slot.antecedent {
                            sub { class: "ml-0.5 font-mono text-[10px] text-ink-faint", "c{c}" }
                        }
                    }
                }
            }
        }
    }
}
