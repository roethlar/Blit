//! Elm-ish message/update skeleton. The model is the single source
//! of truth for the console's browse state; `update` is pure-ish —
//! it mutates the model and returns [`Effect`]s for the host (a
//! future egui/ratatui face, or a test) to execute. Executing an
//! [`Effect::Browse`] means calling [`crate::browse::browse`] and
//! feeding the result back as [`Msg::ListingLoaded`] /
//! [`Msg::ListingFailed`], so both faces drive identical logic.
//! Every browse carries a generation the host echoes back; the model
//! drops completions for superseded requests, so asynchronous hosts
//! can never show a stale directory or error.

use crate::endpoint::{Endpoint, EndpointId};
use blit_app::admin::ls::DirEntry;
use std::path::PathBuf;

/// Everything a face can tell the core.
#[derive(Debug, Clone)]
pub enum Msg {
    /// Bring an endpoint's browse pane into focus.
    SelectEndpoint(EndpointId),
    /// Ask to list `path` on the selected endpoint.
    NavigateTo(PathBuf),
    /// A browse effect completed successfully. `generation` must echo
    /// the [`Effect::Browse`] it answers; stale completions are dropped.
    ListingLoaded {
        generation: u64,
        path: PathBuf,
        entries: Vec<DirEntry>,
    },
    /// A browse effect failed. Same generation rule as
    /// [`Msg::ListingLoaded`].
    ListingFailed {
        generation: u64,
        path: PathBuf,
        error: String,
    },
}

/// Work the host must perform on the core's behalf.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Effect {
    /// List `path` on `endpoint` and report back with
    /// [`Msg::ListingLoaded`] / [`Msg::ListingFailed`], echoing
    /// `generation` so the core can drop out-of-order completions.
    Browse {
        endpoint: EndpointId,
        path: PathBuf,
        generation: u64,
    },
}

/// Console state: registered endpoints plus the current browse.
#[derive(Debug)]
pub struct Model {
    endpoints: Vec<(EndpointId, Endpoint)>,
    selected: Option<EndpointId>,
    current_path: PathBuf,
    listing: Vec<DirEntry>,
    loading: bool,
    last_error: Option<String>,
    next_id: u64,
    /// Generation of the most recently issued [`Effect::Browse`].
    /// Completions echoing an older generation are dropped.
    browse_generation: u64,
}

impl Default for Model {
    fn default() -> Self {
        Self::new()
    }
}

impl Model {
    /// A fresh model with the local filesystem registered (and
    /// selected) as endpoint 0 — a console that cannot browse the
    /// machine it runs on is useless.
    pub fn new() -> Self {
        let mut model = Self {
            endpoints: Vec::new(),
            selected: None,
            current_path: PathBuf::from("/"),
            listing: Vec::new(),
            loading: false,
            last_error: None,
            next_id: 0,
            browse_generation: 0,
        };
        let local = model.add_endpoint(Endpoint::Local);
        model.selected = Some(local);
        model
    }

    /// Register an endpoint and return its assigned id.
    pub fn add_endpoint(&mut self, endpoint: Endpoint) -> EndpointId {
        let id = EndpointId(self.next_id);
        self.next_id += 1;
        self.endpoints.push((id, endpoint));
        id
    }

    /// All registered endpoints, in registration order.
    pub fn endpoints(&self) -> &[(EndpointId, Endpoint)] {
        &self.endpoints
    }

    pub fn endpoint(&self, id: EndpointId) -> Option<&Endpoint> {
        self.endpoints
            .iter()
            .find(|(candidate, _)| *candidate == id)
            .map(|(_, endpoint)| endpoint)
    }

    pub fn selected(&self) -> Option<EndpointId> {
        self.selected
    }

    pub fn current_path(&self) -> &PathBuf {
        &self.current_path
    }

    pub fn listing(&self) -> &[DirEntry] {
        &self.listing
    }

    pub fn is_loading(&self) -> bool {
        self.loading
    }

    pub fn last_error(&self) -> Option<&str> {
        self.last_error.as_deref()
    }
}

