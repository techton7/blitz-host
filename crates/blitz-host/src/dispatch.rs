//! Host Action Dispatching for Blitz Host
//!
//! Provides internal host action dispatching that directly drives
//! Blitz DOM's standard [`BaseDocument::handle_ui_event`] event pipeline
//! and delivers corresponding events to the active Dioxus runtime.

use std::any::Any;
use std::cell::{Cell, RefCell};
use std::rc::{Rc, Weak};
use std::str::FromStr;

use ::keyboard_types::{Code, Key, Location, Modifiers, NamedKey};
use blitz_dom::{
    BaseDocument, Document, EventDriver, EventHandler as BlitzEventHandler, FocusOptions,
    FocusSource, Node, NodeId, ScrollBehavior, ScrollLogicalPosition,
};
use blitz_traits::SmolStr;
pub use blitz_traits::events::MouseEventButton;
use blitz_traits::events::{
    BlitzKeyEvent, BlitzPointerEvent, BlitzPointerId, BlitzWheelDelta, BlitzWheelEvent,
    BlitzWheelPhase, BlitzWheelSource, DomEvent, DomEventData, EventState, KeyState,
    MouseEventButtons, PointerCoords, UiEvent,
};
use dioxus_native::prelude::*;

std::thread_local! {
    static HELD_MOUSE_BUTTONS: Cell<MouseEventButtons> = const { Cell::new(MouseEventButtons::None) };
}

struct HostDioxusEventHandler {
    doc_handle: Option<Weak<RefCell<BaseDocument>>>,
}

impl BlitzEventHandler for HostDioxusEventHandler {
    fn handle_event(
        &mut self,
        chain: &[NodeId],
        event: &mut DomEvent,
        doc: &mut dyn Document,
        event_state: &mut EventState,
    ) {
        let Ok(runtime) = std::panic::catch_unwind(dioxus_core::Runtime::current) else {
            return;
        };

        let event_data: Option<Rc<dyn Any>> = match &event.data {
            DomEventData::PointerMove(mevent)
            | DomEventData::PointerDown(mevent)
            | DomEventData::PointerUp(mevent)
            | DomEventData::PointerCancel(mevent)
            | DomEventData::PointerLeave(mevent)
            | DomEventData::PointerEnter(mevent)
            | DomEventData::PointerOver(mevent)
            | DomEventData::PointerOut(mevent)
            | DomEventData::GotPointerCapture(mevent)
            | DomEventData::LostPointerCapture(mevent)
            | DomEventData::MouseMove(mevent)
            | DomEventData::MouseDown(mevent)
            | DomEventData::MouseUp(mevent)
            | DomEventData::MouseLeave(mevent)
            | DomEventData::MouseEnter(mevent)
            | DomEventData::MouseOver(mevent)
            | DomEventData::MouseOut(mevent)
            | DomEventData::Click(mevent)
            | DomEventData::ContextMenu(mevent)
            | DomEventData::DoubleClick(mevent) => {
                let ptr = if let Some(weak_doc) = &self.doc_handle {
                    dioxus_native::NativePointerData::with_target(
                        mevent.clone(),
                        weak_doc.clone(),
                        event.target,
                    )
                } else {
                    dioxus_native::NativePointerData::new(mevent.clone())
                };
                Some(Rc::new(PlatformEventData::new(Box::new(ptr))) as Rc<dyn Any>)
            }
            DomEventData::Focus(_)
            | DomEventData::Blur(_)
            | DomEventData::FocusIn(_)
            | DomEventData::FocusOut(_) => {
                Some(Rc::new(PlatformEventData::new(Box::new(()))) as Rc<dyn Any>)
            }
            DomEventData::KeyDown(kevent)
            | DomEventData::KeyUp(kevent)
            | DomEventData::KeyPress(kevent) => Some(Rc::new(PlatformEventData::new(
                dioxus_native::synthetic_keyboard_event(kevent.clone()),
            )) as Rc<dyn Any>),
            DomEventData::Input(data) | DomEventData::Change(data) => Some(Rc::new(
                PlatformEventData::new(dioxus_native::synthetic_form_event(
                    data.value.clone(),
                    vec![],
                )),
            ) as Rc<dyn Any>),
            DomEventData::Submit(data) => {
                let values = data
                    .entries
                    .iter()
                    .map(|blitz_traits::net::Entry { name, value }| {
                        let form_value = match value {
                            blitz_traits::net::EntryValue::String(s) => {
                                dioxus::prelude::FormValue::Text(s.clone())
                            }
                            blitz_traits::net::EntryValue::File(_)
                            | blitz_traits::net::EntryValue::EmptyFile => {
                                dioxus::prelude::FormValue::File(None)
                            }
                        };
                        (name.clone(), form_value)
                    })
                    .collect::<Vec<_>>();
                Some(Rc::new(PlatformEventData::new(
                    dioxus_native::synthetic_form_event(String::new(), values),
                )) as Rc<dyn Any>)
            }
            _ => None,
        };

        let Some(event_data) = event_data else {
            return;
        };

        let dioxus_id = {
            let inner = doc.inner();
            chain
                .iter()
                .find_map(|node_id| inner.get_node(*node_id).and_then(get_dioxus_id))
        };
        let Some(id) = dioxus_id else {
            return;
        };

        let dx_event = dioxus_core::Event::new(event_data, event.bubbles);
        runtime.handle_event(event.name(), dx_event.clone(), id);

        if !dx_event.default_action_enabled() {
            event_state.prevent_default();
        }
        if !dx_event.propagates() {
            event_state.stop_propagation();
        }
    }
}

/// Extract Dioxus [`dioxus_core::ElementId`] from node's "data-dioxus-id" attribute.
pub fn get_dioxus_id(node: &Node) -> Option<dioxus_core::ElementId> {
    node.element_data()?
        .attrs
        .iter()
        .find(|attr| *attr.name.local == *"data-dioxus-id")
        .and_then(|attr| attr.value.parse::<usize>().ok())
        .map(dioxus_core::ElementId::from_raw)
}

/// Helper to find nearest Dioxus element ID and target NodeId for a given node
/// by traversing up the parent chain.
pub fn find_dioxus_target(
    doc: &BaseDocument,
    node_id: NodeId,
) -> Option<(dioxus_core::ElementId, NodeId)> {
    let mut current = Some(node_id);
    while let Some(id) = current {
        if let Some(node) = doc.get_node(id) {
            if let Some(dioxus_id) = get_dioxus_id(node) {
                return Some((dioxus_id, id));
            }
            current = node.parent;
        } else {
            break;
        }
    }
    None
}

/// Helper to compute absolute center coordinates of a node.
pub fn get_node_center_coords(doc: &BaseDocument, node_id: NodeId) -> Option<(f32, f32)> {
    let node = doc.get_node(node_id)?;
    let pos = node.absolute_position(0.0, 0.0);
    let size = node.final_layout().size;
    Some((pos.x + size.width / 2.0, pos.y + size.height / 2.0))
}

