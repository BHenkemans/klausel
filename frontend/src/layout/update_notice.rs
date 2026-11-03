use dioxus::prelude::*;

const LATEST_RELEASE_PAGE: &str = "https://github.com/BHenkemans/klausel/releases/latest";
const LATEST_RELEASE_API: &str = "https://api.github.com/repos/BHenkemans/klausel/releases/latest";

#[component]
pub fn UpdateNotice() -> Element {
    let Some(built_from) = option_env!("KLAUSEL_VERSION").filter(|_| cfg!(feature = "desktop")) else {
        return rsx! {};
    };

    let latest_tag = use_resource(|| async {
        document::eval(&format!(
            "const response = await fetch('{LATEST_RELEASE_API}');
             return (await response.json()).tag_name;"
        ))
        .join::<String>()
        .await
        .ok()
    });

    match latest_tag() {
        Some(Some(tag)) if version(&tag) > version(built_from) => rsx! {
            a {
                href: LATEST_RELEASE_PAGE,
                class: "px-3 py-1.5 rounded-lg bg-warn-bg text-warn-ink text-sm font-medium hover:underline",
                "Update available: {tag}"
            }
        },
        _ => rsx! {},
    }
}

fn version(described: &str) -> Vec<u32> {
    let tag = described.split('-').next().unwrap_or_default();
    tag.trim_start_matches('v')
        .split('.')
        .map(|part| part.parse().unwrap_or(0))
        .collect()
}
