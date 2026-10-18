// SPDX-FileCopyrightText: 2026 Kerrick Long <me@kerricklong.com>
// SPDX-License-Identifier: LGPL-3.0-or-later

use magnus::{Error, IntoValue, TryConvert, Value};
use rb_sys::rb_thread_call_without_gvl;
use std::cell::RefCell;
use std::os::raw::c_void;

/// Wrapper enum for test events - includes crossterm events and our Sync event.
#[derive(Debug, Clone)]
enum TestEvent {
    Crossterm(ratatui::crossterm::event::Event),
    Sync,
    None,
}

thread_local! {
    static EVENT_QUEUE: RefCell<Vec<TestEvent>> = const { RefCell::new(Vec::new()) };
}

use ratatui::crossterm::event::{KeyCode, KeyModifiers, MediaKeyCode, ModifierKeyCode};

/// Single source of truth for base key code mappings.
const BASE_KEY_MAPPINGS: &[(&str, KeyCode)] = &[
    // Arrow keys
    ("up", KeyCode::Up),
    ("down", KeyCode::Down),
    ("left", KeyCode::Left),
    ("right", KeyCode::Right),
    // Common keys
    ("enter", KeyCode::Enter),
    ("esc", KeyCode::Esc),
    ("backspace", KeyCode::Backspace),
    ("tab", KeyCode::Tab),
    ("back_tab", KeyCode::BackTab),
    ("null", KeyCode::Null),
    // Navigation keys
    ("home", KeyCode::Home),
    ("end", KeyCode::End),
    ("page_up", KeyCode::PageUp),
    ("page_down", KeyCode::PageDown),
    ("insert", KeyCode::Insert),
    ("delete", KeyCode::Delete),
    // Lock keys
    ("caps_lock", KeyCode::CapsLock),
    ("scroll_lock", KeyCode::ScrollLock),
    ("num_lock", KeyCode::NumLock),
    // System keys
    ("print_screen", KeyCode::PrintScreen),
    ("pause", KeyCode::Pause),
    ("menu", KeyCode::Menu),
    ("keypad_begin", KeyCode::KeypadBegin),
];

/// Single source of truth for media key mappings.
const MEDIA_KEY_MAPPINGS: &[(&str, MediaKeyCode)] = &[
    ("media_play", MediaKeyCode::Play),
    ("media_pause", MediaKeyCode::Pause),
    ("media_play_pause", MediaKeyCode::PlayPause),
    ("media_reverse", MediaKeyCode::Reverse),
    ("media_stop", MediaKeyCode::Stop),
    ("media_fast_forward", MediaKeyCode::FastForward),
    ("media_rewind", MediaKeyCode::Rewind),
    ("media_track_next", MediaKeyCode::TrackNext),
    ("media_track_previous", MediaKeyCode::TrackPrevious),
    ("media_record", MediaKeyCode::Record),
    ("media_lower_volume", MediaKeyCode::LowerVolume),
    ("media_raise_volume", MediaKeyCode::RaiseVolume),
    ("media_mute_volume", MediaKeyCode::MuteVolume),
];

/// Single source of truth for modifier key mappings.
const MODIFIER_KEY_MAPPINGS: &[(&str, ModifierKeyCode)] = &[
    ("left_shift", ModifierKeyCode::LeftShift),
    ("left_control", ModifierKeyCode::LeftControl),
    ("left_alt", ModifierKeyCode::LeftAlt),
    ("left_super", ModifierKeyCode::LeftSuper),
    ("left_hyper", ModifierKeyCode::LeftHyper),
    ("left_meta", ModifierKeyCode::LeftMeta),
    ("right_shift", ModifierKeyCode::RightShift),
    ("right_control", ModifierKeyCode::RightControl),
    ("right_alt", ModifierKeyCode::RightAlt),
    ("right_super", ModifierKeyCode::RightSuper),
    ("right_hyper", ModifierKeyCode::RightHyper),
    ("right_meta", ModifierKeyCode::RightMeta),
    ("iso_level3_shift", ModifierKeyCode::IsoLevel3Shift),
    ("iso_level5_shift", ModifierKeyCode::IsoLevel5Shift),
];

/// Single source of truth for keyboard modifier flag mappings.
const KEYBOARD_MODIFIER_MAPPINGS: &[(&str, KeyModifiers)] = &[
    ("ctrl", KeyModifiers::CONTROL),
    ("alt", KeyModifiers::ALT),
    ("shift", KeyModifiers::SHIFT),
];