/// Helper to map key string to (Key, Code, Modifiers).
pub fn parse_key_str(raw: &str) -> (Key, Code, Modifiers) {
    fn trim_preserving_space(s: &str) -> &str {
        let t = s.trim();
        if t.is_empty() && s.contains(' ') {
            " "
        } else {
            t
        }
    }
    let mut mods = Modifiers::empty();
    let mut key_part = trim_preserving_space(raw);

    while let Some(idx) = key_part.find('+') {
        let prefix = key_part[..idx].trim();
        if prefix.eq_ignore_ascii_case("shift") {
            mods |= Modifiers::SHIFT;
        } else if prefix.eq_ignore_ascii_case("ctrl") || prefix.eq_ignore_ascii_case("control") {
            mods |= Modifiers::CONTROL;
        } else if prefix.eq_ignore_ascii_case("cmd")
            || prefix.eq_ignore_ascii_case("command")
            || prefix.eq_ignore_ascii_case("meta")
            || prefix.eq_ignore_ascii_case("super")
        {
            mods |= Modifiers::META;
        } else if prefix.eq_ignore_ascii_case("alt")
            || prefix.eq_ignore_ascii_case("opt")
            || prefix.eq_ignore_ascii_case("option")
        {
            mods |= Modifiers::ALT;
        }
        key_part = trim_preserving_space(&key_part[idx + 1..]);
    }

    let (key, code) = if key_part.eq_ignore_ascii_case("tab") {
        (Key::Named(NamedKey::Tab), Code::Tab)
    } else if key_part.eq_ignore_ascii_case("enter") || key_part.eq_ignore_ascii_case("return") {
        (Key::Named(NamedKey::Enter), Code::Enter)
    } else if key_part.eq_ignore_ascii_case("space") || key_part == " " {
        (Key::Character(" ".into()), Code::Space)
    } else if key_part.eq_ignore_ascii_case("escape") || key_part.eq_ignore_ascii_case("esc") {
        (Key::Named(NamedKey::Escape), Code::Escape)
    } else if key_part.eq_ignore_ascii_case("backspace") {
        (Key::Named(NamedKey::Backspace), Code::Backspace)
    } else if key_part.eq_ignore_ascii_case("delete") || key_part.eq_ignore_ascii_case("del") {
        (Key::Named(NamedKey::Delete), Code::Delete)
    } else if key_part.eq_ignore_ascii_case("arrowleft") || key_part.eq_ignore_ascii_case("left") {
        (Key::Named(NamedKey::ArrowLeft), Code::ArrowLeft)
    } else if key_part.eq_ignore_ascii_case("arrowright") || key_part.eq_ignore_ascii_case("right")
    {
        (Key::Named(NamedKey::ArrowRight), Code::ArrowRight)
    } else if key_part.eq_ignore_ascii_case("arrowup") || key_part.eq_ignore_ascii_case("up") {
        (Key::Named(NamedKey::ArrowUp), Code::ArrowUp)
    } else if key_part.eq_ignore_ascii_case("arrowdown") || key_part.eq_ignore_ascii_case("down") {
        (Key::Named(NamedKey::ArrowDown), Code::ArrowDown)
    } else if key_part.eq_ignore_ascii_case("a") {
        (Key::Character(key_part.to_string()), Code::KeyA)
    } else if key_part.eq_ignore_ascii_case("c") {
        (Key::Character(key_part.to_string()), Code::KeyC)
    } else if key_part.eq_ignore_ascii_case("v") {
        (Key::Character(key_part.to_string()), Code::KeyV)
    } else if key_part.eq_ignore_ascii_case("x") {
        (Key::Character(key_part.to_string()), Code::KeyX)
    } else if key_part.eq_ignore_ascii_case("z") {
        (Key::Character(key_part.to_string()), Code::KeyZ)
    } else if key_part.chars().count() == 1 {
        (Key::Character(key_part.to_string()), Code::Unidentified)
    } else {
        (
            Key::from_str(key_part).unwrap_or(Key::Named(NamedKey::Unidentified)),
            Code::from_str(key_part).unwrap_or(Code::Unidentified),
        )
    };

    (key, code, mods)
}

fn attr_val<'a>(el: &'a blitz_dom::ElementData, name: &str) -> Option<&'a str> {
    el.attrs()
        .iter()
        .find(|a| &*a.name.local == name)
        .map(|a| a.value.as_ref())
}

fn has_attr_name(el: &blitz_dom::ElementData, name: &str) -> bool {
    attr_val(el, name).is_some()
}

fn is_host_form_control_disabled(doc: &BaseDocument, node_id: NodeId) -> bool {
    let Some(el) = doc.get_node(node_id).and_then(|n| n.element_data()) else {
        return false;
    };
    if !matches!(&*el.name.local, "button" | "input" | "select" | "textarea") {
        return false;
    }
    if has_attr_name(el, "disabled") {
        return true;
    }
    let mut child_id = node_id;
    while let Some(parent_id) = doc.get_node(child_id).and_then(|n| n.parent) {
        let Some(parent) = doc.get_node(parent_id) else {
            break;
        };
        if let Some(pel) = parent.element_data()
            && &*pel.name.local == "fieldset"
            && has_attr_name(pel, "disabled")
        {
            let first_legend_id = parent.children.iter().copied().find(|&cid| {
                doc.get_node(cid)
                    .and_then(|n| n.element_data())
                    .is_some_and(|cel| &*cel.name.local == "legend")
            });
            if Some(child_id) != first_legend_id {
                return true;
            }
        }
        child_id = parent_id;
    }
    false
}

enum PressFocusResolution {
    Focus(NodeId),
    Clear,
    Unchanged,
}

fn resolve_press_focus_target(doc: &BaseDocument, start_id: NodeId) -> PressFocusResolution {
    let mut curr = Some(start_id);
    while let Some(node_id) = curr {
        if is_host_form_control_disabled(doc, node_id) {
            return PressFocusResolution::Unchanged;
        }
        let Some(node) = doc.get_node(node_id) else {
            break;
        };
        if let Some(el) = node.element_data() {
            let has_tabindex =
                attr_val(el, "tabindex").is_some_and(|v| v.trim().parse::<i32>().is_ok());
            if el.is_focussable || has_tabindex {
                return PressFocusResolution::Focus(node_id);
            }
        }
        curr = node.parent;
    }
    PressFocusResolution::Clear
}

/// Construct click event payload for a node.
pub fn click_event_payload(node: &Node, modifiers: Modifiers) -> Box<dyn Any> {
    dioxus_native::synthetic_click_event(node, modifiers)
}

/// Dispatch a click event on a target node in `doc`, bubbling up the DOM hierarchy
/// to locate the nearest Dioxus element listener.
pub fn dispatch_click(doc: &BaseDocument, node_id: NodeId, modifiers: Modifiers) -> bool {
    let Some((dioxus_id, target_node_id)) = find_dioxus_target(doc, node_id) else {
        return false;
    };

    let Some(target_node) = doc.get_node(target_node_id) else {
        return false;
    };

    let event_data = click_event_payload(target_node, modifiers);
    let platform_event = PlatformEventData::new(event_data);
    let dx_event = dioxus_core::Event::new(Rc::new(platform_event) as Rc<dyn Any>, true);

    if let Ok(runtime) = std::panic::catch_unwind(dioxus_core::Runtime::current) {
        runtime.handle_event("click", dx_event, dioxus_id);
        true
    } else {
        false
    }
}

/// Dispatch a pointer-modality click event on `doc_rc`, updating pointer focus modality
/// (`FocusSource::Pointer`) before delivering `"click"` with a target-aware `NativePointerData`.
pub fn dispatch_click_rc(
    doc_rc: &Rc<RefCell<BaseDocument>>,
    node_id: NodeId,
    modifiers: Modifiers,
) -> bool {
    let (old_dx, new_dx, click_dispatch) = {
        let mut doc = doc_rc.borrow_mut();
        if doc.get_node(node_id).is_none() {
            return false;
        }
        let old_focus = doc.get_focussed_node_id();
        match resolve_press_focus_target(&doc, node_id) {
            PressFocusResolution::Focus(focus_id) => {
                let _ = doc.set_focus_to_with_source(focus_id, FocusSource::Pointer);
            }
            PressFocusResolution::Clear => {
                let root_id = doc.root_node().id;
                let _ = doc.set_focus_to_with_source(root_id, FocusSource::Pointer);
                doc.clear_focus();
            }
            PressFocusResolution::Unchanged => {}
        }
        let new_focus = doc.get_focussed_node_id();
        let old_dx = if old_focus != new_focus {
            old_focus.and_then(|oid| find_dioxus_target(&doc, oid).map(|(dx, _)| dx))
        } else {
            None
        };
        let new_dx = if old_focus != new_focus {
            new_focus.and_then(|nid| find_dioxus_target(&doc, nid).map(|(dx, _)| dx))
        } else {
            None
        };

        let click_dispatch = find_dioxus_target(&doc, node_id).and_then(|(dx_id, target_nid)| {
            let (x, y) = get_node_center_coords(&doc, target_nid).unwrap_or((0.0, 0.0));
            let ptr_event = BlitzPointerEvent {
                id: BlitzPointerId::Mouse,
                is_primary: true,
                coords: PointerCoords {
                    page_x: x,
                    page_y: y,
                    screen_x: x,
                    screen_y: y,
                    client_x: x,
                    client_y: y,
                },
                button: MouseEventButton::Main,
                buttons: MouseEventButtons::None,
                mods: modifiers,
                details: Default::default(),
                element: Default::default(),
                active_pointers: Default::default(),
            };
            let ptr_data = dioxus_native::NativePointerData::with_target(
                ptr_event,
                Rc::downgrade(doc_rc),
                target_nid,
            );
            Some((dx_id, ptr_data))
        });

        (old_dx, new_dx, click_dispatch)
    };

    let Ok(runtime) = std::panic::catch_unwind(dioxus_core::Runtime::current) else {
        return true;
    };

    if let Some(old_id) = old_dx {
        let platform_event = Rc::new(PlatformEventData::new(Box::new(())));
        let dx_blur = dioxus_core::Event::new(platform_event.clone() as Rc<dyn Any>, false);
        let dx_focusout = dioxus_core::Event::new(platform_event as Rc<dyn Any>, true);
        runtime.handle_event("blur", dx_blur, old_id);
        runtime.handle_event("focusout", dx_focusout, old_id);
    }
    if let Some(new_id) = new_dx {
        let platform_event = Rc::new(PlatformEventData::new(Box::new(())));
        let dx_focus = dioxus_core::Event::new(platform_event.clone() as Rc<dyn Any>, false);
        let dx_focusin = dioxus_core::Event::new(platform_event as Rc<dyn Any>, true);
        runtime.handle_event("focus", dx_focus, new_id);
        runtime.handle_event("focusin", dx_focusin, new_id);
    }

    if let Some((dioxus_id, ptr_data)) = click_dispatch {
        let platform_event = PlatformEventData::new(Box::new(ptr_data));
        let dx_event = dioxus_core::Event::new(Rc::new(platform_event) as Rc<dyn Any>, true);
        runtime.handle_event("click", dx_event, dioxus_id);
    }
    true
}

