mod algos;
mod layout;

use algos::cdcl::state::{CdclAppState, CdclGenerateState, CdclPracticeState};
use algos::cdcl::view::CdclView;
use algos::dp::state::{DpAppState, GenerateState, PracticeState};
use algos::dp::view::DpView;
use algos::dpll::state::{DpllAppState, DpllGenerateState, DpllPracticeState};
use algos::dpll::view::DpllView;
use layout::footer::Footer;
use layout::topbar::Topbar;

use dioxus::prelude::*;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Algorithm {
    #[default]
    DavisPutnam,
    Dpll,
    Cdcl,
}

fn main() {
    // `desktop!` collapses to `()` on every other platform, and `with_cfg`
    // ignores a config whose type the active renderer does not know.
    dioxus::LaunchBuilder::new()
        .with_cfg(desktop! {
            use dioxus::desktop::{Config, LogicalSize, WindowBuilder, icon_from_memory};

            Config::new()
                .with_window(
                    WindowBuilder::new()
                        .with_title("Klausel")
                        .with_inner_size(LogicalSize::new(1280.0, 860.0)),
                )
                .with_icon(
                    icon_from_memory(include_bytes!("../icons/256x256.png"))
                        .expect("bundled app icon is a valid PNG"),
                )
        })
        .launch(App);
}

#[component]
fn App() -> Element {
    let dark = use_signal(|| false);
    let algo = use_signal(Algorithm::default);
    let dp_app = use_signal(DpAppState::default);
    let dp_generate = use_signal(GenerateState::default);
    let dp_solve = use_signal(PracticeState::default);
    let dpll_generate = use_signal(DpllGenerateState::default);
    let dpll_app = use_signal(DpllAppState::default);
    let dpll_solve = use_signal(DpllPracticeState::default);
    use_context_provider(|| dp_app);
    use_context_provider(|| dp_generate);
    use_context_provider(|| dp_solve);
    use_context_provider(|| dpll_generate);
    use_context_provider(|| dpll_app);
    use_context_provider(|| dpll_solve);
    let cdcl_generate = use_signal(CdclGenerateState::default);
    let cdcl_app = use_signal(CdclAppState::default);
    let cdcl_solve = use_signal(CdclPracticeState::default);
    use_context_provider(|| cdcl_generate);
    use_context_provider(|| cdcl_app);
    use_context_provider(|| cdcl_solve);

    use_effect(move || {
        let cls = if dark() { "dark" } else { "" };
        let _ = document::eval(&format!("document.documentElement.className = '{cls}';"));
    });

    rsx! {
        document::Stylesheet { href: asset!("./assets/tailwind.css") }
        document::Link { rel: "icon", r#type: "image/svg+xml", href: asset!("/icons/icon.svg") }
        document::Title {"Klausel"}

        div { class: "min-h-screen bg-page-bg text-ink font-sans flex flex-col",
            Topbar { dark, algo }

            main { class: "flex-1 flex flex-col",
                match algo() {
                    Algorithm::DavisPutnam => rsx! { DpView {} },
                    Algorithm::Dpll => rsx! { DpllView {} },
                    Algorithm::Cdcl => rsx! { CdclView {} },
                }
            }

            Footer {}
        }
    }
}