/// Returns all supported key codes for RBS generation.
pub fn all_key_codes() -> magnus::RHash {
    let ruby = magnus::Ruby::get().unwrap();
    let hash = ruby.hash_new();

    let base: Vec<&str> = BASE_KEY_MAPPINGS.iter().map(|(s, _)| *s).collect();
    let media: Vec<&str> = MEDIA_KEY_MAPPINGS.iter().map(|(s, _)| *s).collect();
    let modifier_keys: Vec<&str> = MODIFIER_KEY_MAPPINGS.iter().map(|(s, _)| *s).collect();
    let keyboard_modifiers: Vec<&str> =
        KEYBOARD_MODIFIER_MAPPINGS.iter().map(|(s, _)| *s).collect();

    let _ = hash.aset(ruby.to_symbol("base_keys"), base);
    let _ = hash.aset(ruby.to_symbol("media_keys"), media);
    let _ = hash.aset(ruby.to_symbol("modifier_keys"), modifier_keys);
    let _ = hash.aset(ruby.to_symbol("keyboard_modifiers"), keyboard_modifiers);

    hash
}

#[allow(clippy::needless_pass_by_value)]
pub fn inject_test_event(event_type: String, data: magnus::RHash) -> Result<(), Error> {
    let ruby = magnus::Ruby::get().unwrap();
    let event = match event_type.as_str() {
        "key" => TestEvent::Crossterm(parse_key_event(data, &ruby)?),
        "mouse" => TestEvent::Crossterm(parse_mouse_event(data, &ruby)?),
        "resize" => TestEvent::Crossterm(parse_resize_event(data, &ruby)?),
        "paste" => TestEvent::Crossterm(parse_paste_event(data, &ruby)?),
        "focus_gained" => TestEvent::Crossterm(ratatui::crossterm::event::Event::FocusGained),
        "focus_lost" => TestEvent::Crossterm(ratatui::crossterm::event::Event::FocusLost),
        "sync" => TestEvent::Sync,
        "none" => TestEvent::None,
        _ => {
            return Err(Error::new(
                ruby.exception_arg_error(),
                format!("Unknown event type: {event_type}"),
            ))
        }
    };

    EVENT_QUEUE.with(|q| q.borrow_mut().push(event));
    Ok(())
}

fn parse_base_key(s: &str) -> Option<KeyCode> {
    BASE_KEY_MAPPINGS
        .iter()
        .find(|(key, _)| *key == s)
        .map(|(_, code)| *code)
}

fn parse_media_key(s: &str) -> Option<MediaKeyCode> {
    MEDIA_KEY_MAPPINGS
        .iter()
        .find(|(key, _)| *key == s)
        .map(|(_, code)| *code)
}

fn parse_modifier_key(s: &str) -> Option<ModifierKeyCode> {
    MODIFIER_KEY_MAPPINGS
        .iter()
        .find(|(key, _)| *key == s)
        .map(|(_, code)| *code)
}

fn parse_keyboard_modifier(s: &str) -> Option<KeyModifiers> {
    KEYBOARD_MODIFIER_MAPPINGS
        .iter()
        .find(|(key, _)| *key == s)
        .map(|(_, mods)| *mods)
}

/// Maps a crossterm `KeyEventKind` to its Ruby symbol name.
///
/// Single source of truth for the `event_kind` value reported by
/// `Event::Key`: `press`, `repeat`, or `release`.
fn key_event_kind_to_string(kind: ratatui::crossterm::event::KeyEventKind) -> &'static str {
    match kind {
        ratatui::crossterm::event::KeyEventKind::Press => "press",
        ratatui::crossterm::event::KeyEventKind::Repeat => "repeat",
        ratatui::crossterm::event::KeyEventKind::Release => "release",
    }
}

/// Parses a Ruby `event_kind` string into a crossterm `KeyEventKind`.
///
/// Inverse of `key_event_kind_to_string`. Returns `None` for unknown
/// strings so callers can raise `ArgumentError` like the rest of the
/// event parser.
fn parse_key_event_kind(s: &str) -> Option<ratatui::crossterm::event::KeyEventKind> {
    match s {
        "press" => Some(ratatui::crossterm::event::KeyEventKind::Press),
        "repeat" => Some(ratatui::crossterm::event::KeyEventKind::Repeat),
        "release" => Some(ratatui::crossterm::event::KeyEventKind::Release),
        _ => None,
    }
}