/// Dispatch a focus event on a target node in `doc`.
/// Sets focus on the Blitz DOM node and dispatches "focus" and "focusin" events to Dioxus listeners.
pub fn dispatch_focus(doc: &mut BaseDocument, node_id: NodeId) -> bool {
    dispatch_focus_with_options(doc, node_id, FocusOptions::default())
}

/// Dispatch a focus event on a target node in `doc` with explicit [`FocusOptions`].
pub fn dispatch_focus_with_options(
    doc: &mut BaseDocument,
    node_id: NodeId,
    options: FocusOptions,
) -> bool {
    let Some((dioxus_id, target_node_id)) = find_dioxus_target(doc, node_id) else {
        let _ = doc.set_focus_to_with_options(node_id, FocusSource::Script, options);
        return doc.get_focussed_node_id() == Some(node_id);
    };

    let old_focus = doc.get_focussed_node_id();
    let _ = doc.set_focus_to_with_options(target_node_id, FocusSource::Script, options);
    let old_dx = if old_focus != Some(target_node_id) {
        old_focus.and_then(|oid| find_dioxus_target(doc, oid).map(|(dx, _)| dx))
    } else {
        None
    };

    if let Ok(runtime) = std::panic::catch_unwind(dioxus_core::Runtime::current) {
        if let Some(old_id) = old_dx {
            let platform_blur = Rc::new(PlatformEventData::new(Box::new(())));
            let dx_blur = dioxus_core::Event::new(platform_blur.clone() as Rc<dyn Any>, false);
            let dx_focusout = dioxus_core::Event::new(platform_blur as Rc<dyn Any>, true);
            runtime.handle_event("blur", dx_blur, old_id);
            runtime.handle_event("focusout", dx_focusout, old_id);
        }
        let platform_event = Rc::new(PlatformEventData::new(Box::new(())));
        let dx_focus = dioxus_core::Event::new(platform_event.clone() as Rc<dyn Any>, false);
        runtime.handle_event("focus", dx_focus, dioxus_id);
        let dx_focusin = dioxus_core::Event::new(platform_event as Rc<dyn Any>, true);
        runtime.handle_event("focusin", dx_focusin, dioxus_id);
        true
    } else {
        true
    }
}

/// Borrow-safe `Rc<RefCell<BaseDocument>>` variant of [`dispatch_focus_with_options`].
pub fn dispatch_focus_rc(
    doc_rc: &Rc<RefCell<BaseDocument>>,
    node_id: NodeId,
    focus_visible: Option<bool>,
) -> bool {
    let options = FocusOptions {
        prevent_scroll: false,
        focus_visible,
    };
    let (target_dx, old_dx, raw_ok) = {
        let mut doc = doc_rc.borrow_mut();
        if let Some((dioxus_id, target_node_id)) = find_dioxus_target(&doc, node_id) {
            let old_focus = doc.get_focussed_node_id();
            let _ = doc.set_focus_to_with_options(target_node_id, FocusSource::Script, options);
            let old_dx = if old_focus != Some(target_node_id) {
                old_focus.and_then(|oid| find_dioxus_target(&doc, oid).map(|(dx, _)| dx))
            } else {
                None
            };
            (Some(dioxus_id), old_dx, true)
        } else {
            let _ = doc.set_focus_to_with_options(node_id, FocusSource::Script, options);
            let ok = doc.get_focussed_node_id() == Some(node_id);
            (None, None, ok)
        }
    };

    let Some(dioxus_id) = target_dx else {
        return raw_ok;
    };

    if let Ok(runtime) = std::panic::catch_unwind(dioxus_core::Runtime::current) {
        if let Some(old_id) = old_dx {
            let platform_blur = Rc::new(PlatformEventData::new(Box::new(())));
            let dx_blur = dioxus_core::Event::new(platform_blur.clone() as Rc<dyn Any>, false);
            let dx_focusout = dioxus_core::Event::new(platform_blur as Rc<dyn Any>, true);
            runtime.handle_event("blur", dx_blur, old_id);
            runtime.handle_event("focusout", dx_focusout, old_id);
        }
        let platform_event = Rc::new(PlatformEventData::new(Box::new(())));
        let dx_focus = dioxus_core::Event::new(platform_event.clone() as Rc<dyn Any>, false);
        runtime.handle_event("focus", dx_focus, dioxus_id);
        let dx_focusin = dioxus_core::Event::new(platform_event as Rc<dyn Any>, true);
        runtime.handle_event("focusin", dx_focusin, dioxus_id);
    }
    true
}

/// Scroll `node_id` into the viewport (nearest alignment, instant) and request a redraw.
pub fn dispatch_scroll_into_view(doc: &mut BaseDocument, node_id: NodeId) -> bool {
    if doc.get_node(node_id).is_none() {
        return false;
    }
    doc.scroll_into_view(
        node_id,
        ScrollBehavior::Instant,
        ScrollLogicalPosition::Nearest,
        ScrollLogicalPosition::Nearest,
    );
    doc.shell_provider.request_redraw();
    true
}

/// Borrow-safe `Rc<RefCell<BaseDocument>>` variant of [`dispatch_scroll_into_view`].
pub fn dispatch_scroll_into_view_rc(doc_rc: &Rc<RefCell<BaseDocument>>, node_id: NodeId) -> bool {
    let mut doc = doc_rc.borrow_mut();
    dispatch_scroll_into_view(&mut doc, node_id)
}

/// Dispatch an input/value change on a target node in `doc`.
/// Updates the Blitz DOM text input editor (if applicable) and dispatches "input" and "change"
/// events containing the new value to Dioxus listeners.
pub fn dispatch_input(doc: &mut BaseDocument, node_id: NodeId, value: &str) -> bool {
    let Some((dioxus_id, target_node_id)) = find_dioxus_target(doc, node_id) else {
        return false;
    };

    doc.set_text_input_value(target_node_id, value);

    let event_data = dioxus_native::synthetic_form_event(value, vec![]);
    let platform_event = Rc::new(PlatformEventData::new(event_data));
    let dx_input = dioxus_core::Event::new(platform_event.clone() as Rc<dyn Any>, true);

    if let Ok(runtime) = std::panic::catch_unwind(dioxus_core::Runtime::current) {
        runtime.handle_event("input", dx_input, dioxus_id);
        let dx_change = dioxus_core::Event::new(platform_event as Rc<dyn Any>, true);
        runtime.handle_event("change", dx_change, dioxus_id);
        true
    } else {
        true
    }
}

/// Borrow-safe `Rc<RefCell<BaseDocument>>` variant of [`dispatch_input`].
pub fn dispatch_input_rc(doc_rc: &Rc<RefCell<BaseDocument>>, node_id: NodeId, value: &str) -> bool {
    let dioxus_id = {
        let mut doc = doc_rc.borrow_mut();
        let Some((dioxus_id, target_node_id)) = find_dioxus_target(&doc, node_id) else {
            return false;
        };
        doc.set_text_input_value(target_node_id, value);
        dioxus_id
    };

    let event_data = dioxus_native::synthetic_form_event(value, vec![]);
    let platform_event = Rc::new(PlatformEventData::new(event_data));
    let dx_input = dioxus_core::Event::new(platform_event.clone() as Rc<dyn Any>, true);

    if let Ok(runtime) = std::panic::catch_unwind(dioxus_core::Runtime::current) {
        runtime.handle_event("input", dx_input, dioxus_id);
        let dx_change = dioxus_core::Event::new(platform_event as Rc<dyn Any>, true);
        runtime.handle_event("change", dx_change, dioxus_id);
    }
    true
}

