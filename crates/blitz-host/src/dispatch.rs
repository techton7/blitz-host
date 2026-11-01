//! Host Action Dispatching for Blitz Host
//!
//! Provides internal host action dispatching that directly drives
//! Blitz DOM's standard [`BaseDocument::handle_ui_event`] event pipeline
//! and delivers corresponding events to the active Dioxus runtime.

use std::any::Any;
use std::rc::Rc;
use std::str::FromStr;

use blitz_dom::{BaseDocument, Document, Node, NodeId};
use blitz_traits::SmolStr;
pub use blitz_traits::events::MouseEventButton;
use blitz_traits::events::{
    BlitzKeyEvent, BlitzPointerEvent, BlitzPointerId, BlitzWheelDelta, BlitzWheelEvent, KeyState,
    MouseEventButtons, PointerCoords, UiEvent,
};
use dioxus_native::prelude::*;
use ::keyboard_types::{Code, Key, Location, Modifiers};

/// Extract Dioxus [`dioxus_core::ElementId`] from node's "data-dioxus-id" attribute.
pub fn get_dioxus_id(node: &Node) -> Option<dioxus_core::ElementId> {
    node.element_data()?
        .attrs
        .iter()
        .find(|attr| *attr.name.local == *"data-dioxus-id")
        .and_then(|attr| attr.value.parse::<usize>().ok())
        .map(dioxus_core::ElementId)
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
    let mut mods = Modifiers::empty();
    let mut key_part = raw.trim();

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
        key_part = key_part[idx + 1..].trim();
    }

    let (key, code) = if key_part.eq_ignore_ascii_case("tab") {
        (Key::Tab, Code::Tab)
    } else if key_part.eq_ignore_ascii_case("enter") || key_part.eq_ignore_ascii_case("return") {
        (Key::Enter, Code::Enter)
    } else if key_part.eq_ignore_ascii_case("space") || key_part == " " {
        (Key::Character(" ".into()), Code::Space)
    } else if key_part.eq_ignore_ascii_case("escape") || key_part.eq_ignore_ascii_case("esc") {
        (Key::Escape, Code::Escape)
    } else if key_part.eq_ignore_ascii_case("backspace") {
        (Key::Backspace, Code::Backspace)
    } else if key_part.eq_ignore_ascii_case("delete") || key_part.eq_ignore_ascii_case("del") {
        (Key::Delete, Code::Delete)
    } else if key_part.eq_ignore_ascii_case("arrowleft") || key_part.eq_ignore_ascii_case("left") {
        (Key::ArrowLeft, Code::ArrowLeft)
    } else if key_part.eq_ignore_ascii_case("arrowright") || key_part.eq_ignore_ascii_case("right")
    {
        (Key::ArrowRight, Code::ArrowRight)
    } else if key_part.eq_ignore_ascii_case("arrowup") || key_part.eq_ignore_ascii_case("up") {
        (Key::ArrowUp, Code::ArrowUp)
    } else if key_part.eq_ignore_ascii_case("arrowdown") || key_part.eq_ignore_ascii_case("down") {
        (Key::ArrowDown, Code::ArrowDown)
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
            Key::from_str(key_part).unwrap_or(Key::Unidentified),
            Code::from_str(key_part).unwrap_or(Code::Unidentified),
        )
    };

    (key, code, mods)
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

/// Dispatch a focus event on a target node in `doc`.
/// Sets focus on the Blitz DOM node and dispatches "focus" and "focusin" events to Dioxus listeners.
pub fn dispatch_focus(doc: &mut BaseDocument, node_id: NodeId) -> bool {
    let Some((dioxus_id, target_node_id)) = find_dioxus_target(doc, node_id) else {
        return false;
    };

    // Update Blitz DOM focus state
    let _ = doc.set_focus_to(target_node_id);

    // Dispatch Dioxus focus events
    let platform_event = Rc::new(PlatformEventData::new(Box::new(())));
    let dx_focus = dioxus_core::Event::new(platform_event.clone() as Rc<dyn Any>, false);

    if let Ok(runtime) = std::panic::catch_unwind(dioxus_core::Runtime::current) {
        runtime.handle_event("focus", dx_focus, dioxus_id);
        let dx_focusin = dioxus_core::Event::new(platform_event as Rc<dyn Any>, true);
        runtime.handle_event("focusin", dx_focusin, dioxus_id);
        true
    } else {
        true
    }
}

