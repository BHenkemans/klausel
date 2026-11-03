//! Bulk trace entry, shared by both generators.
//!
//! Designing a real exercise a chip at a time is the slow path — a textbook
//! derivation runs to a dozen steps. Both algorithms already have a text form
//! for traces, so pasting one is the fast path, and `apply` is where each
//! algorithm decides what that text implies for its bounds.

use dioxus::prelude::*;

#[component]
pub fn TracePasteBox(
    /// Parse and install the text, returning how many steps were loaded.
    apply: Callback<String, Result<usize, String>>,
    placeholder: &'static str,
    hint: &'static str,
) -> Element {
    let mut text = use_signal(String::new);
    let mut status = use_signal(|| None::<Result<String, String>>);

    rsx! {
        details { class: "mt-4",
            summary { class: "text-[12px] font-semibold text-ink-muted cursor-pointer hover:text-ink list-none flex items-center gap-1.5",
                span { class: "text-[10px]", "▸" }
                "Paste a trace"
            }
            div { class: "mt-2.5",
                textarea {
                    class: "w-full h-28 px-2.5 py-2 rounded-[11px] border border-border bg-surface font-mono text-[11.5px] text-ink leading-[1.5] resize-y",
                    placeholder,
                    spellcheck: false,
                    value: "{text}",
                    oninput: move |e| {
                        text.set(e.value());
                        status.set(None);
                    },
                }
                div { class: "flex items-center gap-2 mt-1.5",
                    button {
                        class: "px-3 py-1.5 rounded-lg bg-surface-2 text-ink-2 text-[12px] font-bold cursor-pointer hover:bg-surface disabled:opacity-50 disabled:cursor-not-allowed",
                        disabled: text.read().trim().is_empty(),
                        onclick: move |_| {
                            let outcome = apply.call(text.peek().clone());
                            status
                                .set(
                                    Some(
                                        match outcome {
                                            Ok(n) => Ok(format!("Loaded {n} steps")),
                                            Err(e) => Err(e),
                                        },
                                    ),
                                );
                        },
                        "Apply"
                    }
                    match &*status.read() {
                        Some(Ok(msg)) => rsx! {
                            span { class: "text-[11.5px] font-semibold text-accent", "{msg}" }
                        },
                        Some(Err(msg)) => rsx! {
                            span { class: "text-[11.5px] text-err-ink leading-[1.4]", "{msg}" }
                        },
                        None => rsx! {
                            span { class: "text-[11.5px] text-ink-faint", "{hint}" }
                        },
                    }
                }
            }
        }
    }
}