fn build_key_events(
    key_str: &str,
    modifiers: Modifiers,
) -> (Key, Modifiers, BlitzKeyEvent, BlitzKeyEvent) {
    let (key, code, parsed_mods) = parse_key_str(key_str);
    let combined_mods = modifiers | parsed_mods;
    let text_payload = match &key {
        Key::Character(s) => Some(SmolStr::new(s)),
        _ => None,
    };
    let key_down = BlitzKeyEvent {
        key: key.clone(),
        code,
        modifiers: combined_mods,
        location: Location::Standard,
        is_auto_repeating: false,
        is_composing: false,
        state: KeyState::Pressed,
        text: text_payload,
    };
    let key_up = BlitzKeyEvent {
        key: key.clone(),
        code,
        modifiers: combined_mods,
        location: Location::Standard,
        is_auto_repeating: false,
        is_composing: false,
        state: KeyState::Released,
        text: None,
    };
    (key, combined_mods, key_down, key_up)
}

#[cfg(target_os = "macos")]
fn has_focused_text_input(doc: &BaseDocument) -> bool {
    doc.get_focussed_node_id()
        .and_then(|id| doc.get_node(id))
        .and_then(|n| n.element_data())
        .is_some_and(|el| el.text_input_data().is_some())
}

#[cfg(target_os = "macos")]
fn macos_text_input_standard_keybinding(key: &Key, mods: Modifiers) -> Option<&'static str> {
    if mods.intersects(Modifiers::META | Modifiers::CONTROL) {
        return None;
    }
    let alt = mods.contains(Modifiers::ALT);
    let shift = mods.contains(Modifiers::SHIFT);
    match key {
        Key::Named(NamedKey::Backspace) => {
            if alt {
                Some("deleteWordBackward:")
            } else {
                Some("deleteBackward:")
            }
        }
        Key::Named(NamedKey::Delete) => {
            if alt {
                Some("deleteWordForward:")
            } else {
                Some("deleteForward:")
            }
        }
        Key::Named(NamedKey::ArrowLeft) => match (alt, shift) {
            (true, true) => Some("moveWordLeftAndModifySelection:"),
            (true, false) => Some("moveWordLeft:"),
            (false, true) => Some("moveLeftAndModifySelection:"),
            (false, false) => Some("moveLeft:"),
        },
        Key::Named(NamedKey::ArrowRight) => match (alt, shift) {
            (true, true) => Some("moveWordRightAndModifySelection:"),
            (true, false) => Some("moveWordRight:"),
            (false, true) => Some("moveRightAndModifySelection:"),
            (false, false) => Some("moveRight:"),
        },
        Key::Named(NamedKey::ArrowUp) => match (alt, shift) {
            (true, true) => Some("moveToBeginningOfParagraphAndModifySelection:"),
            (true, false) => Some("moveToBeginningOfParagraph:"),
            (false, true) => Some("moveUpAndModifySelection:"),
            (false, false) => Some("moveUp:"),
        },
        Key::Named(NamedKey::ArrowDown) => match (alt, shift) {
            (true, true) => Some("moveToEndOfParagraphAndModifySelection:"),
            (true, false) => Some("moveToEndOfParagraph:"),
            (false, true) => Some("moveDownAndModifySelection:"),
            (false, false) => Some("moveDown:"),
        },
        _ => None,
    }
}

/// Dispatch a keyboard action on a target node in `doc`.
/// Drives real Blitz DOM `EventDriver` keyboard handling, `:focus-visible` modality,
/// HTML Enter/Space activation (`#839`), Tab traversal, Escape focus clearing, and text editing.
pub fn dispatch_key(
    doc: &mut BaseDocument,
    node_id: Option<NodeId>,
    key_str: &str,
    modifiers: Modifiers,
) -> Result<NodeId, String> {
    let (key, combined_mods, key_down, key_up) = build_key_events(key_str, modifiers);
    #[cfg(not(target_os = "macos"))]
    let _ = combined_mods;

    let target_node_id = match node_id {
        Some(nid) => {
            if doc.get_focussed_node_id() != Some(nid) {
                doc.set_focus_to(nid);
            }
            nid
        }
        None => doc
            .get_focussed_node_id()
            .unwrap_or_else(|| doc.root_node().id),
    };
    let old_focus = doc.get_focussed_node_id();
    #[cfg(target_os = "macos")]
    let apple_cmd = if has_focused_text_input(doc) {
        macos_text_input_standard_keybinding(&key, combined_mods)
    } else {
        None
    };

    {
        let mut driver = EventDriver::new(doc, HostDioxusEventHandler { doc_handle: None });
        #[cfg(target_os = "macos")]
        if let Some(cmd) = apple_cmd {
            driver.handle_ui_event(UiEvent::AppleStandardKeybinding(SmolStr::new_static(cmd)));
        }
        driver.handle_ui_event(UiEvent::KeyDown(key_down));
        driver.handle_ui_event(UiEvent::KeyUp(key_up));
    }

    if key == Key::Named(NamedKey::Escape)
        && let Some(old_id) = old_focus
        && doc.get_focussed_node_id().is_none()
        && let Some((dioxus_id, _)) = find_dioxus_target(doc, old_id)
        && let Ok(runtime) = std::panic::catch_unwind(dioxus_core::Runtime::current)
    {
        let platform_event = Rc::new(PlatformEventData::new(Box::new(())));
        let dx_blur = dioxus_core::Event::new(platform_event.clone() as Rc<dyn Any>, false);
        let dx_focusout = dioxus_core::Event::new(platform_event as Rc<dyn Any>, true);
        runtime.handle_event("blur", dx_blur, dioxus_id);
        runtime.handle_event("focusout", dx_focusout, dioxus_id);
    }

    if key == Key::Named(NamedKey::Tab) {
        return Ok(doc.get_focussed_node_id().unwrap_or(target_node_id));
    }
    Ok(target_node_id)
}

/// Borrow-safe `Rc<RefCell<BaseDocument>>` variant of [`dispatch_key`].
pub fn dispatch_key_rc(
    doc_rc: &Rc<RefCell<BaseDocument>>,
    node_id: Option<NodeId>,
    key_str: &str,
    modifiers: Modifiers,
) -> Result<NodeId, String> {
    let (key, combined_mods, key_down, key_up) = build_key_events(key_str, modifiers);
    #[cfg(not(target_os = "macos"))]
    let _ = combined_mods;

    let (target_node_id, old_focus, apple_cmd) = {
        let mut doc = doc_rc.borrow_mut();
        let target_node_id = match node_id {
            Some(nid) => {
                if doc.get_focussed_node_id() != Some(nid) {
                    doc.set_focus_to(nid);
                }
                nid
            }
            None => doc
                .get_focussed_node_id()
                .unwrap_or_else(|| doc.root_node().id),
        };
        let old_focus = doc.get_focussed_node_id();
        #[cfg(target_os = "macos")]
        let apple_cmd = if has_focused_text_input(&doc) {
            macos_text_input_standard_keybinding(&key, combined_mods)
        } else {
            None
        };
        #[cfg(not(target_os = "macos"))]
        let apple_cmd: Option<&'static str> = None;
        (target_node_id, old_focus, apple_cmd)
    };

    {
        let mut doc_handle = Rc::clone(doc_rc);
        let mut driver = EventDriver::new(
            &mut doc_handle,
            HostDioxusEventHandler {
                doc_handle: Some(Rc::downgrade(doc_rc)),
            },
        );
        #[cfg(target_os = "macos")]
        if let Some(cmd) = apple_cmd {
            driver.handle_ui_event(UiEvent::AppleStandardKeybinding(SmolStr::new_static(cmd)));
        }
        #[cfg(not(target_os = "macos"))]
        let _ = apple_cmd;
        driver.handle_ui_event(UiEvent::KeyDown(key_down));
        driver.handle_ui_event(UiEvent::KeyUp(key_up));
    }

    let (escape_blur_dx, final_focus) = {
        let doc = doc_rc.borrow();
        let final_focus = doc.get_focussed_node_id();
        let escape_blur_dx = if key == Key::Named(NamedKey::Escape)
            && let Some(old_id) = old_focus
            && final_focus.is_none()
        {
            find_dioxus_target(&doc, old_id).map(|(dx, _)| dx)
        } else {
            None
        };
        (escape_blur_dx, final_focus)
    };

    if let Some(dioxus_id) = escape_blur_dx
        && let Ok(runtime) = std::panic::catch_unwind(dioxus_core::Runtime::current)
    {
        let platform_event = Rc::new(PlatformEventData::new(Box::new(())));
        let dx_blur = dioxus_core::Event::new(platform_event.clone() as Rc<dyn Any>, false);
        let dx_focusout = dioxus_core::Event::new(platform_event as Rc<dyn Any>, true);
        runtime.handle_event("blur", dx_blur, dioxus_id);
        runtime.handle_event("focusout", dx_focusout, dioxus_id);
    }

    if key == Key::Named(NamedKey::Tab) {
        return Ok(final_focus.unwrap_or(target_node_id));
    }
    Ok(target_node_id)
}