fn parse_key_event(
    data: magnus::RHash,
    ruby: &magnus::Ruby,
) -> Result<ratatui::crossterm::event::Event, Error> {
    let code_val: Value = data
        .get(ruby.to_symbol("code"))
        .ok_or_else(|| Error::new(ruby.exception_arg_error(), "Missing 'code' in key event"))?;
    let code_str: String = String::try_convert(code_val)?;

    let code = if let Some(kc) = parse_base_key(&code_str) {
        kc
    } else if let Some(m) = parse_media_key(&code_str) {
        KeyCode::Media(m)
    } else if let Some(m) = parse_modifier_key(&code_str) {
        KeyCode::Modifier(m)
    } else if let Some(num_str) = code_str.strip_prefix('f') {
        if let Ok(n) = num_str.parse::<u8>() {
            KeyCode::F(n)
        } else {
            KeyCode::Char(code_str.chars().next().unwrap_or('\0'))
        }
    } else if code_str.len() == 1 {
        KeyCode::Char(code_str.chars().next().unwrap())
    } else {
        KeyCode::Null
    };

    let mut modifiers = KeyModifiers::empty();
    if let Some(mods_val) = data.get(ruby.to_symbol("modifiers")) {
        let mods: Vec<String> = Vec::try_convert(mods_val)?;
        for m in mods {
            if let Some(mod_flag) = parse_keyboard_modifier(&m) {
                modifiers |= mod_flag;
            }
        }
    }

    let kind = match data.get(ruby.to_symbol("event_kind")) {
        Some(kind_val) => {
            let kind_str: String = String::try_convert(kind_val)?;
            parse_key_event_kind(&kind_str).ok_or_else(|| {
                Error::new(
                    ruby.exception_arg_error(),
                    format!("Unknown event_kind: {kind_str}"),
                )
            })?
        }
        None => ratatui::crossterm::event::KeyEventKind::Press,
    };

    Ok(ratatui::crossterm::event::Event::Key(
        ratatui::crossterm::event::KeyEvent::new_with_kind(code, modifiers, kind),
    ))
}

fn parse_mouse_event(
    data: magnus::RHash,
    ruby: &magnus::Ruby,
) -> Result<ratatui::crossterm::event::Event, Error> {
    let kind_val: Value = data
        .get(ruby.to_symbol("kind"))
        .ok_or_else(|| Error::new(ruby.exception_arg_error(), "Missing 'kind' in mouse event"))?;
    let kind_str: String = String::try_convert(kind_val)?;

    let button = if let Some(btn_val) = data.get(ruby.to_symbol("button")) {
        let button_str: String = String::try_convert(btn_val)?;
        match button_str.as_str() {
            "right" => ratatui::crossterm::event::MouseButton::Right,
            "middle" => ratatui::crossterm::event::MouseButton::Middle,
            _ => ratatui::crossterm::event::MouseButton::Left,
        }
    } else {
        ratatui::crossterm::event::MouseButton::Left
    };

    let x_val: Value = data
        .get(ruby.to_symbol("x"))
        .ok_or_else(|| Error::new(ruby.exception_arg_error(), "Missing 'x' in mouse event"))?;
    let x: u16 = u16::try_convert(x_val)?;

    let y_val: Value = data
        .get(ruby.to_symbol("y"))
        .ok_or_else(|| Error::new(ruby.exception_arg_error(), "Missing 'y' in mouse event"))?;
    let y: u16 = u16::try_convert(y_val)?;

    let kind = match kind_str.as_str() {
        "down" => ratatui::crossterm::event::MouseEventKind::Down(button),
        "up" => ratatui::crossterm::event::MouseEventKind::Up(button),
        "drag" => ratatui::crossterm::event::MouseEventKind::Drag(button),
        "moved" => ratatui::crossterm::event::MouseEventKind::Moved,
        "scroll_down" => ratatui::crossterm::event::MouseEventKind::ScrollDown,
        "scroll_up" => ratatui::crossterm::event::MouseEventKind::ScrollUp,
        "scroll_left" => ratatui::crossterm::event::MouseEventKind::ScrollLeft,
        "scroll_right" => ratatui::crossterm::event::MouseEventKind::ScrollRight,
        _ => {
            return Err(Error::new(
                ruby.exception_arg_error(),
                format!("Unknown mouse kind: {kind_str}"),
            ))
        }
    };

    let mut modifiers = ratatui::crossterm::event::KeyModifiers::empty();
    if let Some(mods_val) = data.get(ruby.to_symbol("modifiers")) {
        let mods: Vec<String> = Vec::try_convert(mods_val)?;
        for m in mods {
            match m.as_str() {
                "ctrl" => modifiers |= ratatui::crossterm::event::KeyModifiers::CONTROL,
                "alt" => modifiers |= ratatui::crossterm::event::KeyModifiers::ALT,
                "shift" => modifiers |= ratatui::crossterm::event::KeyModifiers::SHIFT,
                _ => {}
            }
        }
    }

    Ok(ratatui::crossterm::event::Event::Mouse(
        ratatui::crossterm::event::MouseEvent {
            kind,
            column: x,
            row: y,
            modifiers,
        },
    ))
}

fn parse_resize_event(
    data: magnus::RHash,
    ruby: &magnus::Ruby,
) -> Result<ratatui::crossterm::event::Event, Error> {
    let width_val: Value = data.get(ruby.to_symbol("width")).ok_or_else(|| {
        Error::new(
            ruby.exception_arg_error(),
            "Missing 'width' in resize event",
        )
    })?;
    let width: u16 = u16::try_convert(width_val)?;

    let height_val: Value = data.get(ruby.to_symbol("height")).ok_or_else(|| {
        Error::new(
            ruby.exception_arg_error(),
            "Missing 'height' in resize event",
        )
    })?;
    let height: u16 = u16::try_convert(height_val)?;

    Ok(ratatui::crossterm::event::Event::Resize(width, height))
}

