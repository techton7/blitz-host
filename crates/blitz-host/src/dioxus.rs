//! # Dioxus Native Integration for `blitz-host`
//!
//! Provides zero-overhead, declarative host integration for Dioxus Native applications
//! running on Blitz.
//!
//! # Usage
//!
//! In your main application or root component:
//! ```rust,ignore
//! use blitz_host::BlitzHost;
//!
//! #[component]
//! fn App() -> Element {
//!     rsx! {
//!         BlitzHost {
//!             // Your normal Dioxus Native UI here
//!         }
//!     }
//! }
//! ```
//!
//! In `main()` (optional, for explicit app metadata logging):
//! ```rust,ignore
//! blitz_host::init_if_debug("my-app", env!("CARGO_PKG_VERSION"));
//! ```

use std::cell::Cell;
use std::rc::Rc;

use dioxus::prelude::*;
use dioxus_native::winit::event::WindowEvent;

use crate::host::HostControl;

/// Root wrapper component that automatically injects `blitz-host` debug control
/// into a Dioxus Native application.
///
/// In the feature-enabled development lane (`feature = "blitz-host"`):
/// 1. Captures the live window `NodeHandle` from the root mounted DOM element with
///    zero layout interference (`display: contents;`).
/// 2. Hooks into `WindowEvent::RedrawRequested` to automatically service pending debug
///    control requests on the main UI thread right before each frame.
/// 3. Automatically handles synthetic action dispatches (such as click events).
///
/// When debug control is suppressed (via `--no-debug-control` or `BLITZ_DEBUG_CONTROL=0`),
/// this component simply renders `children` with zero overhead.
#[component]
pub fn BlitzHost(children: Element) -> Element {
    // Attempt automatic fallback initialization if not already initialized
    use_hook(|| {
        if !HostControl::is_global_active() && HostControl::is_enabled() {
            crate::host::init_default();
        }
    });


    let live_handle = use_hook(|| Rc::new(std::cell::RefCell::new(None::<dioxus_native::NodeHandle>)));
    let handle_for_mount = live_handle.clone();
    let handle_for_event = live_handle.clone();

    let frame_counter = use_hook(|| Cell::new(0u64));
    let window = dioxus_native::use_window();

    // Listen to native window redraw requests to service control requests
    dioxus_native::use_window_event(move |event, _target| {
        if let WindowEvent::RedrawRequested = event {
            if !HostControl::is_global_active() {
                return;
            }
            let frame = frame_counter.get() + 1;
            frame_counter.set(frame);

            if let Some(handle) = handle_for_event.borrow().as_ref() {
                let doc = handle.doc();
                let serviced = HostControl::service_global_frame(
                    &doc,
                    frame,
                    |action_req, base_doc| {
                        HostControl::handle_action_click(action_req, base_doc, |d, nid| {
                            dioxus_native::dispatch_synthetic_click(
                                d,
                                blitz_dom::NodeId::from_u64(nid),
                                keyboard_types::Modifiers::empty(),
                            )
                        })
                    },
                );

                if serviced > 0 {
                    window.request_redraw();
                }
            }
        }
    });

    rsx! {
        div {
            style: "display: contents;",
            onmounted: move |evt: Event<MountedData>| {
                if let Some(handle) = evt.downcast::<dioxus_native::NodeHandle>() {
                    *handle_for_mount.borrow_mut() = Some(handle.clone());
                    let doc_id = handle.doc().id();
                    let win_id = u64::from(window.id());
                    HostControl::set_global_primary_window(Some(win_id), Some(doc_id));
                }
            },
            {children}
        }
    }
}