fn build_pointer_event(
    x: f32,
    y: f32,
    button: MouseEventButton,
    buttons: MouseEventButtons,
    modifiers: Modifiers,
) -> BlitzPointerEvent {
    BlitzPointerEvent {
        id: BlitzPointerId::Mouse,
        is_primary: true,
        coords: PointerCoords {
            page_x: x,
            page_y: y,
            screen_x: x,
            screen_y: y,
            client_x: x,
            client_y: y,
        },
        button,
        buttons,
        mods: modifiers,
        details: Default::default(),
        element: Default::default(),
        active_pointers: Default::default(),
    }
}

/// Dispatch a pointer/mouse move event in `doc` via Blitz `EventDriver` (honoring W3C pointer capture).
pub fn dispatch_pointer_move(
    doc: &mut BaseDocument,
    target_node_id: Option<NodeId>,
    coords: Option<(f32, f32)>,
    modifiers: Modifiers,
) -> Result<NodeId, String> {
    let (x, y) = if let Some(c) = coords {
        c
    } else if let Some(nid) = target_node_id {
        get_node_center_coords(doc, nid)
            .ok_or_else(|| format!("Target node #{:?} not found for pointer move", nid))?
    } else {
        (0.0, 0.0)
    };

    let buttons = HELD_MOUSE_BUTTONS.with(|c| c.get());
    let ptr_event = build_pointer_event(x, y, MouseEventButton::Main, buttons, modifiers);

    {
        let mut driver = EventDriver::new(doc, HostDioxusEventHandler { doc_handle: None });
        driver.handle_ui_event(UiEvent::PointerMove(ptr_event));
    }

    let resolved_nid = doc
        .get_hover_node_id()
        .or(target_node_id)
        .unwrap_or_else(|| doc.root_node().id);
    Ok(resolved_nid)
}

/// Borrow-safe `Rc<RefCell<BaseDocument>>` variant of [`dispatch_pointer_move`].
pub fn dispatch_pointer_move_rc(
    doc_rc: &Rc<RefCell<BaseDocument>>,
    target_node_id: Option<NodeId>,
    coords: Option<(f32, f32)>,
    modifiers: Modifiers,
) -> Result<NodeId, String> {
    let (x, y) = {
        let doc = doc_rc.borrow();
        if let Some(c) = coords {
            c
        } else if let Some(nid) = target_node_id {
            get_node_center_coords(&doc, nid)
                .ok_or_else(|| format!("Target node #{:?} not found for pointer move", nid))?
        } else {
            (0.0, 0.0)
        }
    };

    let buttons = HELD_MOUSE_BUTTONS.with(|c| c.get());
    let ptr_event = build_pointer_event(x, y, MouseEventButton::Main, buttons, modifiers);

    {
        let mut doc_handle = Rc::clone(doc_rc);
        let mut driver = EventDriver::new(
            &mut doc_handle,
            HostDioxusEventHandler {
                doc_handle: Some(Rc::downgrade(doc_rc)),
            },
        );
        driver.handle_ui_event(UiEvent::PointerMove(ptr_event));
    }

    let doc = doc_rc.borrow();
    let resolved_nid = doc
        .get_hover_node_id()
        .or(target_node_id)
        .unwrap_or_else(|| doc.root_node().id);
    Ok(resolved_nid)
}

/// Dispatch a pointer/mouse down event on a target node or coordinates in `doc` via Blitz `EventDriver`.
pub fn dispatch_pointer_down(
    doc: &mut BaseDocument,
    target_node_id: Option<NodeId>,
    coords: Option<(f32, f32)>,
    button: MouseEventButton,
    modifiers: Modifiers,
) -> Result<NodeId, String> {
    let (x, y) = if let Some(c) = coords {
        c
    } else if let Some(nid) = target_node_id {
        get_node_center_coords(doc, nid)
            .ok_or_else(|| format!("Target node #{:?} not found for pointer down", nid))?
    } else if let Some(nid) = doc.get_hover_node_id() {
        get_node_center_coords(doc, nid).unwrap_or((0.0, 0.0))
    } else {
        (0.0, 0.0)
    };

    let buttons = HELD_MOUSE_BUTTONS.with(|c| {
        let next = c.get() | MouseEventButtons::from(button);
        c.set(next);
        next
    });
    let ptr_event = build_pointer_event(x, y, button, buttons, modifiers);

    {
        let mut driver = EventDriver::new(doc, HostDioxusEventHandler { doc_handle: None });
        driver.handle_ui_event(UiEvent::PointerDown(ptr_event));
    }

    let resolved_nid = target_node_id
        .or(doc.get_hover_node_id())
        .unwrap_or_else(|| doc.root_node().id);
    Ok(resolved_nid)
}

/// Borrow-safe `Rc<RefCell<BaseDocument>>` variant of [`dispatch_pointer_down`].
pub fn dispatch_pointer_down_rc(
    doc_rc: &Rc<RefCell<BaseDocument>>,
    target_node_id: Option<NodeId>,
    coords: Option<(f32, f32)>,
    button: MouseEventButton,
    modifiers: Modifiers,
) -> Result<NodeId, String> {
    let (x, y) = {
        let doc = doc_rc.borrow();
        if let Some(c) = coords {
            c
        } else if let Some(nid) = target_node_id {
            get_node_center_coords(&doc, nid)
                .ok_or_else(|| format!("Target node #{:?} not found for pointer down", nid))?
        } else if let Some(nid) = doc.get_hover_node_id() {
            get_node_center_coords(&doc, nid).unwrap_or((0.0, 0.0))
        } else {
            (0.0, 0.0)
        }
    };

    let buttons = HELD_MOUSE_BUTTONS.with(|c| {
        let next = c.get() | MouseEventButtons::from(button);
        c.set(next);
        next
    });
    let ptr_event = build_pointer_event(x, y, button, buttons, modifiers);

    {
        let mut doc_handle = Rc::clone(doc_rc);
        let mut driver = EventDriver::new(
            &mut doc_handle,
            HostDioxusEventHandler {
                doc_handle: Some(Rc::downgrade(doc_rc)),
            },
        );
        driver.handle_ui_event(UiEvent::PointerDown(ptr_event));
    }

    let doc = doc_rc.borrow();
    let resolved_nid = target_node_id
        .or(doc.get_hover_node_id())
        .unwrap_or_else(|| doc.root_node().id);
    Ok(resolved_nid)
}

/// Dispatch a pointer/mouse up event on a target node or coordinates in `doc` via Blitz `EventDriver`.
pub fn dispatch_pointer_up(
    doc: &mut BaseDocument,
    target_node_id: Option<NodeId>,
    coords: Option<(f32, f32)>,
    button: MouseEventButton,
    modifiers: Modifiers,
) -> Result<NodeId, String> {
    let (x, y) = if let Some(c) = coords {
        c
    } else if let Some(nid) = target_node_id {
        get_node_center_coords(doc, nid)
            .ok_or_else(|| format!("Target node #{:?} not found for pointer up", nid))?
    } else if let Some(nid) = doc.get_hover_node_id() {
        get_node_center_coords(doc, nid).unwrap_or((0.0, 0.0))
    } else {
        (0.0, 0.0)
    };

    let buttons = HELD_MOUSE_BUTTONS.with(|c| {
        let next = c.get() & !MouseEventButtons::from(button);
        c.set(next);
        next
    });
    let ptr_event = build_pointer_event(x, y, button, buttons, modifiers);

    {
        let mut driver = EventDriver::new(doc, HostDioxusEventHandler { doc_handle: None });
        driver.handle_ui_event(UiEvent::PointerUp(ptr_event));
    }

    let resolved_nid = target_node_id
        .or(doc.get_hover_node_id())
        .unwrap_or_else(|| doc.root_node().id);
    Ok(resolved_nid)
}