fn parse_paste_event(
    data: magnus::RHash,
    ruby: &magnus::Ruby,
) -> Result<ratatui::crossterm::event::Event, Error> {
    let content_val: Value = data.get(ruby.to_symbol("content")).ok_or_else(|| {
        Error::new(
            ruby.exception_arg_error(),
            "Missing 'content' in paste event",
        )
    })?;
    let content: String = String::try_convert(content_val)?;
    Ok(ratatui::crossterm::event::Event::Paste(content))
}

pub fn clear_events() {
    EVENT_QUEUE.with(|q| q.borrow_mut().clear());
}

/// Result of polling crossterm from outside the GVL.
///
/// The blocking I/O (poll + read) happens without the Ruby GVL held,
/// so other Ruby threads can run concurrently. This enum carries the
/// result back across the GVL boundary for Ruby object construction.
#[derive(Debug)]
enum PollResult {
    /// A crossterm event was read successfully.
    Event(ratatui::crossterm::event::Event),
    /// The timeout expired with no event available.
    NoEvent,
    /// An I/O error occurred during poll or read.
    Error(String),
}

/// Data passed to the GVL-free polling callback.
///
/// The `timeout` field controls the poll behavior:
/// - `Some(duration)`: poll with a timeout, return `NoEvent` if it expires.
/// - `None`: block indefinitely until an event arrives.
struct PollData {
    timeout: Option<std::time::Duration>,
    result: PollResult,
}

/// Polls crossterm for events without the Ruby GVL held.
///
/// This is the work function passed to `rb_thread_call_without_gvl`.
/// It performs blocking I/O that would otherwise starve Ruby threads.
extern "C" fn poll_without_gvl(data: *mut c_void) -> *mut c_void {
    // SAFETY: `data` is a valid pointer to `PollData`, passed by
    // `poll_crossterm_without_gvl` via `rb_thread_call_without_gvl`. The
    // pointer remains valid for the duration of this callback because the
    // caller owns the `PollData` on the stack and waits for us to return.
    let data = unsafe { &mut *data.cast::<PollData>() };

    data.result = match data.timeout {
        Some(duration) => match ratatui::crossterm::event::poll(duration) {
            Ok(true) => match ratatui::crossterm::event::read() {
                Ok(event) => PollResult::Event(event),
                Err(e) => PollResult::Error(e.to_string()),
            },
            Ok(false) => PollResult::NoEvent,
            Err(e) => PollResult::Error(e.to_string()),
        },
        None => match ratatui::crossterm::event::read() {
            Ok(event) => PollResult::Event(event),
            Err(e) => PollResult::Error(e.to_string()),
        },
    };

    std::ptr::null_mut()
}

/// Polls crossterm with the GVL released, then converts the result
/// to a Ruby value after reacquiring the GVL.
fn poll_crossterm_without_gvl(
    ruby: &magnus::Ruby,
    timeout: Option<std::time::Duration>,
) -> Result<Value, Error> {
    let mut data = PollData {
        timeout,
        result: PollResult::NoEvent,
    };

    // SAFETY: `data` is a valid stack-local `PollData` whose lifetime
    // spans this entire call. `poll_without_gvl` only reads/writes
    // through the pointer while `rb_thread_call_without_gvl` blocks,
    // and we don't access `data` again until that function returns.
    unsafe {
        rb_thread_call_without_gvl(
            Some(poll_without_gvl),
            (&raw mut data).cast::<c_void>(),
            None,
            std::ptr::null_mut(),
        );
    }

    // GVL is now re-held — safe to create Ruby objects.
    match data.result {
        PollResult::Event(event) => handle_crossterm_event(event),
        PollResult::NoEvent => Ok(ruby.qnil().into_value_with(ruby)),
        PollResult::Error(msg) => Err(Error::new(ruby.exception_runtime_error(), msg)),
    }
}

pub fn poll_event(ruby: &magnus::Ruby, timeout_val: Option<f64>) -> Result<Value, Error> {
    let event = EVENT_QUEUE.with(|q| {
        let mut queue = q.borrow_mut();
        if queue.is_empty() {
            None
        } else {
            Some(queue.remove(0))
        }
    });

    if let Some(e) = event {
        return handle_test_event(e);
    }

    let is_test_mode = crate::terminal::with_query(|q| q.is_test_mode()).unwrap_or(false);

    if is_test_mode {
        return Ok(ruby.qnil().into_value_with(ruby));
    }

    let timeout = timeout_val.map(std::time::Duration::from_secs_f64);
    poll_crossterm_without_gvl(ruby, timeout)
}

