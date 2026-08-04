//! Elm-ish message/update skeleton. The model is the single source
//! of truth for the console's browse state; `update` is pure-ish —
//! it mutates the model and returns [`Effect`]s for the host (a
//! future egui/ratatui face, or a test) to execute. Executing an
//! [`Effect::Browse`] means calling [`crate::browse::browse`] and
//! feeding the result back as [`Msg::ListingLoaded`] /
//! [`Msg::ListingFailed`], so both faces drive identical logic.

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
    /// A browse effect completed successfully.
    ListingLoaded {
        path: PathBuf,
        entries: Vec<DirEntry>,
    },
    /// A browse effect failed.
    ListingFailed { path: PathBuf, error: String },
}

/// Work the host must perform on the core's behalf.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Effect {
    /// List `path` on `endpoint` and report back with
    /// [`Msg::ListingLoaded`] / [`Msg::ListingFailed`].
    Browse { endpoint: EndpointId, path: PathBuf },
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
                model.selected = Some(id);
                model.last_error = None;
            } else {
                model.last_error = Some(format!("unknown endpoint {}", id.0));
            }
            Vec::new()
        }
        Msg::NavigateTo(path) => match model.selected {
            Some(endpoint) => {
                model.loading = true;
                model.current_path = path.clone();
                vec![Effect::Browse { endpoint, path }]
            }
            None => {
                model.last_error = Some("no endpoint selected".to_string());
                Vec::new()
            }
        },
        Msg::ListingLoaded { path, entries } => {
            model.loading = false;
            model.current_path = path;
            model.listing = entries;
            model.last_error = None;
            Vec::new()
        }
        Msg::ListingFailed { error, .. } => {
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
        assert!(effects.is_empty());
        assert_eq!(model.selected(), Some(daemon));
        assert_eq!(model.last_error(), None);
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
    fn navigate_without_selection_is_an_error() {
        let mut model = Model {
            endpoints: Vec::new(),
            selected: None,
            current_path: PathBuf::from("/"),
            listing: Vec::new(),
            loading: false,
            last_error: None,
            next_id: 0,
        };
        let effects = update(&mut model, Msg::NavigateTo(PathBuf::from("/tmp")));
        assert!(effects.is_empty());
        assert_eq!(model.last_error(), Some("no endpoint selected"));
    }
}