/// Dispatch an input/value change on a target node in `doc`.
/// Updates the Blitz DOM text input editor (if applicable) and dispatches "input" and "change"
/// events containing the new value to Dioxus listeners.
pub fn dispatch_input(doc: &mut BaseDocument, node_id: NodeId, value: &str) -> bool {
    let Some((dioxus_id, target_node_id)) = find_dioxus_target(doc, node_id) else {
        return false;
    };

    // If target node is a text input, update its editor buffer and refresh layout
    doc.set_text_input_value(target_node_id, value);

    // Dispatch Dioxus input and change events
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

/// Dispatch a keyboard action on a target node in `doc`.
/// Drives real Blitz DOM keyboard handling, Dioxus VirtualDom keyboard events,
/// and reactive updates for Focus/Activation, Text Editing, Navigation, and Modifiers.
pub fn dispatch_key(
    doc: &mut BaseDocument,
    node_id: Option<NodeId>,
    key_str: &str,
    modifiers: Modifiers,
) -> Result<NodeId, String> {
    let (key, code, parsed_mods) = parse_key_str(key_str);
    let combined_mods = modifiers | parsed_mods;

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

    // 1. Handle Tab / Shift+Tab focus traversal
    if key == Key::Tab {
        let old_focus = doc.get_focussed_node_id();
        let new_focus = if combined_mods.contains(Modifiers::SHIFT) {
            doc.focus_prev_node()
        } else {
            doc.focus_next_node()
        };

        if let Some(new_id) = new_focus {
            if let Some(old_id) = old_focus
                && old_id != new_id
                && let Some(node) = doc.get_node(old_id)
                && let Some(dioxus_id) = get_dioxus_id(node)
            {
                let platform_event = Rc::new(PlatformEventData::new(Box::new(())));
                let dx_blur = dioxus_core::Event::new(platform_event as Rc<dyn Any>, false);
                if let Ok(runtime) = std::panic::catch_unwind(dioxus_core::Runtime::current) {
                    runtime.handle_event("blur", dx_blur, dioxus_id);
                }
            }

            if let Some(node) = doc.get_node(new_id)
                && let Some(dioxus_id) = get_dioxus_id(node)
            {
                let platform_event = Rc::new(PlatformEventData::new(Box::new(())));
                let dx_focus =
                    dioxus_core::Event::new(platform_event.clone() as Rc<dyn Any>, false);
                let dx_focusin = dioxus_core::Event::new(platform_event as Rc<dyn Any>, true);
                if let Ok(runtime) = std::panic::catch_unwind(dioxus_core::Runtime::current) {
                    runtime.handle_event("focus", dx_focus, dioxus_id);
                    runtime.handle_event("focusin", dx_focusin, dioxus_id);
                }
            }
            return Ok(new_id);
        }
        return Ok(target_node_id);
    }

    // 2. Handle Escape focus clearing
    if key == Key::Escape
        && let Some(old_id) = doc.get_focussed_node_id()
    {
        doc.clear_focus();
        if let Some(node) = doc.get_node(old_id)
            && let Some(dioxus_id) = get_dioxus_id(node)
        {
            let platform_event = Rc::new(PlatformEventData::new(Box::new(())));
            let dx_blur = dioxus_core::Event::new(platform_event as Rc<dyn Any>, false);
            if let Ok(runtime) = std::panic::catch_unwind(dioxus_core::Runtime::current) {
                runtime.handle_event("blur", dx_blur, dioxus_id);
            }
        }
    }

    // 3. Handle Enter / Space button activation
    let is_button = doc
        .get_node(target_node_id)
        .and_then(|n| n.element_data())
        .is_some_and(|el| {
            &*el.name.local == "button"
                || el
                    .attrs()
                    .iter()
                    .any(|a| &*a.name.local == "role" && a.value == "button")
        });
    if is_button && (key == Key::Enter || key == Key::Character(" ".into())) {
        dispatch_click(doc, target_node_id, combined_mods);
    }

    // 4. Construct BlitzKeyEvent for KeyDown and KeyUp
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

    // 5. Dispatch UI events to BaseDocument
    doc.handle_ui_event(UiEvent::KeyDown(key_down.clone()));
    doc.handle_ui_event(UiEvent::KeyUp(key_up.clone()));

    // 6. If target node is a text input, propagate updated value to Dioxus listeners
    if let Some(node) = doc.get_node(target_node_id)
        && let Some(el) = node.element_data()
        && let Some(input_data) = el.text_input_data()
    {
        let current_val = input_data.editor.raw_text().to_string();
        let event_data = dioxus_native::synthetic_form_event(current_val, vec![]);
        let platform_event = Rc::new(PlatformEventData::new(event_data));
        let dx_input = dioxus_core::Event::new(platform_event.clone() as Rc<dyn Any>, true);
        let dx_change = dioxus_core::Event::new(platform_event as Rc<dyn Any>, true);

        if let Some(dioxus_id) = get_dioxus_id(node)
            && let Ok(runtime) = std::panic::catch_unwind(dioxus_core::Runtime::current)
        {
            runtime.handle_event("input", dx_input, dioxus_id);
            runtime.handle_event("change", dx_change, dioxus_id);
        }
    }

    // 7. Dispatch Dioxus KeyDown and KeyUp events
    if let Some((dioxus_id, _)) = find_dioxus_target(doc, target_node_id)
        && let Ok(runtime) = std::panic::catch_unwind(dioxus_core::Runtime::current)
    {
        let platform_down = Rc::new(PlatformEventData::new(
            dioxus_native::synthetic_keyboard_event(key_down.clone()),
        ));
        let platform_up = Rc::new(PlatformEventData::new(
            dioxus_native::synthetic_keyboard_event(key_up.clone()),
        ));
        let dx_down = dioxus_core::Event::new(platform_down as Rc<dyn Any>, true);
        let dx_up = dioxus_core::Event::new(platform_up as Rc<dyn Any>, true);
        runtime.handle_event("keydown", dx_down, dioxus_id);
        runtime.handle_event("keyup", dx_up, dioxus_id);
    }

    Ok(target_node_id)
}

/// Dispatch a pointer/mouse move event in `doc`.
/// Drives Blitz DOM hover state resolution and delivers
/// Dioxus pointer and mouse events (`pointermove`, `mousemove`, `pointerenter`, `pointerleave`, etc.).
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

    let prev_hover = doc.get_hover_node_id();

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

    // 1. Dispatch UI event to BaseDocument (updates doc.hover_node_id and element states)
    doc.handle_ui_event(UiEvent::PointerMove(ptr_event));

    let curr_hover = doc.get_hover_node_id().or(target_node_id);
    let resolved_nid = curr_hover.unwrap_or_else(|| doc.root_node().id);

    // 2. Dispatch Dioxus pointer events via Runtime::current
    if let Ok(runtime) = std::panic::catch_unwind(dioxus_core::Runtime::current) {
        let platform_event = if let Some(node) = doc.get_node(resolved_nid) {
            Rc::new(PlatformEventData::new(click_event_payload(node, modifiers)))
        } else {
            Rc::new(PlatformEventData::new(Box::new(())))
        };

        // If hovered target changed, dispatch leave on old and enter on new
        if prev_hover != curr_hover {
            if let Some(prev_id) = prev_hover
                && let Some((prev_dx_id, _)) = find_dioxus_target(doc, prev_id)
            {
                let dx_leave =
                    dioxus_core::Event::new(platform_event.clone() as Rc<dyn Any>, false);
                runtime.handle_event("pointerleave", dx_leave.clone(), prev_dx_id);
                runtime.handle_event("mouseleave", dx_leave, prev_dx_id);

                let dx_out = dioxus_core::Event::new(platform_event.clone() as Rc<dyn Any>, true);
                runtime.handle_event("pointerout", dx_out.clone(), prev_dx_id);
                runtime.handle_event("mouseout", dx_out, prev_dx_id);
            }

            if let Some(curr_id) = curr_hover
                && let Some((curr_dx_id, _)) = find_dioxus_target(doc, curr_id)
            {
                let dx_enter =
                    dioxus_core::Event::new(platform_event.clone() as Rc<dyn Any>, false);
                runtime.handle_event("pointerenter", dx_enter.clone(), curr_dx_id);
                runtime.handle_event("mouseenter", dx_enter, curr_dx_id);

                let dx_over = dioxus_core::Event::new(platform_event.clone() as Rc<dyn Any>, true);
                runtime.handle_event("pointerover", dx_over.clone(), curr_dx_id);
                runtime.handle_event("mouseover", dx_over, curr_dx_id);
            }
        }

        // Always dispatch pointermove / mousemove to current hovered target
        if let Some((curr_dx_id, _)) = find_dioxus_target(doc, resolved_nid) {
            let dx_move = dioxus_core::Event::new(platform_event as Rc<dyn Any>, true);
            runtime.handle_event("pointermove", dx_move.clone(), curr_dx_id);
            runtime.handle_event("mousemove", dx_move, curr_dx_id);
        }
    }

    Ok(resolved_nid)
}

