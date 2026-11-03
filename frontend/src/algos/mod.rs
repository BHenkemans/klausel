pub mod cdcl;
pub mod controls;
pub mod dp;
pub mod dpll;
pub mod feedback;
pub mod lit;
pub mod math;
pub mod model_text;
pub mod paste_box;
#[cfg(feature = "server")]
pub mod trace_generator;
pub mod trace_text;

use serde::{Deserialize, Serialize};

/// The verdict an exercise is generated for. Shared by both algorithms: it is
/// the target of the final-state constraint, not a property of any one rule set.
#[derive(Clone, Copy, PartialEq, Eq, Debug, Serialize, Deserialize)]
pub enum Sat {
    Sat,
    Unsat,
}

impl Sat {
    pub fn label(self) -> &'static str {
        match self {
            Sat::Sat => "SAT",
            Sat::Unsat => "UNSAT",
        }
    }
}

/// Which submodule of an algorithm is on screen. Both algorithms have the
/// same Generate/Practice split, so the enum is shared rather than duplicated.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Module {
    Generate,
    Practice,
}

/// A random exercise is designed from one valid run, but the student follows
/// the priority order, which can finish far sooner. The exercise is handed out
/// once that run lands within this many steps of the length asked for.
#[cfg(feature = "server")]
pub fn step_slack(wanted: usize) -> usize {
    wanted / 4
}

/// Run CPU-bound solver work off the async executor.
///
/// Both builds need this for different reasons: on the server it keeps a solve
/// from occupying an axum worker, and on desktop the executor polling the
/// `#[server]` body is the one driving the window, so blocking it stalls the UI.
#[cfg(feature = "server")]
pub async fn blocking<T, F>(f: F) -> Result<T, dioxus::prelude::ServerFnError>
where
    F: FnOnce() -> T + Send + 'static,
    T: Send + 'static,
{
    tokio::task::spawn_blocking(f)
        .await
        .map_err(|e| dioxus::prelude::ServerFnError::new(format!("solver task failed: {e}")))
}
