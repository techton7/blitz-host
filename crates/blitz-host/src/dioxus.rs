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

use dioxus_native::prelude::*;
use dioxus_native::winit::event::WindowEvent;

use crate::host::{HostControl, ServiceResult};

fn service_turn(
    doc_rc: &Rc<std::cell::RefCell<blitz_dom::BaseDocument>>,
    frame: u64,
) -> ServiceResult {
    HostControl::service_global_frame_rc(doc_rc, frame, |action_req, doc_rc| {
        HostControl::handle_action_rc(
            action_req,
            doc_rc,
            |d, nid| {
                crate::dispatch::dispatch_click_rc(
                    d,
                    blitz_dom::NodeId::from_u64(nid),
                    keyboard_types::Modifiers::empty(),
                )
            },
            |d, nid, focus_visible| {
                crate::dispatch::dispatch_focus_rc(
                    d,
                    blitz_dom::NodeId::from_u64(nid),
                    focus_visible,
                )
            },
            |d, nid, val| {
                crate::dispatch::dispatch_input_rc(d, blitz_dom::NodeId::from_u64(nid), val)
            },
            |d, nid, key, mods| {
                let mut kb_mods = keyboard_types::Modifiers::empty();
                if let Some(m) = mods {
                    if m.shift {
                        kb_mods |= keyboard_types::Modifiers::SHIFT;
                    }
                    if m.ctrl {
                        kb_mods |= keyboard_types::Modifiers::CONTROL;
                    }
                    if m.alt {
                        kb_mods |= keyboard_types::Modifiers::ALT;
                    }
                    if m.meta {
                        kb_mods |= keyboard_types::Modifiers::META;
                    }
                }
                crate::dispatch::dispatch_key_rc(
                    d,
                    nid.map(blitz_dom::NodeId::from_u64),
                    key,
                    kb_mods,
                )
                .map(|nid| nid.as_u64())
            },
            |d, nid, coords, mods| {
                let mut kb_mods = keyboard_types::Modifiers::empty();
                if let Some(m) = mods {
                    if m.shift {
                        kb_mods |= keyboard_types::Modifiers::SHIFT;
                    }
                    if m.ctrl {
                        kb_mods |= keyboard_types::Modifiers::CONTROL;
                    }
                    if m.alt {
                        kb_mods |= keyboard_types::Modifiers::ALT;
                    }
                    if m.meta {
                        kb_mods |= keyboard_types::Modifiers::META;
                    }
                }
                crate::dispatch::dispatch_pointer_move_rc(
                    d,
                    nid.map(blitz_dom::NodeId::from_u64),
                    coords,
                    kb_mods,
                )
                .map(|nid| nid.as_u64())
            },
            |d, nid, coords, btn, mods| {
                let mut kb_mods = keyboard_types::Modifiers::empty();
                if let Some(m) = mods {
                    if m.shift {
                        kb_mods |= keyboard_types::Modifiers::SHIFT;
                    }
                    if m.ctrl {
                        kb_mods |= keyboard_types::Modifiers::CONTROL;
                    }
                    if m.alt {
                        kb_mods |= keyboard_types::Modifiers::ALT;
                    }
                    if m.meta {
                        kb_mods |= keyboard_types::Modifiers::META;
                    }
                }
                let button = match btn {
                    Some("right") | Some("secondary") => {
                        crate::dispatch::MouseEventButton::Secondary
                    }
                    Some("middle") | Some("auxiliary") => {
                        crate::dispatch::MouseEventButton::Auxiliary
                    }
                    _ => crate::dispatch::MouseEventButton::Main,
                };
                crate::dispatch::dispatch_pointer_down_rc(
                    d,
                    nid.map(blitz_dom::NodeId::from_u64),
                    coords,
                    button,
                    kb_mods,
                )
                .map(|nid| nid.as_u64())
            },
            |d, nid, coords, btn, mods| {
                let mut kb_mods = keyboard_types::Modifiers::empty();
                if let Some(m) = mods {
                    if m.shift {
                        kb_mods |= keyboard_types::Modifiers::SHIFT;
                    }
                    if m.ctrl {
                        kb_mods |= keyboard_types::Modifiers::CONTROL;
                    }
                    if m.alt {
                        kb_mods |= keyboard_types::Modifiers::ALT;
                    }
                    if m.meta {
                        kb_mods |= keyboard_types::Modifiers::META;
                    }
                }
                let button = match btn {
                    Some("right") | Some("secondary") => {
                        crate::dispatch::MouseEventButton::Secondary
                    }
                    Some("middle") | Some("auxiliary") => {
                        crate::dispatch::MouseEventButton::Auxiliary
                    }
                    _ => crate::dispatch::MouseEventButton::Main,
                };
                crate::dispatch::dispatch_pointer_up_rc(
                    d,
                    nid.map(blitz_dom::NodeId::from_u64),
                    coords,
                    button,
                    kb_mods,
                )
                .map(|nid| nid.as_u64())
            },
            |d, nid, coords, dx, dy, mods| {
                let mut kb_mods = keyboard_types::Modifiers::empty();
                if let Some(m) = mods {
                    if m.shift {
                        kb_mods |= keyboard_types::Modifiers::SHIFT;
                    }
                    if m.ctrl {
                        kb_mods |= keyboard_types::Modifiers::CONTROL;
                    }
                    if m.alt {
                        kb_mods |= keyboard_types::Modifiers::ALT;
                    }
                    if m.meta {
                        kb_mods |= keyboard_types::Modifiers::META;
                    }
                }
                crate::dispatch::dispatch_wheel_rc(
                    d,
                    nid.map(blitz_dom::NodeId::from_u64),
                    coords,
                    dx,
                    dy,
                    kb_mods,
                )
                .map(|nid| nid.as_u64())
            },
        )
    })
}