/// Dispatch a pointer/mouse down event on a target node or coordinates in `doc`.
/// Drives Blitz DOM active state (`doc.active_node()`), focus, text selection, and delivers
/// Dioxus `pointerdown` and `mousedown` events.
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

    // Ensure hover is set to coordinates
    let _ = doc.set_hover_to(x, y);

    let buttons = MouseEventButtons::from(button);

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
        button,
        buttons,
        mods: modifiers,
        details: Default::default(),
        element: Default::default(),
        active_pointers: Default::default(),
    };

    // 1. Dispatch UI event to BaseDocument (activates node, sets mousedown_node_id, handles focus)
    doc.handle_ui_event(UiEvent::PointerDown(ptr_event));

    let resolved_nid = target_node_id
        .or(doc.get_hover_node_id())
        .unwrap_or_else(|| doc.root_node().id);

    // 2. Dispatch Dioxus pointerdown and mousedown events
    if let Some((dioxus_id, _)) = find_dioxus_target(doc, resolved_nid)
        && let Ok(runtime) = std::panic::catch_unwind(dioxus_core::Runtime::current)
    {
        let platform_event = if let Some(node) = doc.get_node(resolved_nid) {
            Rc::new(PlatformEventData::new(click_event_payload(node, modifiers)))
        } else {
            Rc::new(PlatformEventData::new(Box::new(())))
        };
        let dx_down = dioxus_core::Event::new(platform_event as Rc<dyn Any>, true);
        runtime.handle_event("pointerdown", dx_down.clone(), dioxus_id);
        runtime.handle_event("mousedown", dx_down, dioxus_id);
    }

    Ok(resolved_nid)
}