fn handle_test_event(event: TestEvent) -> Result<Value, Error> {
    match event {
        TestEvent::Crossterm(e) => handle_crossterm_event(e),
        TestEvent::Sync => handle_sync_event(),
        TestEvent::None => {
            let ruby = magnus::Ruby::get().unwrap();
            Ok(ruby.qnil().into_value_with(&ruby))
        }
    }
}

fn handle_crossterm_event(event: ratatui::crossterm::event::Event) -> Result<Value, Error> {
    match event {
        ratatui::crossterm::event::Event::Key(key) => handle_key_event(key),
        ratatui::crossterm::event::Event::Mouse(event) => handle_mouse_event(event),
        ratatui::crossterm::event::Event::Resize(w, h) => handle_resize_event(w, h),
        ratatui::crossterm::event::Event::Paste(content) => handle_paste_event(content),
        ratatui::crossterm::event::Event::FocusGained => handle_focus_event("focus_gained"),
        ratatui::crossterm::event::Event::FocusLost => handle_focus_event("focus_lost"),
    }
}

fn handle_sync_event() -> Result<Value, Error> {
    let ruby = magnus::Ruby::get().unwrap();
    let hash = ruby.hash_new();
    hash.aset(ruby.to_symbol("type"), ruby.to_symbol("sync"))?;
    Ok(hash.into_value_with(&ruby))
}

fn media_key_to_string(m: MediaKeyCode) -> &'static str {
    MEDIA_KEY_MAPPINGS
        .iter()
        .find(|(_, code)| *code == m)
        .map_or("unknown", |(s, _)| *s)
}

fn modifier_key_to_string(m: ModifierKeyCode) -> &'static str {
    MODIFIER_KEY_MAPPINGS
        .iter()
        .find(|(_, code)| *code == m)
        .map_or("unknown", |(s, _)| *s)
}

fn base_key_to_string(kc: KeyCode) -> Option<&'static str> {
    BASE_KEY_MAPPINGS
        .iter()
        .find(|(_, code)| *code == kc)
        .map(|(s, _)| *s)
}

fn handle_key_event(key: ratatui::crossterm::event::KeyEvent) -> Result<Value, Error> {
    use ratatui::crossterm::event::KeyCode;

    let ruby = magnus::Ruby::get().unwrap();
    let hash = ruby.hash_new();
    hash.aset(ruby.to_symbol("type"), ruby.to_symbol("key"))?;

    // Determine the kind (category) of the key
    let kind = match key.code {
        KeyCode::Char(_)
        | KeyCode::Enter
        | KeyCode::Tab
        | KeyCode::Backspace
        | KeyCode::BackTab
        | KeyCode::Up
        | KeyCode::Down
        | KeyCode::Left
        | KeyCode::Right
        | KeyCode::Home
        | KeyCode::End
        | KeyCode::PageUp
        | KeyCode::PageDown
        | KeyCode::Insert
        | KeyCode::Delete
        | KeyCode::Null => "standard",
        KeyCode::F(_) => "function",
        KeyCode::Media(_) => "media",
        KeyCode::Modifier(_) => "modifier",
        KeyCode::Esc
        | KeyCode::CapsLock
        | KeyCode::ScrollLock
        | KeyCode::NumLock
        | KeyCode::PrintScreen
        | KeyCode::Pause
        | KeyCode::Menu
        | KeyCode::KeypadBegin => "system",
    };

    let code = if let KeyCode::Char(c) = key.code {
        c.to_string()
    } else if let KeyCode::F(n) = key.code {
        format!("f{n}")
    } else if let KeyCode::Media(m) = key.code {
        media_key_to_string(m).to_string()
    } else if let KeyCode::Modifier(m) = key.code {
        modifier_key_to_string(m).to_string()
    } else if let Some(s) = base_key_to_string(key.code) {
        s.to_string()
    } else {
        "unknown".to_string()
    };

    hash.aset(ruby.to_symbol("code"), code)?;
    hash.aset(ruby.to_symbol("kind"), ruby.to_symbol(kind))?;
    hash.aset(
        ruby.to_symbol("event_kind"),
        ruby.to_symbol(key_event_kind_to_string(key.kind)),
    )?;

    let mut modifiers = Vec::new();
    for (name, flag) in KEYBOARD_MODIFIER_MAPPINGS {
        if key.modifiers.contains(*flag) {
            modifiers.push(*name);
        }
    }
    if !modifiers.is_empty() {
        hash.aset(ruby.to_symbol("modifiers"), modifiers)?;
    }
    Ok(hash.into_value_with(&ruby))
}