/// Apply one message to the model, returning effects for the host.
pub fn update(model: &mut Model, msg: Msg) -> Vec<Effect> {
    match msg {
        Msg::SelectEndpoint(id) => {
            if model.endpoint(id).is_some() {
                // A switch owns the pane: drop the previous endpoint's
                // listing/path and immediately browse the new endpoint's
                // root. Issuing the browse bumps the generation, so any
                // completion still in flight from the old endpoint is
                // dropped as stale.
                model.selected = Some(id);
                model.last_error = None;
                model.listing.clear();
                model.current_path = PathBuf::from("/");
                model.browse_generation += 1;
                let generation = model.browse_generation;
                model.loading = true;
                vec![Effect::Browse {
                    endpoint: id,
                    path: model.current_path.clone(),
                    generation,
                }]
            } else {
                model.last_error = Some(format!("unknown endpoint {}", id.0));
                Vec::new()
            }
        }
        Msg::NavigateTo(path) => match model.selected {
            Some(endpoint) => {
                model.browse_generation += 1;
                let generation = model.browse_generation;
                model.loading = true;
                model.current_path = path.clone();
                vec![Effect::Browse {
                    endpoint,
                    path,
                    generation,
                }]
            }
            None => {
                model.last_error = Some("no endpoint selected".to_string());
                Vec::new()
            }
        },
        Msg::ListingLoaded {
            generation,
            path,
            entries,
        } => {
            if !model.loading || generation != model.browse_generation {
                // Stale or unsolicited completion: a newer request (or
                // none) owns the pane — drop it.
                return Vec::new();
            }
            model.loading = false;
            model.current_path = path;
            model.listing = entries;
            model.last_error = None;
            Vec::new()
        }
        Msg::ListingFailed {
            generation, error, ..
        } => {
            if !model.loading || generation != model.browse_generation {
                return Vec::new();
            }
            model.loading = false;
            model.listing.clear();
            model.last_error = Some(error);
            Vec::new()
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::endpoint::DaemonEndpoint;

    #[test]
    fn new_model_has_local_endpoint_selected() {
        let model = Model::new();
        assert_eq!(model.endpoints().len(), 1);
        let selected = model.selected().unwrap();
        assert_eq!(selected, EndpointId(0));
        assert!(matches!(model.endpoint(selected), Some(Endpoint::Local)));
    }

    #[test]
    fn select_unknown_endpoint_sets_error() {
        let mut model = Model::new();
        let effects = update(&mut model, Msg::SelectEndpoint(EndpointId(99)));
        assert!(effects.is_empty());
        assert_eq!(model.selected(), Some(EndpointId(0)));
        assert_eq!(model.last_error(), Some("unknown endpoint 99"));
    }

    #[test]
    fn select_registered_daemon_clears_error() {
        let mut model = Model::new();
        let daemon = model.add_endpoint(Endpoint::Daemon(DaemonEndpoint {
            address: "magneto:9833".to_string(),
            name: "magneto".to_string(),
        }));
        update(&mut model, Msg::SelectEndpoint(EndpointId(99)));
        let effects = update(&mut model, Msg::SelectEndpoint(daemon));
        assert_eq!(
            effects,
            vec![Effect::Browse {
                endpoint: daemon,
                path: PathBuf::from("/"),
                generation: 1,
            }]
        );
        assert_eq!(model.selected(), Some(daemon));
        assert_eq!(model.last_error(), None);
    }

    #[test]
    fn select_resets_pane_and_replaces_in_flight_browse() {
        let mut model = Model::new();
        let daemon = model.add_endpoint(Endpoint::Daemon(DaemonEndpoint {
            address: "magneto:9833".to_string(),
            name: "magneto".to_string(),
        }));
        // Load the local pane, then start another browse so one is in
        // flight when the switch lands.
        update(&mut model, Msg::NavigateTo(PathBuf::from("/tmp")));
        update(
            &mut model,
            Msg::ListingLoaded {
                generation: 1,
                path: PathBuf::from("/tmp"),
                entries: vec![DirEntry {
                    name: "local-file".to_string(),
                    is_dir: false,
                    size: 1,
                    mtime_seconds: 1,
                }],
            },
        );
        update(&mut model, Msg::NavigateTo(PathBuf::from("/var")));
        assert!(model.is_loading());
        let effects = update(&mut model, Msg::SelectEndpoint(daemon));
        // The pane is reset and re-loading the new endpoint's root; the
        // old endpoint's in-flight browse (generation 2) is superseded.
        assert_eq!(
            effects,
            vec![Effect::Browse {
                endpoint: daemon,
                path: PathBuf::from("/"),
                generation: 3,
            }]
        );
        assert!(model.listing().is_empty());
        assert_eq!(model.current_path(), &PathBuf::from("/"));
        assert!(model.is_loading());
        // The superseded local completion is dropped, not displayed.
        update(
            &mut model,
            Msg::ListingLoaded {
                generation: 2,
                path: PathBuf::from("/var"),
                entries: vec![DirEntry {
                    name: "stale".to_string(),
                    is_dir: true,
                    size: 0,
                    mtime_seconds: 1,
                }],
            },
        );
        assert!(model.is_loading());
        assert!(model.listing().is_empty());
        assert_eq!(model.current_path(), &PathBuf::from("/"));
        // The switch's own completion resolves the pane.
        update(
            &mut model,
            Msg::ListingFailed {
                generation: 3,
                path: PathBuf::from("/"),
                error: "daemon browse unsupported".to_string(),
            },
        );
        assert!(!model.is_loading());
        assert_eq!(model.last_error(), Some("daemon browse unsupported"));
    }

    #[test]
    fn navigate_emits_browse_effect_and_loads() {
        let mut model = Model::new();
        let effects = update(&mut model, Msg::NavigateTo(PathBuf::from("/tmp")));
        assert_eq!(
            effects,
            vec![Effect::Browse {
                endpoint: EndpointId(0),
                path: PathBuf::from("/tmp"),
                generation: 1,
            }]
        );
        assert!(model.is_loading());
        assert_eq!(model.current_path(), &PathBuf::from("/tmp"));
    }

    #[test]
    fn listing_loaded_replaces_state() {
        let mut model = Model::new();
        update(&mut model, Msg::NavigateTo(PathBuf::from("/tmp")));
        let entries = vec![DirEntry {
            name: "a".to_string(),
            is_dir: true,
            size: 0,
            mtime_seconds: 1,
        }];
        let effects = update(
            &mut model,
            Msg::ListingLoaded {
                generation: 1,
                path: PathBuf::from("/tmp"),
                entries,
            },
        );
        assert!(effects.is_empty());
        assert!(!model.is_loading());
        assert_eq!(model.listing().len(), 1);
        assert_eq!(model.listing()[0].name, "a");
        assert_eq!(model.last_error(), None);
    }

    #[test]
    fn listing_failed_clears_listing_and_records_error() {
        let mut model = Model::new();
        update(&mut model, Msg::NavigateTo(PathBuf::from("/tmp")));
        let effects = update(
            &mut model,
            Msg::ListingFailed {
                generation: 1,
                path: PathBuf::from("/tmp"),
                error: "permission denied".to_string(),
            },
        );
        assert!(effects.is_empty());
        assert!(!model.is_loading());
        assert!(model.listing().is_empty());
        assert_eq!(model.last_error(), Some("permission denied"));
    }

    #[test]
    fn stale_loaded_is_dropped_in_favour_of_newer_request() {
        let mut model = Model::new();
        update(&mut model, Msg::NavigateTo(PathBuf::from("/a")));
        update(&mut model, Msg::NavigateTo(PathBuf::from("/b")));
        // The /a browse (generation 1) completes after /b (generation 2)
        // was issued — it must not overwrite the pane.
        let effects = update(
            &mut model,
            Msg::ListingLoaded {
                generation: 1,
                path: PathBuf::from("/a"),
                entries: vec![DirEntry {
                    name: "stale".to_string(),
                    is_dir: true,
                    size: 0,
                    mtime_seconds: 1,
                }],
            },
        );
        assert!(effects.is_empty());
        assert!(model.is_loading());
        assert_eq!(model.current_path(), &PathBuf::from("/b"));
        assert!(model.listing().is_empty());
        // The matching completion for generation 2 still lands.
        update(
            &mut model,
            Msg::ListingLoaded {
                generation: 2,
                path: PathBuf::from("/b"),
                entries: vec![DirEntry {
                    name: "fresh".to_string(),
                    is_dir: true,
                    size: 0,
                    mtime_seconds: 1,
                }],
            },
        );
        assert!(!model.is_loading());
        assert_eq!(model.listing().len(), 1);
        assert_eq!(model.listing()[0].name, "fresh");
    }

    #[test]
    fn stale_failure_is_dropped_in_favour_of_newer_request() {
        let mut model = Model::new();
        update(&mut model, Msg::NavigateTo(PathBuf::from("/a")));
        update(&mut model, Msg::NavigateTo(PathBuf::from("/b")));
        let effects = update(
            &mut model,
            Msg::ListingFailed {
                generation: 1,
                path: PathBuf::from("/a"),
                error: "stale error".to_string(),
            },
        );
        assert!(effects.is_empty());
        assert!(model.is_loading());
        assert_eq!(model.current_path(), &PathBuf::from("/b"));
        assert_eq!(model.last_error(), None);
    }

    #[test]
    fn unsolicited_completion_without_browse_is_dropped() {
        let mut model = Model::new();
        let effects = update(
            &mut model,
            Msg::ListingLoaded {
                generation: 0,
                path: PathBuf::from("/tmp"),
                entries: vec![],
            },
        );
        assert!(effects.is_empty());
        assert!(!model.is_loading());
        assert!(model.listing().is_empty());
    }

    #[test]
    fn navigate_without_selection_is_an_error() {
        let mut model = Model {
            endpoints: Vec::new(),
            selected: None,
            current_path: PathBuf::from("/"),
            listing: Vec::new(),
            loading: false,
            last_error: None,
            next_id: 0,
            browse_generation: 0,
        };
        let effects = update(&mut model, Msg::NavigateTo(PathBuf::from("/tmp")));
        assert!(effects.is_empty());
        assert_eq!(model.last_error(), Some("no endpoint selected"));
    }
}