/// Dispatch a pointer/mouse up event on a target node or coordinates in `doc`.
/// Drives Blitz DOM unactive state (`doc.unactive_node()`) and delivers
/// Dioxus `pointerup` and `mouseup` events.
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
        button,
        buttons: MouseEventButtons::None,
        mods: modifiers,
        details: Default::default(),
        element: Default::default(),
        active_pointers: Default::default(),
    };

    // 1. Dispatch UI event to BaseDocument (unactivates node)
    doc.handle_ui_event(UiEvent::PointerUp(ptr_event));

    let resolved_nid = target_node_id
        .or(doc.get_hover_node_id())
        .unwrap_or_else(|| doc.root_node().id);

    // 2. Dispatch Dioxus pointerup and mouseup events
    if let Some((dioxus_id, _)) = find_dioxus_target(doc, resolved_nid)
        && let Ok(runtime) = std::panic::catch_unwind(dioxus_core::Runtime::current)
    {
        let platform_event = if let Some(node) = doc.get_node(resolved_nid) {
            Rc::new(PlatformEventData::new(click_event_payload(node, modifiers)))
        } else {
            Rc::new(PlatformEventData::new(Box::new(())))
        };
        let dx_up = dioxus_core::Event::new(platform_event as Rc<dyn Any>, true);
        runtime.handle_event("pointerup", dx_up.clone(), dioxus_id);
        runtime.handle_event("mouseup", dx_up, dioxus_id);
    }

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

    // Ensure hover position is set at scroll point
    let _ = doc.set_hover_to(x, y);

    let resolved_nid = doc
        .get_hover_node_id()
        .or(target_node_id)
        .unwrap_or_else(|| doc.root_node().id);

    let wheel_event = BlitzWheelEvent {
        delta: BlitzWheelDelta::Pixels(delta_x, delta_y),
        coords: PointerCoords {
            page_x: x,
            page_y: y,
            screen_x: x,
            screen_y: y,
            client_x: x,
            client_y: y,
        },
        buttons: MouseEventButtons::None,
        mods: modifiers,
        element: Default::default(),
    };

    // 1. Dispatch UI event to BaseDocument (triggers handle_wheel and doc.scroll_chain_by)
    doc.handle_ui_event(UiEvent::Wheel(wheel_event));

    // 2. Dispatch Dioxus wheel and scroll events
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_key_str() {
        let (k, c, m) = parse_key_str("ctrl+shift+Tab");
        assert_eq!(k, Key::Tab);
        assert_eq!(c, Code::Tab);
        assert!(m.contains(Modifiers::CONTROL));
        assert!(m.contains(Modifiers::SHIFT));

        let (k, c, m) = parse_key_str("cmd+Enter");
        assert_eq!(k, Key::Enter);
        assert_eq!(c, Code::Enter);
        assert!(m.contains(Modifiers::META));

        let (k, c, m) = parse_key_str("Escape");
        assert_eq!(k, Key::Escape);
        assert_eq!(c, Code::Escape);
        assert!(m.is_empty());
    }
}