fn handle_mouse_event(event: ratatui::crossterm::event::MouseEvent) -> Result<Value, Error> {
    let ruby = magnus::Ruby::get().unwrap();
    let hash = ruby.hash_new();
    hash.aset(ruby.to_symbol("type"), ruby.to_symbol("mouse"))?;
    let (kind, button) = match event.kind {
        ratatui::crossterm::event::MouseEventKind::Down(btn) => ("down", btn),
        ratatui::crossterm::event::MouseEventKind::Up(btn) => ("up", btn),
        ratatui::crossterm::event::MouseEventKind::Drag(btn) => ("drag", btn),
        ratatui::crossterm::event::MouseEventKind::Moved => {
            ("moved", ratatui::crossterm::event::MouseButton::Left)
        }
        ratatui::crossterm::event::MouseEventKind::ScrollDown => {
            ("scroll_down", ratatui::crossterm::event::MouseButton::Left)
        }
        ratatui::crossterm::event::MouseEventKind::ScrollUp => {
            ("scroll_up", ratatui::crossterm::event::MouseButton::Left)
        }
        ratatui::crossterm::event::MouseEventKind::ScrollLeft => {
            ("scroll_left", ratatui::crossterm::event::MouseButton::Left)
        }
        ratatui::crossterm::event::MouseEventKind::ScrollRight => {
            ("scroll_right", ratatui::crossterm::event::MouseButton::Left)
        }
    };
    hash.aset(ruby.to_symbol("kind"), ruby.to_symbol(kind))?;
    if matches!(
        event.kind,
        ratatui::crossterm::event::MouseEventKind::Down(_)
            | ratatui::crossterm::event::MouseEventKind::Up(_)
            | ratatui::crossterm::event::MouseEventKind::Drag(_)
    ) {
        let btn_sym = match button {
            ratatui::crossterm::event::MouseButton::Left => "left",
            ratatui::crossterm::event::MouseButton::Right => "right",
            ratatui::crossterm::event::MouseButton::Middle => "middle",
        };
        hash.aset(ruby.to_symbol("button"), ruby.to_symbol(btn_sym))?;
    } else {
        hash.aset(ruby.to_symbol("button"), ruby.to_symbol("none"))?;
    }
    hash.aset(ruby.to_symbol("x"), event.column)?;
    hash.aset(ruby.to_symbol("y"), event.row)?;
    let mut modifiers = Vec::new();
    if event
        .modifiers
        .contains(ratatui::crossterm::event::KeyModifiers::CONTROL)
    {
        modifiers.push("ctrl");
    }
    if event
        .modifiers
        .contains(ratatui::crossterm::event::KeyModifiers::ALT)
    {
        modifiers.push("alt");
    }
    if event
        .modifiers
        .contains(ratatui::crossterm::event::KeyModifiers::SHIFT)
    {
        modifiers.push("shift");
    }
    hash.aset(ruby.to_symbol("modifiers"), modifiers)?;
    Ok(hash.into_value_with(&ruby))
}

fn handle_resize_event(w: u16, h: u16) -> Result<Value, Error> {
    let ruby = magnus::Ruby::get().unwrap();
    let hash = ruby.hash_new();
    hash.aset(ruby.to_symbol("type"), ruby.to_symbol("resize"))?;
    hash.aset(ruby.to_symbol("width"), w)?;
    hash.aset(ruby.to_symbol("height"), h)?;
    Ok(hash.into_value_with(&ruby))
}

fn handle_paste_event(content: String) -> Result<Value, Error> {
    let ruby = magnus::Ruby::get().unwrap();
    let hash = ruby.hash_new();
    hash.aset(ruby.to_symbol("type"), ruby.to_symbol("paste"))?;
    hash.aset(ruby.to_symbol("content"), content)?;
    Ok(hash.into_value_with(&ruby))
}

fn handle_focus_event(event_type: &str) -> Result<Value, Error> {
    let ruby = magnus::Ruby::get().unwrap();
    let hash = ruby.hash_new();
    hash.aset(ruby.to_symbol("type"), ruby.to_symbol(event_type))?;
    Ok(hash.into_value_with(&ruby))
}

#[cfg(test)]
mod tests {
    use super::*;
    use ratatui::crossterm::event::KeyEventKind;

    /// Tests that host a Ruby VM inside the test binary.
    ///
    /// Linking a Ruby VM into a Rust executable requires a shared libruby.
    /// Static libruby builds (such as mise's prebuilt rubies) embed YJIT's
    /// Rust runtime, whose panic machinery collides with the test binary's
    /// own, so these tests compile only behind the `embed-ruby-tests`
    /// feature. The `cargo:test` rake task enables it when the active Ruby
    /// provides a shared libruby; the Ruby test suite exercises the same
    /// code paths through the compiled extension everywhere else.
    #[cfg(all(test, feature = "embed-ruby-tests"))]
    mod embedded_ruby_tests {
        use super::*;

