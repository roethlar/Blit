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
//!
//! Discovery works the same way: [`Msg::RefreshDiscovery`] yields
//! [`Effect::Discover`], the host runs [`crate::discover::discover_daemons`]
//! and answers with [`Msg::DiscoveryLoaded`] / [`Msg::DiscoveryFailed`],
//! again generation-tagged so a slow scan can never overwrite a newer
//! one. Merging is **upsert-by-address**: an already-known daemon keeps
//! its [`EndpointId`] (and any selection pointing at it) while its
//! display name refreshes, and daemons absent from the new snapshot are
//! removed — an mDNS scan is authoritative for what is on the LAN right
//! now. The one staleness hazard this creates is endpoints disappearing
//! under the selection: if the selected daemon vanishes, the model
//! falls back to the Local endpoint with a fresh root browse (the
//! in-flight browse of the dead daemon is superseded by generation), so
//! the pane never shows an endpoint that left the network.

use crate::discover::DEFAULT_DISCOVERY_TIMEOUT;
use crate::endpoint::{DaemonEndpoint, Endpoint, EndpointId};
use blit_core::admin::ls::DirEntry;
use std::collections::HashSet;
use std::path::PathBuf;
use std::time::Duration;

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
    /// Ask for a fresh mDNS scan of the LAN.
    RefreshDiscovery,
    /// A discovery effect completed successfully. `generation` must
    /// echo the [`Effect::Discover`] it answers; stale completions are
    /// dropped. `endpoints` is the whole snapshot — the merge into the
    /// registered endpoints is upsert-by-address (module doc above).
    DiscoveryLoaded {
        generation: u64,
        endpoints: Vec<DaemonEndpoint>,
    },
    /// A discovery effect failed. Same generation rule as
    /// [`Msg::DiscoveryLoaded`]. The previously registered endpoints
    /// are kept; only the error surface changes.
    DiscoveryFailed { generation: u64, error: String },
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
    /// Scan the LAN for daemons (the host runs
    /// [`crate::discover::discover_daemons`] with `timeout`) and report
    /// back with [`Msg::DiscoveryLoaded`] / [`Msg::DiscoveryFailed`],
    /// echoing `generation` so the core can drop out-of-order scans.
    Discover { timeout: Duration, generation: u64 },
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
    /// Whether a discovery scan is in flight.
    discovering: bool,
    /// Generation of the most recently issued [`Effect::Discover`].
    discovery_generation: u64,
    /// Provenance for the discovery merge: which registered endpoints
    /// came from mDNS (and may therefore be removed by a snapshot that
    /// no longer lists them). Manually registered endpoints are never
    /// in this set and survive every scan.
    discovered: HashSet<EndpointId>,
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
            discovering: false,
            discovery_generation: 0,
            discovered: HashSet::new(),
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

    pub fn is_discovering(&self) -> bool {
        self.discovering
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
        Msg::RefreshDiscovery => {
            model.discovery_generation += 1;
            let generation = model.discovery_generation;
            model.discovering = true;
            vec![Effect::Discover {
                timeout: DEFAULT_DISCOVERY_TIMEOUT,
                generation,
            }]
        }
        Msg::DiscoveryLoaded {
            generation,
            endpoints,
        } => {
            if !model.discovering || generation != model.discovery_generation {
                // Stale or unsolicited scan: a newer one owns the
                // endpoint set — drop it.
                return Vec::new();
            }
            model.discovering = false;

            // Upsert by address: an already-known daemon keeps its id
            // (and any selection pointing at it); only the display
            // name refreshes. New daemons are appended in the
            // snapshot's (name-sorted) order.
            let fresh: HashSet<String> = endpoints
                .iter()
                .map(|daemon| daemon.address.clone())
                .collect();
            for daemon in endpoints {
                let existing = model.endpoints.iter_mut().find(|(_, endpoint)| {
                    matches!(endpoint, Endpoint::Daemon(known) if known.address == daemon.address)
                });
                match existing {
                    Some((id, Endpoint::Daemon(known))) => {
                        known.name = daemon.name;
                        model.discovered.insert(*id);
                    }
                    // `find` above only matches Daemon variants.
                    Some(_) => unreachable!("address match implies a daemon endpoint"),
                    None => {
                        let id = model.add_endpoint(Endpoint::Daemon(daemon));
                        model.discovered.insert(id);
                    }
                }
            }

            // Departures: previously discovered daemons missing from
            // this snapshot left the LAN — remove them.
            let mut removed_selected = false;
            let previously_discovered = std::mem::take(&mut model.discovered);
            model.endpoints.retain(|(id, endpoint)| {
                let keep = !previously_discovered.contains(id)
                    || matches!(endpoint, Endpoint::Daemon(daemon) if fresh.contains(&daemon.address));
                if !keep && model.selected == Some(*id) {
                    removed_selected = true;
                }
                keep
            });
            model.discovered = previously_discovered
                .into_iter()
                .filter(|id| model.endpoints.iter().any(|(kept, _)| kept == id))
                .collect();

            if !removed_selected {
                return Vec::new();
            }
            // The endpoint the operator was browsing left the LAN:
            // fall back to Local with a fresh root browse (same reset
            // SelectEndpoint performs) so the pane never shows a dead
            // endpoint. Issuing the browse bumps the browse generation,
            // so the dead daemon's in-flight completion is dropped.
            let local = model
                .endpoints
                .iter()
                .find(|(_, endpoint)| matches!(endpoint, Endpoint::Local))
                .map(|(id, _)| *id);
            match local {
                Some(local_id) => {
                    model.selected = Some(local_id);
                    model.last_error = None;
                    model.listing.clear();
                    model.current_path = PathBuf::from("/");
                    model.browse_generation += 1;
                    let generation = model.browse_generation;
                    model.loading = true;
                    vec![Effect::Browse {
                        endpoint: local_id,
                        path: model.current_path.clone(),
                        generation,
                    }]
                }
                None => {
                    model.selected = None;
                    model.listing.clear();
                    model.loading = false;
                    // No replacement browse is issued, so supersede any
                    // in-flight one explicitly.
                    model.browse_generation += 1;
                    Vec::new()
                }
            }
        }
        Msg::DiscoveryFailed { generation, error } => {
            if !model.discovering || generation != model.discovery_generation {
                return Vec::new();
            }
            model.discovering = false;
            // The registered endpoints are kept as-is (the previous
            // snapshot may still be right); the console has one error
            // surface in this slice, so the failure lands there.
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
            discovering: false,
            discovery_generation: 0,
            discovered: HashSet::new(),
        };
        let effects = update(&mut model, Msg::NavigateTo(PathBuf::from("/tmp")));
        assert!(effects.is_empty());
        assert_eq!(model.last_error(), Some("no endpoint selected"));
    }

    fn daemon_endpoint(name: &str, address: &str) -> DaemonEndpoint {
        DaemonEndpoint {
            address: address.to_string(),
            name: name.to_string(),
        }
    }

    #[test]
    fn refresh_discovery_emits_effect_and_marks_scan_in_flight() {
        let mut model = Model::new();
        let effects = update(&mut model, Msg::RefreshDiscovery);
        assert_eq!(
            effects,
            vec![Effect::Discover {
                timeout: DEFAULT_DISCOVERY_TIMEOUT,
                generation: 1,
            }]
        );
        assert!(model.is_discovering());
    }

    #[test]
    fn discovery_loaded_registers_new_daemons() {
        let mut model = Model::new();
        update(&mut model, Msg::RefreshDiscovery);
        let effects = update(
            &mut model,
            Msg::DiscoveryLoaded {
                generation: 1,
                endpoints: vec![
                    daemon_endpoint("alpha", "192.168.1.10:9031"),
                    daemon_endpoint("bravo", "192.168.1.20:9031"),
                ],
            },
        );
        assert!(effects.is_empty());
        assert!(!model.is_discovering());
        // Local + two daemons, in snapshot order.
        assert_eq!(model.endpoints().len(), 3);
        assert_eq!(model.endpoints()[1].1.display_name(), "alpha");
        assert_eq!(model.endpoints()[2].1.display_name(), "bravo");
        // Selection is untouched.
        assert_eq!(model.selected(), Some(EndpointId(0)));
    }

    #[test]
    fn repeated_discovery_upserts_by_address_without_duplicates() {
        let mut model = Model::new();
        update(&mut model, Msg::RefreshDiscovery);
        update(
            &mut model,
            Msg::DiscoveryLoaded {
                generation: 1,
                endpoints: vec![daemon_endpoint("alpha", "192.168.1.10:9031")],
            },
        );
        let alpha_id = model.endpoints()[1].0;
        // Second scan: same address, new display name.
        update(&mut model, Msg::RefreshDiscovery);
        update(
            &mut model,
            Msg::DiscoveryLoaded {
                generation: 2,
                endpoints: vec![daemon_endpoint("alpha-renamed", "192.168.1.10:9031")],
            },
        );
        assert_eq!(model.endpoints().len(), 2, "upsert must not duplicate");
        assert_eq!(
            model.endpoints()[1].0,
            alpha_id,
            "id is stable across scans"
        );
        assert_eq!(model.endpoints()[1].1.display_name(), "alpha-renamed");
    }

    #[test]
    fn discovery_removes_departed_daemons_but_keeps_manual_and_local() {
        let mut model = Model::new();
        let manual = model.add_endpoint(Endpoint::Daemon(daemon_endpoint(
            "manual",
            "10.0.0.99:9031",
        )));
        update(&mut model, Msg::RefreshDiscovery);
        update(
            &mut model,
            Msg::DiscoveryLoaded {
                generation: 1,
                endpoints: vec![daemon_endpoint("alpha", "192.168.1.10:9031")],
            },
        );
        assert_eq!(model.endpoints().len(), 3);
        // Second scan: alpha is gone from the LAN.
        update(&mut model, Msg::RefreshDiscovery);
        update(
            &mut model,
            Msg::DiscoveryLoaded {
                generation: 2,
                endpoints: vec![],
            },
        );
        let names: Vec<_> = model
            .endpoints()
            .iter()
            .map(|(_, endpoint)| endpoint.display_name().to_string())
            .collect();
        assert_eq!(names, vec!["Local", "manual"]);
        assert!(model.endpoint(manual).is_some());
    }

    #[test]
    fn vanished_selected_daemon_falls_back_to_local_root_browse() {
        let mut model = Model::new();
        update(&mut model, Msg::RefreshDiscovery);
        update(
            &mut model,
            Msg::DiscoveryLoaded {
                generation: 1,
                endpoints: vec![daemon_endpoint("alpha", "192.168.1.10:9031")],
            },
        );
        let alpha = model.endpoints()[1].0;
        update(&mut model, Msg::SelectEndpoint(alpha));
        assert!(model.is_loading());
        // Alpha departs while its root browse is in flight.
        update(&mut model, Msg::RefreshDiscovery);
        let effects = update(
            &mut model,
            Msg::DiscoveryLoaded {
                generation: 2,
                endpoints: vec![],
            },
        );
        assert_eq!(
            effects,
            vec![Effect::Browse {
                endpoint: EndpointId(0),
                path: PathBuf::from("/"),
                generation: 2,
            }]
        );
        assert_eq!(model.selected(), Some(EndpointId(0)));
        assert_eq!(model.current_path(), &PathBuf::from("/"));
        assert!(model.listing().is_empty());
        assert!(model.is_loading());
        // The dead daemon's in-flight completion is superseded.
        update(
            &mut model,
            Msg::ListingLoaded {
                generation: 1,
                path: PathBuf::from("/"),
                entries: vec![DirEntry {
                    name: "stale".to_string(),
                    is_dir: true,
                    size: 0,
                    mtime_seconds: 1,
                }],
            },
        );
        assert!(model.listing().is_empty());
        assert!(model.is_loading());
    }

    #[test]
    fn departed_unselected_daemon_issues_no_browse() {
        let mut model = Model::new();
        update(&mut model, Msg::RefreshDiscovery);
        update(
            &mut model,
            Msg::DiscoveryLoaded {
                generation: 1,
                endpoints: vec![daemon_endpoint("alpha", "192.168.1.10:9031")],
            },
        );
        update(&mut model, Msg::RefreshDiscovery);
        let effects = update(
            &mut model,
            Msg::DiscoveryLoaded {
                generation: 2,
                endpoints: vec![],
            },
        );
        assert!(effects.is_empty());
        assert_eq!(model.selected(), Some(EndpointId(0)));
        assert!(!model.is_loading());
    }

    #[test]
    fn stale_discovery_result_is_dropped() {
        let mut model = Model::new();
        update(&mut model, Msg::RefreshDiscovery);
        update(&mut model, Msg::RefreshDiscovery);
        // Scan 1's result arrives after scan 2 was issued.
        let effects = update(
            &mut model,
            Msg::DiscoveryLoaded {
                generation: 1,
                endpoints: vec![daemon_endpoint("stale", "192.168.1.10:9031")],
            },
        );
        assert!(effects.is_empty());
        assert!(model.is_discovering());
        assert_eq!(model.endpoints().len(), 1, "nothing registered");
        // Scan 2's own result still lands.
        update(
            &mut model,
            Msg::DiscoveryLoaded {
                generation: 2,
                endpoints: vec![daemon_endpoint("fresh", "192.168.1.20:9031")],
            },
        );
        assert_eq!(model.endpoints().len(), 2);
        assert_eq!(model.endpoints()[1].1.display_name(), "fresh");
    }

    #[test]
    fn discovery_failure_keeps_endpoints_and_records_error() {
        let mut model = Model::new();
        update(&mut model, Msg::RefreshDiscovery);
        update(
            &mut model,
            Msg::DiscoveryLoaded {
                generation: 1,
                endpoints: vec![daemon_endpoint("alpha", "192.168.1.10:9031")],
            },
        );
        update(&mut model, Msg::RefreshDiscovery);
        let effects = update(
            &mut model,
            Msg::DiscoveryFailed {
                generation: 2,
                error: "network unreachable".to_string(),
            },
        );
        assert!(effects.is_empty());
        assert!(!model.is_discovering());
        assert_eq!(model.endpoints().len(), 2, "previous snapshot kept");
        assert_eq!(model.last_error(), Some("network unreachable"));
    }

    #[test]
    fn stale_discovery_failure_is_dropped() {
        let mut model = Model::new();
        update(&mut model, Msg::RefreshDiscovery);
        update(&mut model, Msg::RefreshDiscovery);
        let effects = update(
            &mut model,
            Msg::DiscoveryFailed {
                generation: 1,
                error: "stale error".to_string(),
            },
        );
        assert!(effects.is_empty());
        assert!(model.is_discovering());
        assert_eq!(model.last_error(), None);
    }
}
