use crate::algos::dp::generate::Generate;
use crate::algos::dp::practice::Practice;
use crate::algos::dp::state::{DpAppState, Module};
use dioxus::prelude::*;

#[component]
pub fn DpView() -> Element {
    let state = use_context::<Signal<DpAppState>>();
    let module = state.read().module;
    rsx! {
        match module {
            Module::Generate => rsx! { Generate {} },
            Module::Practice => rsx! { Practice {} },
        }
    }
}