        /// Initializes an embedded Ruby VM once for the whole test binary.
        ///
        /// Tests run with `--test-threads=1` (see the `cargo:test` rake task),
        /// so every test runs on the main thread and shares this one VM. The
        /// cleanup guard is intentionally leaked: the Ruby VM must outlive
        /// every test, and the test binary exits right after.
        fn embedded_ruby() -> magnus::Ruby {
            static INIT: std::sync::OnceLock<()> = std::sync::OnceLock::new();
            INIT.get_or_init(|| {
                // SAFETY: embed::init sets up the Ruby VM on the current thread,
                // which is safe here because tests run serially on one thread
                // (see the `cargo:test` rake task) and no other Ruby state
                // exists in the process. The returned cleanup guard would tear
                // the VM down when dropped, so it is leaked below to keep the
                // VM alive for every remaining test.
                let cleanup = unsafe { magnus::embed::init() };
                std::mem::forget(cleanup);
            });
            // SAFETY: the Ruby VM was initialized above on this same thread,
            // so a handle exists and this thread is a Ruby thread.
            unsafe { magnus::Ruby::get_unchecked() }
        }

        /// Builds a right-arrow `KeyEvent` with the given kind and modifiers.
        fn right_key_event(
            kind: KeyEventKind,
            modifiers: ratatui::crossterm::event::KeyModifiers,
        ) -> ratatui::crossterm::event::KeyEvent {
            ratatui::crossterm::event::KeyEvent::new_with_kind(KeyCode::Right, modifiers, kind)
        }

        /// Reads a Symbol hash value as its name.
        fn hash_symbol(hash: magnus::RHash, ruby: &magnus::Ruby, key: &str) -> String {
            let value = hash.get(ruby.to_symbol(key)).expect("hash key present");
            let symbol = magnus::Symbol::try_convert(value).expect("value is a Symbol");
            symbol.name().expect("symbol name").into_owned()
        }

        /// Reads a String hash value.
        fn hash_string(hash: magnus::RHash, ruby: &magnus::Ruby, key: &str) -> String {
            let value = hash.get(ruby.to_symbol(key)).expect("hash key present");
            String::try_convert(value).expect("value is a String")
        }

        /// Reads a String-array hash value.
        fn hash_strings(hash: magnus::RHash, ruby: &magnus::Ruby, key: &str) -> Vec<String> {
            let value = hash.get(ruby.to_symbol(key)).expect("hash key present");
            Vec::try_convert(value).expect("value is an Array of Strings")
        }

        /// Builds an injected-key hash shaped like `inject_test_event` payloads.
        fn injected_key_hash(ruby: &magnus::Ruby, event_kind: Option<&str>) -> magnus::RHash {
            let hash = ruby.hash_new();
            hash.aset(ruby.to_symbol("code"), "right").unwrap();
            if let Some(kind) = event_kind {
                hash.aset(ruby.to_symbol("event_kind"), kind).unwrap();
            }
            hash
        }

        fn assert_handle_key_event_hashes(
            ruby: &magnus::Ruby,
            event_kind: KeyEventKind,
            modifiers: ratatui::crossterm::event::KeyModifiers,
            expected_event_kind: &str,
            expected_modifiers: &[&str],
        ) {
            let value = handle_key_event(right_key_event(event_kind, modifiers)).unwrap();
            let hash = magnus::RHash::try_convert(value).expect("key event maps to a Hash");

            assert_eq!(hash_symbol(hash, ruby, "type"), "key");
            assert_eq!(hash_string(hash, ruby, "code"), "right");
            assert_eq!(hash_symbol(hash, ruby, "kind"), "standard");
            assert_eq!(hash_symbol(hash, ruby, "event_kind"), expected_event_kind);
            if expected_modifiers.is_empty() {
                assert!(
                    hash.get(ruby.to_symbol("modifiers")).is_none(),
                    "no modifiers were pressed, so the hash omits the key"
                );
            } else {
                assert_eq!(hash_strings(hash, ruby, "modifiers"), expected_modifiers);
            }
        }

        fn handle_key_event_reports_press(ruby: &magnus::Ruby) {
            assert_handle_key_event_hashes(
                ruby,
                KeyEventKind::Press,
                KeyModifiers::empty(),
                "press",
                &[],
            );
        }

        fn handle_key_event_reports_repeat(ruby: &magnus::Ruby) {
            assert_handle_key_event_hashes(
                ruby,
                KeyEventKind::Repeat,
                KeyModifiers::empty(),
                "repeat",
                &[],
            );
        }

        fn handle_key_event_reports_release(ruby: &magnus::Ruby) {
            assert_handle_key_event_hashes(
                ruby,
                KeyEventKind::Release,
                KeyModifiers::CONTROL,
                "release",
                &["ctrl"],
            );
        }

