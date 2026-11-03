use crate::Algorithm;
use crate::algos::Module;
use crate::algos::cdcl::state::CdclAppState;
use crate::algos::dp::state::DpAppState;
use crate::algos::dpll::state::DpllAppState;
use crate::layout::update_notice::UpdateNotice;
use dioxus::prelude::*;

#[component]
pub fn Topbar(mut dark: Signal<bool>, mut algo: Signal<Algorithm>) -> Element {
    let mut dp_state = use_context::<Signal<DpAppState>>();
    let mut dpll_state = use_context::<Signal<DpllAppState>>();
    let mut cdcl_state = use_context::<Signal<CdclAppState>>();
    let cur_algo = algo();

    let cur_module = match cur_algo {
        Algorithm::DavisPutnam => dp_state.read().module,
        Algorithm::Dpll => dpll_state.read().module,
        Algorithm::Cdcl => cdcl_state.read().module,
    };

    rsx! {
        div { class: "flex items-center justify-between px-6 h-topbar border-b border-border bg-topbar sticky top-0 z-20",
            div { class: "flex items-center gap-4",
                div { class: "flex items-center gap-2.5",
                    div { class: "w-[30px] h-[30px] rounded-[9px] bg-accent text-white font-serif text-[17px] flex items-center justify-center",
                        "α"
                    }
                    div { class: "font-extrabold text-[16px] tracking-tight", "Klausel" }
                }
                div { class: "w-px h-6 bg-border" }
                Segmented {
                    items: vec![
                        (Module::Generate, "Generate", true),
                        (Module::Practice, "Practice", true),
                    ],
                    current: cur_module,
                    on_pick: move |m| match cur_algo {
                        Algorithm::DavisPutnam => dp_state.write().module = m,
                        Algorithm::Dpll => dpll_state.write().module = m,
                        Algorithm::Cdcl => cdcl_state.write().module = m,
                    },
                }
            }
            div { class: "flex items-center gap-3",
                UpdateNotice {}
                Segmented {
                    items: vec![
                        (Algorithm::DavisPutnam, "Davis–Putnam", true),
                        (Algorithm::Dpll, "DPLL", true),
                        (Algorithm::Cdcl, "CDCL", true),
                    ],
                    current: cur_algo,
                    on_pick: move |a| algo.set(a),
                }
                button {
                    class: "px-3 py-1.5 rounded-lg bg-surface-2 text-ink-2 text-sm border border-border-soft hover:bg-surface",
                    onclick: move |_| dark.set(!dark()),
                    if dark() { "☀ Light" } else { "🌓 Dark" }
                }
            }
        }
    }
}

#[component]
fn Segmented<T: 'static + Copy + PartialEq>(
    items: Vec<(T, &'static str, bool)>,
    current: T,
    on_pick: EventHandler<T>,
) -> Element {
    rsx! {
        div { class: "flex gap-1 bg-surface-2 p-[3px] rounded-[10px]",
            for (val, label, enabled) in items {
                button {
                    class: if val == current {
                        "px-3 py-1 rounded-lg text-sm font-medium bg-surface text-ink shadow-sm"
                    } else if enabled {
                        "px-3 py-1 rounded-lg text-sm text-ink-muted hover:text-ink"
                    } else {
                        "px-3 py-1 rounded-lg text-sm text-ink-faint cursor-not-allowed opacity-50"
                    },
                    disabled: !enabled,
                    onclick: move |_| if enabled { on_pick.call(val) },
                    "{label}"
                }
            }
        }
    }
}