#[cfg(target_os = "macos")]
fn wake_macos_main_runloop() {
    unsafe extern "C" {
        fn CFRunLoopGetMain() -> *mut std::ffi::c_void;
        fn CFRunLoopWakeUp(rl: *mut std::ffi::c_void);
    }
    unsafe {
        let main_rl = CFRunLoopGetMain();
        if !main_rl.is_null() {
            CFRunLoopWakeUp(main_rl);
        }
    }
}

/// Root wrapper component that automatically injects `blitz-host` debug control
/// into a Dioxus Native application.
///
/// In the feature-enabled development lane (`feature = "blitz-host"`):
/// 1. Captures the live window `NodeHandle` from the root mounted DOM element with
///    zero layout interference (`display: contents;`).
/// 2. Wakes the native event loop with a redraw request whenever the background server
///    receives control traffic, so servicing occurs on the main UI thread even while hidden.
/// 3. Hooks into `WindowEvent::RedrawRequested` to advance frame sequence and service actions
///    during visible rendering passes.
/// 4. Automatically handles synthetic action dispatches (such as click events).
///
/// When debug control is suppressed (via `--no-debug-control` or `BLITZ_DEBUG_CONTROL=0`),
/// this component simply renders `children` with zero overhead.
#[component]
pub fn BlitzHost(children: Element) -> Element {
    let window = dioxus_native::use_window();
    let window_for_redraw = window.clone();
    let window_for_event = window.clone();
    let window_for_wake = window.clone();

    let live_handle =
        use_hook(|| Rc::new(std::cell::RefCell::new(None::<dioxus_native::NodeHandle>)));
    let handle_for_mount = live_handle.clone();
    let handle_for_event = live_handle.clone();
    let frame_counter = use_hook(|| Rc::new(Cell::new(0u64)));
    let frame_counter_for_event = frame_counter.clone();

    // Attempt automatic fallback initialization and register the native event-loop waker.
    use_hook(|| {
        if !HostControl::is_global_active() && HostControl::is_enabled() {
            crate::host::init_default();
        }

        // The server runs on background threads. Waking a Dioxus task alone is insufficient:
        // the native executor is polled by the winit event loop. Request a real redraw here so
        // every queued request is serviced on the UI thread, even when the window is hidden.
        HostControl::set_global_waker(move || {
            window_for_wake.request_redraw();
            #[cfg(target_os = "macos")]
            wake_macos_main_runloop();
        });

        // Requests that arrived before mounting still need an actual native event-loop turn.
        window_for_redraw.request_redraw();
        #[cfg(target_os = "macos")]
        wake_macos_main_runloop();
    });

    use_drop(|| {
        HostControl::clear_global_waker();
    });

    // Listen to native window redraw requests to service control requests
    dioxus_native::use_window_event(move |event, _target| {
        if let WindowEvent::RedrawRequested = event {
            if !HostControl::is_global_active() {
                return;
            }
            if let Some(handle) = handle_for_event.borrow().as_ref() {
                let frame = frame_counter_for_event.get() + 1;
                frame_counter_for_event.set(frame);
                let doc_rc = handle.base_document();
                let result = service_turn(&doc_rc, frame);
                if result.needs_redraw {
                    window_for_event.request_redraw();
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
                    use std::hash::{Hash, Hasher};
                    let mut hasher = std::collections::hash_map::DefaultHasher::new();
                    window.id().hash(&mut hasher);
                    let win_id = hasher.finish();
                    HostControl::set_global_primary_window(Some(win_id), Some(doc_id));
                }
            },
            {children}
        }
    }
}