        fn parse_key_event_round_trips_every_event_kind(ruby: &magnus::Ruby) {
            for (name, expected) in [
                ("press", KeyEventKind::Press),
                ("repeat", KeyEventKind::Repeat),
                ("release", KeyEventKind::Release),
            ] {
                let event = parse_key_event(injected_key_hash(ruby, Some(name)), ruby)
                    .expect("valid injected event kind parses");
                match event {
                    ratatui::crossterm::event::Event::Key(key) => {
                        assert_eq!(key.kind, expected, "wrong kind for {name}");
                        assert_eq!(key.code, KeyCode::Right, "wrong code for {name}");
                    }
                    _ => panic!("expected a key event for {name}"),
                }
            }
        }

        fn parse_key_event_defaults_to_press(ruby: &magnus::Ruby) {
            let event = parse_key_event(injected_key_hash(ruby, None), ruby)
                .expect("absent event_kind parses as a press");
            match event {
                ratatui::crossterm::event::Event::Key(key) => {
                    assert_eq!(key.kind, KeyEventKind::Press);
                }
                _ => panic!("expected a key event"),
            }
        }

        fn parse_key_event_rejects_unknown_event_kind(ruby: &magnus::Ruby) {
            let result = parse_key_event(injected_key_hash(ruby, Some("held")), ruby);
            let error = result.expect_err("unknown event_kind must raise");
            assert!(error.is_kind_of(ruby.exception_arg_error()));
        }

        /// Runs every assertion that needs the embedded Ruby VM.
        ///
        /// The Ruby VM is a process-wide singleton bound to the thread that
        /// initialized it (see `embedded_ruby`), so all Ruby-backed checks live
        /// in this one test. Each phase is a focused helper so failures still
        /// point at the exact assertion.
        #[test]
        fn event_kind_flows_between_ruby_and_crossterm() {
            let ruby = embedded_ruby();
            handle_key_event_reports_press(&ruby);
            handle_key_event_reports_repeat(&ruby);
            handle_key_event_reports_release(&ruby);
            parse_key_event_round_trips_every_event_kind(&ruby);
            parse_key_event_defaults_to_press(&ruby);
            parse_key_event_rejects_unknown_event_kind(&ruby);
        }
    }
    #[test]
    fn poll_data_defaults_to_no_event() {
        let data = PollData {
            timeout: Some(std::time::Duration::from_millis(0)),
            result: PollResult::NoEvent,
        };
        assert!(matches!(data.result, PollResult::NoEvent));
    }

    #[test]
    fn poll_data_accepts_none_timeout_for_indefinite_blocking() {
        let data = PollData {
            timeout: None,
            result: PollResult::NoEvent,
        };
        assert!(data.timeout.is_none());
    }

    #[test]
    fn poll_result_error_preserves_message() {
        let result = PollResult::Error("connection reset".to_string());
        match result {
            PollResult::Error(msg) => assert_eq!(msg, "connection reset"),
            _ => panic!("Expected PollResult::Error"),
        }
    }

    #[test]
    fn poll_result_event_wraps_crossterm_event() {
        let key_event =
            ratatui::crossterm::event::Event::Key(ratatui::crossterm::event::KeyEvent::new(
                ratatui::crossterm::event::KeyCode::Char('q'),
                ratatui::crossterm::event::KeyModifiers::empty(),
            ));
        let result = PollResult::Event(key_event.clone());
        match result {
            PollResult::Event(e) => assert_eq!(format!("{e:?}"), format!("{key_event:?}")),
            _ => panic!("Expected PollResult::Event"),
        }
    }

    #[test]
    fn poll_without_gvl_returns_no_event_on_zero_timeout() {
        // With a zero-duration timeout, poll returns immediately with no event.
        // In headless environments (CI), crossterm may return an Error because
        // there is no terminal to read from — that's also a valid outcome.
        let mut data = PollData {
            timeout: Some(std::time::Duration::from_millis(0)),
            result: PollResult::Error("sentinel — should be overwritten".to_string()),
        };

        poll_without_gvl(&mut data as *mut PollData as *mut c_void);

        // The callback must have overwritten the sentinel value.
        match &data.result {
            PollResult::Error(msg) if msg == "sentinel — should be overwritten" => {
                panic!("poll_without_gvl did not write to data.result")
            }
            _ => {} // NoEvent, Event, or a *different* Error are all fine.
        }
    }

    #[test]
    fn key_event_kind_to_string_maps_all_kinds() {
        assert_eq!(key_event_kind_to_string(KeyEventKind::Press), "press");
        assert_eq!(key_event_kind_to_string(KeyEventKind::Repeat), "repeat");
        assert_eq!(key_event_kind_to_string(KeyEventKind::Release), "release");
    }

    #[test]
    fn parse_key_event_kind_inverts_the_mapping() {
        assert_eq!(parse_key_event_kind("press"), Some(KeyEventKind::Press));
        assert_eq!(parse_key_event_kind("repeat"), Some(KeyEventKind::Repeat));
        assert_eq!(parse_key_event_kind("release"), Some(KeyEventKind::Release));
        assert_eq!(parse_key_event_kind("held"), None);
    }
}