/// Borrow-safe `Rc<RefCell<BaseDocument>>` variant of [`dispatch_pointer_up`].
pub fn dispatch_pointer_up_rc(
    doc_rc: &Rc<RefCell<BaseDocument>>,
    target_node_id: Option<NodeId>,
    coords: Option<(f32, f32)>,
    button: MouseEventButton,
    modifiers: Modifiers,
) -> Result<NodeId, String> {
    let (x, y) = {
        let doc = doc_rc.borrow();
        if let Some(c) = coords {
            c
        } else if let Some(nid) = target_node_id {
            get_node_center_coords(&doc, nid)
                .ok_or_else(|| format!("Target node #{:?} not found for pointer up", nid))?
        } else if let Some(nid) = doc.get_hover_node_id() {
            get_node_center_coords(&doc, nid).unwrap_or((0.0, 0.0))
        } else {
            (0.0, 0.0)
        }
    };

    let buttons = HELD_MOUSE_BUTTONS.with(|c| {
        let next = c.get() & !MouseEventButtons::from(button);
        c.set(next);
        next
    });
    let ptr_event = build_pointer_event(x, y, button, buttons, modifiers);

    {
        let mut doc_handle = Rc::clone(doc_rc);
        let mut driver = EventDriver::new(
            &mut doc_handle,
            HostDioxusEventHandler {
                doc_handle: Some(Rc::downgrade(doc_rc)),
            },
        );
        driver.handle_ui_event(UiEvent::PointerUp(ptr_event));
    }

    let doc = doc_rc.borrow();
    let resolved_nid = target_node_id
        .or(doc.get_hover_node_id())
        .unwrap_or_else(|| doc.root_node().id);
    Ok(resolved_nid)
}

/// Dispatch a mouse wheel / scroll event in `doc`.
/// Drives Blitz DOM scroll container propagation and delivers Dioxus `wheel` events.
pub fn dispatch_wheel(
    doc: &mut BaseDocument,
    target_node_id: Option<NodeId>,
    coords: Option<(f32, f32)>,
    delta_x: f64,
    delta_y: f64,
    modifiers: Modifiers,
) -> Result<NodeId, String> {
    let (x, y) = if let Some(c) = coords {
        c
    } else if let Some(nid) = target_node_id {
        get_node_center_coords(doc, nid)
            .ok_or_else(|| format!("Target node #{:?} not found for wheel", nid))?
    } else if let Some(nid) = doc.get_hover_node_id() {
        get_node_center_coords(doc, nid).unwrap_or((0.0, 0.0))
    } else {
        (0.0, 0.0)
    };

    let _ = doc.set_hover_to(x, y);

    let resolved_nid = doc
        .get_hover_node_id()
        .or(target_node_id)
        .unwrap_or_else(|| doc.root_node().id);

    let wheel_event = BlitzWheelEvent {
        delta: BlitzWheelDelta::Pixels(delta_x, delta_y),
        phase: BlitzWheelPhase::Moved,
        source: BlitzWheelSource::Wheel,
        coords: PointerCoords {
            page_x: x,
            page_y: y,
            screen_x: x,
            screen_y: y,
            client_x: x,
            client_y: y,
        },
        buttons: HELD_MOUSE_BUTTONS.with(|c| c.get()),
        mods: modifiers,
        element: Default::default(),
    };

    doc.handle_ui_event(UiEvent::Wheel(wheel_event));

    if let Some((dioxus_id, _target_actual_id)) = find_dioxus_target(doc, resolved_nid)
        && let Ok(runtime) = std::panic::catch_unwind(dioxus_core::Runtime::current)
    {
        let platform_wheel = if let Some(node) = doc.get_node(resolved_nid) {
            Rc::new(PlatformEventData::new(click_event_payload(node, modifiers)))
        } else {
            Rc::new(PlatformEventData::new(Box::new(())))
        };
        let dx_wheel = dioxus_core::Event::new(platform_wheel as Rc<dyn Any>, true);
        runtime.handle_event("wheel", dx_wheel, dioxus_id);
    }

    Ok(resolved_nid)
}

/// Borrow-safe `Rc<RefCell<BaseDocument>>` variant of [`dispatch_wheel`].
pub fn dispatch_wheel_rc(
    doc_rc: &Rc<RefCell<BaseDocument>>,
    target_node_id: Option<NodeId>,
    coords: Option<(f32, f32)>,
    delta_x: f64,
    delta_y: f64,
    modifiers: Modifiers,
) -> Result<NodeId, String> {
    let (resolved_nid, wheel_dispatch) = {
        let mut doc = doc_rc.borrow_mut();
        let (x, y) = if let Some(c) = coords {
            c
        } else if let Some(nid) = target_node_id {
            get_node_center_coords(&doc, nid)
                .ok_or_else(|| format!("Target node #{:?} not found for wheel", nid))?
        } else if let Some(nid) = doc.get_hover_node_id() {
            get_node_center_coords(&doc, nid).unwrap_or((0.0, 0.0))
        } else {
            (0.0, 0.0)
        };

        let _ = doc.set_hover_to(x, y);

        let resolved_nid = doc
            .get_hover_node_id()
            .or(target_node_id)
            .unwrap_or_else(|| doc.root_node().id);

        let wheel_event = BlitzWheelEvent {
            delta: BlitzWheelDelta::Pixels(delta_x, delta_y),
            phase: BlitzWheelPhase::Moved,
            source: BlitzWheelSource::Wheel,
            coords: PointerCoords {
                page_x: x,
                page_y: y,
                screen_x: x,
                screen_y: y,
                client_x: x,
                client_y: y,
            },
            buttons: HELD_MOUSE_BUTTONS.with(|c| c.get()),
            mods: modifiers,
            element: Default::default(),
        };

        doc.handle_ui_event(UiEvent::Wheel(wheel_event));

        let wheel_dispatch = find_dioxus_target(&doc, resolved_nid).map(|(dioxus_id, _)| {
            let platform_wheel = if let Some(node) = doc.get_node(resolved_nid) {
                Rc::new(PlatformEventData::new(click_event_payload(node, modifiers)))
            } else {
                Rc::new(PlatformEventData::new(Box::new(())))
            };
            (dioxus_id, platform_wheel)
        });
        (resolved_nid, wheel_dispatch)
    };

    if let Some((dioxus_id, platform_wheel)) = wheel_dispatch
        && let Ok(runtime) = std::panic::catch_unwind(dioxus_core::Runtime::current)
    {
        let dx_wheel = dioxus_core::Event::new(platform_wheel as Rc<dyn Any>, true);
        runtime.handle_event("wheel", dx_wheel, dioxus_id);
    }

    Ok(resolved_nid)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn dispatch_scroll_into_view_moves_viewport_and_rejects_missing_node() {
        use blitz_dom::{Attribute, DocumentConfig, QualName, local_name, ns};
        use blitz_traits::shell::{ColorScheme, Viewport};

        let mut doc = BaseDocument::new(DocumentConfig {
            viewport: Some(Viewport::new(800, 600, 1.0, ColorScheme::Light)),
            ..Default::default()
        });
        let root_id = doc.root_node().id;
        let target = {
            let mut m = doc.mutate();
            let mk = |m: &mut blitz_dom::DocumentMutator<'_>, style: &str| {
                m.create_element(
                    QualName::new(None, ns!(html), local_name!("div")),
                    vec![Attribute {
                        name: QualName::new(None, ns!(), local_name!("style")),
                        value: style.to_string(),
                    }],
                )
            };
            let spacer = mk(&mut m, "display:block;height:1000px;");
            let target = mk(&mut m, "display:block;height:80px;");
            let html =
                m.create_element(QualName::new(None, ns!(html), local_name!("html")), vec![]);
            let body =
                m.create_element(QualName::new(None, ns!(html), local_name!("body")), vec![]);
            m.append_children(root_id, &[html]);
            m.append_children(html, &[body]);
            m.append_children(body, &[spacer, target]);
            target
        };
        doc.resolve(0.0);
        assert_eq!(doc.viewport_scroll().y, 0.0);

        assert!(dispatch_scroll_into_view(&mut doc, target));
        assert!(doc.viewport_scroll().y > 0.0);

        assert!(!dispatch_scroll_into_view(
            &mut doc,
            NodeId::from_u64(u32::MAX as u64 - 1)
        ));
    }

    #[test]
    fn test_parse_key_str() {
        let (k, c, m) = parse_key_str("ctrl+shift+Tab");
        assert_eq!(k, Key::Named(NamedKey::Tab));
        assert_eq!(c, Code::Tab);
        assert!(m.contains(Modifiers::CONTROL));
        assert!(m.contains(Modifiers::SHIFT));

        let (k, c, m) = parse_key_str("cmd+Enter");
        assert_eq!(k, Key::Named(NamedKey::Enter));
        assert_eq!(c, Code::Enter);
        assert!(m.contains(Modifiers::META));

        let (k, c, m) = parse_key_str("Escape");
        assert_eq!(k, Key::Named(NamedKey::Escape));
        assert_eq!(c, Code::Escape);
        assert!(m.is_empty());

        let (k, c, m) = parse_key_str(" ");
        assert_eq!(k, Key::Character(" ".into()));
        assert_eq!(c, Code::Space);
        assert!(m.is_empty());
    }

    #[test]
    fn dispatch_key_html_activation_and_prevent_default_contract() {
        use std::sync::atomic::{AtomicUsize, Ordering};

        static BTN_CLICKS: AtomicUsize = AtomicUsize::new(0);
        static PREVENTED_KEYDOWN: AtomicUsize = AtomicUsize::new(0);
        static PREVENTED_CLICKS: AtomicUsize = AtomicUsize::new(0);
        static ROLE_BTN_CLICKS: AtomicUsize = AtomicUsize::new(0);
        static CHECKBOX_CLICKS: AtomicUsize = AtomicUsize::new(0);
        static LINK_CLICKS: AtomicUsize = AtomicUsize::new(0);
        static FS_BLOCKED_CLICKS: AtomicUsize = AtomicUsize::new(0);
        static FS_LEGEND_CLICKS: AtomicUsize = AtomicUsize::new(0);

        BTN_CLICKS.store(0, Ordering::SeqCst);
        PREVENTED_KEYDOWN.store(0, Ordering::SeqCst);
        PREVENTED_CLICKS.store(0, Ordering::SeqCst);
        ROLE_BTN_CLICKS.store(0, Ordering::SeqCst);
        CHECKBOX_CLICKS.store(0, Ordering::SeqCst);
        LINK_CLICKS.store(0, Ordering::SeqCst);
        FS_BLOCKED_CLICKS.store(0, Ordering::SeqCst);
        FS_LEGEND_CLICKS.store(0, Ordering::SeqCst);

        #[component]
        fn TestApp() -> Element {
            rsx! {
                div {
                    button {
                        id: "plain-btn",
                        onclick: move |_| { BTN_CLICKS.fetch_add(1, Ordering::SeqCst); },
                        "Plain"
                    }
                    button {
                        id: "prevent-btn",
                        onkeydown: move |evt| {
                            if evt.key() == Key::Named(NamedKey::Enter)
                                || evt.key() == Key::Character(" ".into())
                            {
                                evt.prevent_default();
                                PREVENTED_KEYDOWN.fetch_add(1, Ordering::SeqCst);
                            }
                        },
                        onclick: move |_| { PREVENTED_CLICKS.fetch_add(1, Ordering::SeqCst); },
                        "Pattern B"
                    }
                    div {
                        id: "role-btn",
                        role: "button",
                        tabindex: "0",
                        onclick: move |_| { ROLE_BTN_CLICKS.fetch_add(1, Ordering::SeqCst); },
                        "ARIA pseudo-button"
                    }
                    input {
                        id: "chk",
                        r#type: "checkbox",
                        onclick: move |_| { CHECKBOX_CLICKS.fetch_add(1, Ordering::SeqCst); },
                    }
                    a {
                        id: "lnk",
                        href: "#target",
                        onclick: move |_| { LINK_CLICKS.fetch_add(1, Ordering::SeqCst); },
                        "Link"
                    }
                    fieldset {
                        disabled: true,
                        legend {
                            button {
                                id: "legend-btn",
                                onclick: move |_| { FS_LEGEND_CLICKS.fetch_add(1, Ordering::SeqCst); },
                                "Legend Button"
                            }
                        }
                        button {
                            id: "blocked-btn",
                            onclick: move |_| { FS_BLOCKED_CLICKS.fetch_add(1, Ordering::SeqCst); },
                            "Blocked Button"
                        }
                    }
                }
            }
        }

        let vdom = dioxus_core::VirtualDom::new(TestApp);
        let mut dx_doc =
            dioxus_native::DioxusDocument::new(vdom, blitz_dom::DocumentConfig::default());
        dx_doc.initial_build();
        dx_doc.inner_mut().resolve(0.0);

        let find_id = |doc: &dioxus_native::DioxusDocument, sel: &str| -> NodeId {
            doc.inner().query_selector(sel).unwrap().expect(sel)
        };

        let plain_btn = find_id(&dx_doc, "#plain-btn");
        let prevent_btn = find_id(&dx_doc, "#prevent-btn");
        let role_btn = find_id(&dx_doc, "#role-btn");
        let chk = find_id(&dx_doc, "#chk");
        let lnk = find_id(&dx_doc, "#lnk");
        let legend_btn = find_id(&dx_doc, "#legend-btn");
        let blocked_btn = find_id(&dx_doc, "#blocked-btn");

        let send_key = |dx_doc: &mut dioxus_native::DioxusDocument, id: NodeId, key: &str| {
            let runtime = dx_doc.vdom.runtime();
            let _guard = dioxus_core::RuntimeGuard::new(Rc::clone(&runtime));
            let mut inner = dx_doc.inner_mut();
            dispatch_key(&mut inner, Some(id), key, Modifiers::empty()).unwrap();
        };

        // 1. Plain <button>: Enter and Space each fire onclick once
        send_key(&mut dx_doc, plain_btn, "Enter");
        assert_eq!(BTN_CLICKS.load(Ordering::SeqCst), 1);
        send_key(&mut dx_doc, plain_btn, " ");
        assert_eq!(BTN_CLICKS.load(Ordering::SeqCst), 2);

        // 2. Pattern B <button> calling evt.prevent_default(): onkeydown runs, onclick NEVER double-fires
        send_key(&mut dx_doc, prevent_btn, "Enter");
        send_key(&mut dx_doc, prevent_btn, " ");
        assert_eq!(PREVENTED_KEYDOWN.load(Ordering::SeqCst), 2);
        assert_eq!(PREVENTED_CLICKS.load(Ordering::SeqCst), 0);

        // 3. <div role="button"> is NOT auto-clicked by host layer
        send_key(&mut dx_doc, role_btn, "Enter");
        send_key(&mut dx_doc, role_btn, " ");
        assert_eq!(ROLE_BTN_CLICKS.load(Ordering::SeqCst), 0);

        // 4. <input type="checkbox">: Space clicks, Enter does NOT
        send_key(&mut dx_doc, chk, "Enter");
        assert_eq!(CHECKBOX_CLICKS.load(Ordering::SeqCst), 0);
        send_key(&mut dx_doc, chk, " ");
        assert_eq!(CHECKBOX_CLICKS.load(Ordering::SeqCst), 1);

        // 5. <a href>: Enter clicks, Space does NOT
        send_key(&mut dx_doc, lnk, " ");
        assert_eq!(LINK_CLICKS.load(Ordering::SeqCst), 0);
        send_key(&mut dx_doc, lnk, "Enter");
        assert_eq!(LINK_CLICKS.load(Ordering::SeqCst), 1);

        // 6. <fieldset disabled>: first-<legend> descendant allowed, body descendant blocked
        send_key(&mut dx_doc, blocked_btn, "Enter");
        send_key(&mut dx_doc, blocked_btn, " ");
        assert_eq!(FS_BLOCKED_CLICKS.load(Ordering::SeqCst), 0);
        send_key(&mut dx_doc, legend_btn, "Enter");
        assert_eq!(FS_LEGEND_CLICKS.load(Ordering::SeqCst), 1);
    }

    #[test]
    fn dispatch_pointer_capture_out_of_bounds_drag_and_lostpointercapture_contract() {
        use std::sync::atomic::{AtomicUsize, Ordering};

        static GOT_CAPTURE: AtomicUsize = AtomicUsize::new(0);
        static MOVE_COUNT: AtomicUsize = AtomicUsize::new(0);
        static UP_COUNT: AtomicUsize = AtomicUsize::new(0);
        static LOST_CAPTURE: AtomicUsize = AtomicUsize::new(0);
        static OUTSIDE_MOVE_COUNT: AtomicUsize = AtomicUsize::new(0);

        GOT_CAPTURE.store(0, Ordering::SeqCst);
        MOVE_COUNT.store(0, Ordering::SeqCst);
        UP_COUNT.store(0, Ordering::SeqCst);
        LOST_CAPTURE.store(0, Ordering::SeqCst);
        OUTSIDE_MOVE_COUNT.store(0, Ordering::SeqCst);

        std::thread_local! {
            static MOUNTED_SLIDER: RefCell<Option<dioxus_native::NodeHandle>> = const { RefCell::new(None) };
        }
        MOUNTED_SLIDER.with(|cell| *cell.borrow_mut() = None);

        #[component]
        fn CaptureApp() -> Element {
            rsx! {
                div {
                    style: "width: 800px; height: 600px; position: relative;",
                    div {
                        id: "capture-box",
                        style: "position: absolute; left: 10px; top: 10px; width: 100px; height: 40px;",
                        onmounted: move |evt| {
                            if let Some(h) = evt.downcast::<dioxus_native::NodeHandle>() {
                                MOUNTED_SLIDER.with(|cell| *cell.borrow_mut() = Some(h.clone()));
                            }
                        },
                        onpointerdown: move |evt: Event<PointerData>| {
                            let pid = evt.data().pointer_id();
                            MOUNTED_SLIDER.with(|cell| {
                                if let Some(h) = cell.borrow().as_ref() {
                                    let _ = h.set_pointer_capture(pid);
                                }
                            });
                        },
                        ongotpointercapture: move |_| {
                            GOT_CAPTURE.fetch_add(1, Ordering::SeqCst);
                        },
                        onpointermove: move |_| {
                            MOVE_COUNT.fetch_add(1, Ordering::SeqCst);
                        },
                        onpointerup: move |_| {
                            UP_COUNT.fetch_add(1, Ordering::SeqCst);
                        },
                        onlostpointercapture: move |_| {
                            LOST_CAPTURE.fetch_add(1, Ordering::SeqCst);
                        },
                        "Drag"
                    }
                    div {
                        id: "outside-zone",
                        style: "position: absolute; left: 400px; top: 300px; width: 200px; height: 200px;",
                        onpointermove: move |_| {
                            OUTSIDE_MOVE_COUNT.fetch_add(1, Ordering::SeqCst);
                        },
                        "Outside"
                    }
                }
            }
        }

        let vdom = dioxus_core::VirtualDom::new(CaptureApp);
        let mut dx_doc = dioxus_native::DioxusDocument::new(
            vdom,
            blitz_dom::DocumentConfig {
                viewport: Some(blitz_traits::shell::Viewport::new(
                    800,
                    600,
                    1.0,
                    Default::default(),
                )),
                ..Default::default()
            },
        );
        dx_doc.initial_build();
        dx_doc.inner_mut().resolve(0.0);

        let handle = MOUNTED_SLIDER
            .with(|cell| cell.borrow().clone())
            .expect("capture-box must mount");
        let doc_rc = handle.base_document();
        let capture_nid = handle.node_id();

        let runtime = dx_doc.vdom.runtime();
        let _guard = dioxus_core::RuntimeGuard::new(Rc::clone(&runtime));

        // 1. Pointer down inside #capture-box -> queues set_pointer_capture(1) without RefCell panic
        dispatch_pointer_down_rc(
            &doc_rc,
            Some(capture_nid),
            None,
            MouseEventButton::Main,
            Modifiers::empty(),
        )
        .unwrap();
        assert!(handle.has_pointer_capture(1));

        // 2. Pointer move far outside #capture-box bounds (over #outside-zone at 500, 400)
        dispatch_pointer_move_rc(&doc_rc, None, Some((500.0, 400.0)), Modifiers::empty()).unwrap();
        assert_eq!(GOT_CAPTURE.load(Ordering::SeqCst), 1);
        assert_eq!(MOVE_COUNT.load(Ordering::SeqCst), 1);
        assert_eq!(OUTSIDE_MOVE_COUNT.load(Ordering::SeqCst), 0);

        // 3. Pointer up outside bounds -> delivered to #capture-box + implicit release + lostpointercapture
        dispatch_pointer_up_rc(
            &doc_rc,
            None,
            Some((500.0, 400.0)),
            MouseEventButton::Main,
            Modifiers::empty(),
        )
        .unwrap();
        assert_eq!(UP_COUNT.load(Ordering::SeqCst), 1);
        assert_eq!(LOST_CAPTURE.load(Ordering::SeqCst), 1);
        assert!(!handle.has_pointer_capture(1));
    }

    #[test]
    fn dispatch_focus_visible_keyboard_vs_pointer_and_focus_options_contract() {
        #[component]
        fn FocusApp() -> Element {
            rsx! {
                div {
                    style: "width: 400px; height: 200px;",
                    button { id: "btn-a", "Button A" }
                    button { id: "btn-b", "Button B" }
                }
            }
        }

        let vdom = dioxus_core::VirtualDom::new(FocusApp);
        let mut dx_doc = dioxus_native::DioxusDocument::new(
            vdom,
            blitz_dom::DocumentConfig {
                viewport: Some(blitz_traits::shell::Viewport::new(
                    400,
                    200,
                    1.0,
                    Default::default(),
                )),
                ..Default::default()
            },
        );
        dx_doc.initial_build();
        dx_doc.inner_mut().resolve(0.0);

        let doc_rc = Rc::clone(&dx_doc.inner);
        let (btn_a, btn_b) = {
            let doc = doc_rc.borrow();
            (
                doc.query_selector("#btn-a").unwrap().unwrap(),
                doc.query_selector("#btn-b").unwrap().unwrap(),
            )
        };

        let runtime = dx_doc.vdom.runtime();
        let _guard = dioxus_core::RuntimeGuard::new(Rc::clone(&runtime));

        // 1. Tab focuses #btn-a with keyboard modality -> focus_visible == true
        let focused = dispatch_key_rc(&doc_rc, None, "Tab", Modifiers::empty()).unwrap();
        assert_eq!(focused, btn_a);
        assert!(doc_rc.borrow().is_focus_visible(btn_a));

        // 2. Enter & Space activation on #btn-a preserve keyboard modality -> focus_visible == true
        dispatch_key_rc(&doc_rc, None, "Enter", Modifiers::empty()).unwrap();
        assert!(doc_rc.borrow().is_focus_visible(btn_a));
        dispatch_key_rc(&doc_rc, None, " ", Modifiers::empty()).unwrap();
        assert!(doc_rc.borrow().is_focus_visible(btn_a));

        // 3. Pointer click on #btn-b focuses #btn-b with pointer modality -> focus_visible == false
        assert!(dispatch_click_rc(&doc_rc, btn_b, Modifiers::empty()));
        assert_eq!(doc_rc.borrow().get_focussed_node_id(), Some(btn_b));
        assert!(!doc_rc.borrow().is_focus_visible(btn_b));

        // 4. Programmatic focus with explicit focus_visible: Some(true) / Some(false)
        assert!(dispatch_focus_rc(&doc_rc, btn_a, Some(true)));
        assert!(doc_rc.borrow().is_focus_visible(btn_a));
        assert!(dispatch_focus_rc(&doc_rc, btn_a, Some(false)));
        assert!(!doc_rc.borrow().is_focus_visible(btn_a));
    }

    #[test]
    fn dispatch_key_text_input_navigation_and_editing_single_step_contract() {
        #[component]
        fn InputApp() -> Element {
            let mut text = use_signal(|| String::from("abcd"));
            rsx! {
                div {
                    style: "width: 400px; height: 100px;",
                    input {
                        id: "txt",
                        r#type: "text",
                        value: "{text}",
                        oninput: move |evt| text.set(evt.value()),
                    }
                }
            }
        }

        let vdom = dioxus_core::VirtualDom::new(InputApp);
        let mut dx_doc = dioxus_native::DioxusDocument::new(
            vdom,
            blitz_dom::DocumentConfig {
                viewport: Some(blitz_traits::shell::Viewport::new(
                    400,
                    100,
                    1.0,
                    Default::default(),
                )),
                ..Default::default()
            },
        );
        dx_doc.initial_build();
        dx_doc.inner_mut().resolve(0.0);

        let doc_rc = Rc::clone(&dx_doc.inner);
        let input_id = doc_rc.borrow().query_selector("#txt").unwrap().unwrap();

        let read_editor_text = |doc_rc: &Rc<RefCell<BaseDocument>>| -> String {
            doc_rc
                .borrow()
                .get_node(input_id)
                .unwrap()
                .element_data()
                .unwrap()
                .text_input_data()
                .unwrap()
                .editor
                .text()
                .to_string()
        };

        let runtime = dx_doc.vdom.runtime();
        let _guard = dioxus_core::RuntimeGuard::new(Rc::clone(&runtime));

        // Move caret to end of "abcd"
        #[cfg(target_os = "macos")]
        dispatch_key_rc(
            &doc_rc,
            Some(input_id),
            "cmd+ArrowRight",
            Modifiers::empty(),
        )
        .unwrap();
        #[cfg(not(target_os = "macos"))]
        dispatch_key_rc(&doc_rc, Some(input_id), "End", Modifiers::empty()).unwrap();

        // Backspace deletes exactly 1 character ("abc")
        dispatch_key_rc(&doc_rc, None, "Backspace", Modifiers::empty()).unwrap();
        assert_eq!(read_editor_text(&doc_rc), "abc");

        // ArrowLeft moves 1 character left (between 'b' and 'c'), typing 'X' yields "abXc"
        dispatch_key_rc(&doc_rc, None, "ArrowLeft", Modifiers::empty()).unwrap();
        dispatch_key_rc(&doc_rc, None, "X", Modifiers::empty()).unwrap();
        assert_eq!(read_editor_text(&doc_rc), "abXc");

        // Delete removes 'c' -> "abX"
        dispatch_key_rc(&doc_rc, None, "Delete", Modifiers::empty()).unwrap();
        assert_eq!(read_editor_text(&doc_rc), "abX");
    }
}
