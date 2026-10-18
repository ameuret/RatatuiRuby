// SPDX-FileCopyrightText: 2026 Kerrick Long <me@kerricklong.com>
// SPDX-License-Identifier: LGPL-3.0-or-later

//! Terminal mutation functions (write operations).

use magnus::{Error, Module};
use ratatui::crossterm::event::{
    KeyboardEnhancementFlags, PopKeyboardEnhancementFlags, PushKeyboardEnhancementFlags,
};

use super::TerminalWrapper;

/// Single source of truth for Kitty keyboard protocol enhancement flag mappings.
///
/// `report_associated_text` has no entry: crossterm 0.29 does not implement
/// that part of the protocol yet, so asking for it is an explicit error
/// rather than a silent no-op.
const KEYBOARD_ENHANCEMENT_FLAG_MAPPINGS: &[(&str, KeyboardEnhancementFlags)] = &[
    (
        "disambiguate",
        KeyboardEnhancementFlags::DISAMBIGUATE_ESCAPE_CODES,
    ),
    (
        "report_event_types",
        KeyboardEnhancementFlags::REPORT_EVENT_TYPES,
    ),
    (
        "report_alternate_keys",
        KeyboardEnhancementFlags::REPORT_ALTERNATE_KEYS,
    ),
    (
        "report_all_keys_as_escape_codes",
        KeyboardEnhancementFlags::REPORT_ALL_KEYS_AS_ESCAPE_CODES,
    ),
];

/// Maps Ruby keyboard enhancement flag names to crossterm flag bits.
///
/// Pure mapping kept separate from the terminal I/O so tests can verify every
/// flag bit without a live terminal. Failures surface a message for the caller
/// to raise as `ArgumentError`.
fn keyboard_enhancement_flags_from_names<I>(names: I) -> Result<KeyboardEnhancementFlags, String>
where
    I: IntoIterator<Item = String>,
{
    let mut names = names.into_iter().peekable();
    if names.peek().is_none() {
        return Err("At least one keyboard enhancement flag is required".to_string());
    }
    let mut flags = KeyboardEnhancementFlags::empty();
    for name in names {
        let Some((_, bits)) = KEYBOARD_ENHANCEMENT_FLAG_MAPPINGS
            .iter()
            .find(|(flag_name, _)| *flag_name == name)
        else {
            if name == "report_associated_text" {
                return Err(
                    "report_associated_text is not supported by the bundled crossterm version"
                        .to_string(),
                );
            }
            return Err(format!("Unknown keyboard enhancement flag: {name}"));
        };
        flags |= *bits;
    }
    Ok(flags)
}

/// Looks up the `RatatuiRuby::Error::Terminal` class.
fn terminal_error_class(ruby: &magnus::Ruby) -> magnus::ExceptionClass {
    let module = ruby.define_module("RatatuiRuby").unwrap();
    let error_base = module
        .const_get::<_, magnus::RClass>("Error")
        .expect("RatatuiRuby::Error is defined");
    error_base
        .const_get::<_, magnus::ExceptionClass>("Terminal")
        .expect("RatatuiRuby::Error::Terminal is defined")
}

/// Looks up the `RatatuiRuby::Error::Invariant` class.
fn invariant_error_class(ruby: &magnus::Ruby) -> magnus::ExceptionClass {
    let module = ruby.define_module("RatatuiRuby").unwrap();
    let error_base = module
        .const_get::<_, magnus::RClass>("Error")
        .expect("RatatuiRuby::Error is defined");
    error_base
        .const_get::<_, magnus::ExceptionClass>("Invariant")
        .expect("RatatuiRuby::Error::Invariant is defined")
}

/// Pushes Kitty keyboard protocol enhancement flags to the active terminal.
///
/// Flags are opt-in: nothing in `run` or `init_terminal` pushes them, so
/// applications decide explicitly when the terminal should start reporting
/// events like key repeat and release.
pub fn push_keyboard_enhancement_flags(flag_names: Vec<String>) -> Result<(), Error> {
    let ruby = magnus::Ruby::get().unwrap();
    let flags = keyboard_enhancement_flags_from_names(flag_names)
        .map_err(|message| Error::new(ruby.exception_arg_error(), message))?;

    crate::terminal::with_terminal_mut(|wrapper| match wrapper {
        TerminalWrapper::Crossterm(term) => {
            ratatui::crossterm::execute!(term.backend_mut(), PushKeyboardEnhancementFlags(flags))
                .map_err(|e| Error::new(terminal_error_class(&ruby), e.to_string()))
        }
        // The test backend has no real terminal to configure. Treat the
        // push as a no-op so applications run the same code under test as
        // in production.
        TerminalWrapper::Test(_) => Ok(()),
    })
    .unwrap_or_else(|| {
        if crate::terminal::is_in_draw_mode() {
            Err(Error::new(
                invariant_error_class(&ruby),
                "push_keyboard_enhancement_flags cannot be called during draw",
            ))
        } else {
            Err(Error::new(
                terminal_error_class(&ruby),
                "Terminal is not initialized",
            ))
        }
    })
}

/// Pops the Kitty keyboard protocol enhancement flags from the active terminal.
///
/// Pairs with `push_keyboard_enhancement_flags`: applications restore the
/// terminal's previous keyboard state on the way out, typically in an
/// ensure block around their run loop.
pub fn pop_keyboard_enhancement_flags() -> Result<(), Error> {
    let ruby = magnus::Ruby::get().unwrap();

    crate::terminal::with_terminal_mut(|wrapper| match wrapper {
        TerminalWrapper::Crossterm(term) => {
            ratatui::crossterm::execute!(term.backend_mut(), PopKeyboardEnhancementFlags)
                .map_err(|e| Error::new(terminal_error_class(&ruby), e.to_string()))
        }
        TerminalWrapper::Test(_) => Ok(()),
    })
    .unwrap_or_else(|| {
        if crate::terminal::is_in_draw_mode() {
            Err(Error::new(
                invariant_error_class(&ruby),
                "pop_keyboard_enhancement_flags cannot be called during draw",
            ))
        } else {
            Err(Error::new(
                terminal_error_class(&ruby),
                "Terminal is not initialized",
            ))
        }
    })
}

pub fn insert_before(height: u16, widget: magnus::Value) -> Result<(), Error> {
    let ruby = magnus::Ruby::get().unwrap();

    // with_terminal_mut returns None during draw mode, so we handle that case
    crate::terminal::with_terminal_mut(|wrapper| {
        let module = ruby.define_module("RatatuiRuby").unwrap();
        let error_base = module.const_get::<_, magnus::RClass>("Error").unwrap();
        let error_class = error_base.const_get("Terminal").unwrap();

        match wrapper {
            TerminalWrapper::Crossterm(term) => {
                // Capture rendering error since closure can't return Result
                let mut render_error: Option<String> = None;

                let result = term.insert_before(height, |buf| {
                    let area = buf.area();
                    let area_copy = *area; // Copy rect before closure capture

                    // Render widget to buffer using centralized dispatch
                    let render_result =
                        crate::rendering::render_widget_to_buffer(buf, area_copy, widget);

                    if let Err(e) = render_result {
                        render_error = Some(e.to_string());
                    }
                });

                // Handle insert_before error
                result.map_err(|e| Error::new(error_class, e.to_string()))?;

                // Handle rendering error
                if let Some(err_msg) = render_error {
                    return Err(Error::new(error_class, err_msg));
                }
            }
            TerminalWrapper::Test(term) => {
                // Capture rendering error since closure can't return Result
                let mut render_error: Option<String> = None;

                let result = term.insert_before(height, |buf| {
                    let area = buf.area();
                    let area_copy = *area; // Copy rect before closure capture

                    // Render widget to buffer using centralized dispatch
                    let render_result =
                        crate::rendering::render_widget_to_buffer(buf, area_copy, widget);

                    if let Err(e) = render_result {
                        render_error = Some(e.to_string());
                    }
                });

                // Handle insert_before error
                result.map_err(|e| Error::new(error_class, e.to_string()))?;

                // Handle rendering error
                if let Some(err_msg) = render_error {
                    return Err(Error::new(error_class, err_msg));
                }
            }
        }
        Ok(())
    })
    .unwrap_or_else(|| {
        // During draw or terminal not initialized
        let module = ruby.define_module("RatatuiRuby").unwrap();
        let error_base = module.const_get::<_, magnus::RClass>("Error").unwrap();
        if crate::terminal::is_in_draw_mode() {
            let error_class = error_base.const_get("Invariant").unwrap();
            Err(Error::new(
                error_class,
                "insert_before cannot be called during draw",
            ))
        } else {
            let error_class = error_base.const_get("Terminal").unwrap();
            Err(Error::new(error_class, "Terminal not initialized"))
        }
    })
}

pub fn set_cursor_position(x: u16, y: u16) -> Result<(), Error> {
    let ruby = magnus::Ruby::get().unwrap();

    crate::terminal::with_terminal_mut(|wrapper| {
        let module = ruby.define_module("RatatuiRuby").unwrap();
        let error_base = module.const_get::<_, magnus::RClass>("Error").unwrap();
        let error_class = error_base.const_get("Terminal").unwrap();

        match wrapper {
            TerminalWrapper::Crossterm(term) => {
                term.set_cursor_position((x, y))
                    .map_err(|e| Error::new(error_class, e.to_string()))?;
            }
            TerminalWrapper::Test(term) => {
                term.set_cursor_position((x, y))
                    .map_err(|e| Error::new(error_class, e.to_string()))?;
            }
        }
        Ok(())
    })
    .unwrap_or_else(|| {
        let module = ruby.define_module("RatatuiRuby").unwrap();
        let error_base = module.const_get::<_, magnus::RClass>("Error").unwrap();
        if crate::terminal::is_in_draw_mode() {
            let error_class = error_base.const_get("Invariant").unwrap();
            Err(Error::new(
                error_class,
                "set_cursor_position cannot be called during draw",
            ))
        } else {
            let error_class = error_base.const_get("Terminal").unwrap();
            Err(Error::new(error_class, "Terminal is not initialized"))
        }
    })
}

pub fn resize_terminal(width: u16, height: u16) -> Result<(), Error> {
    let ruby = magnus::Ruby::get().unwrap();

    crate::terminal::with_terminal_mut(|wrapper| {
        match wrapper {
            TerminalWrapper::Crossterm(_) => {}
            TerminalWrapper::Test(terminal) => {
                terminal.backend_mut().resize(width, height);
                if let Err(e) = terminal.resize(ratatui::layout::Rect::new(0, 0, width, height)) {
                    let module = ruby.define_module("RatatuiRuby").unwrap();
                    let error_base = module.const_get::<_, magnus::RClass>("Error").unwrap();
                    let error_class = error_base.const_get("Terminal").unwrap();
                    return Err(Error::new(error_class, e.to_string()));
                }
            }
        }
        Ok(())
    })
    .unwrap_or_else(|| {
        let module = ruby.define_module("RatatuiRuby").unwrap();
        let error_base = module.const_get::<_, magnus::RClass>("Error").unwrap();
        if crate::terminal::is_in_draw_mode() {
            let error_class = error_base.const_get("Invariant").unwrap();
            Err(Error::new(
                error_class,
                "resize_terminal cannot be called during draw",
            ))
        } else {
            // No terminal initialized is OK for resize - just no-op
            Ok(())
        }
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn flag_names(names: &[&str]) -> Vec<String> {
        names.iter().copied().map(str::to_string).collect()
    }

    #[test]
    fn each_flag_name_maps_to_its_crossterm_bit() {
        for (name, expected) in [
            (
                "disambiguate",
                KeyboardEnhancementFlags::DISAMBIGUATE_ESCAPE_CODES,
            ),
            (
                "report_event_types",
                KeyboardEnhancementFlags::REPORT_EVENT_TYPES,
            ),
            (
                "report_alternate_keys",
                KeyboardEnhancementFlags::REPORT_ALTERNATE_KEYS,
            ),
            (
                "report_all_keys_as_escape_codes",
                KeyboardEnhancementFlags::REPORT_ALL_KEYS_AS_ESCAPE_CODES,
            ),
        ] {
            let flags = keyboard_enhancement_flags_from_names(flag_names(&[name]))
                .unwrap_or_else(|e| panic!("flag {name} should parse: {e}"));
            assert_eq!(flags, expected, "wrong bits for flag {name}");
        }
    }

    #[test]
    fn multiple_flags_combine_into_one_bit_set() {
        let flags = keyboard_enhancement_flags_from_names(flag_names(&[
            "disambiguate",
            "report_event_types",
        ]))
        .expect("valid flags combine");
        assert_eq!(
            flags,
            KeyboardEnhancementFlags::DISAMBIGUATE_ESCAPE_CODES
                | KeyboardEnhancementFlags::REPORT_EVENT_TYPES
        );
    }

    #[test]
    fn duplicate_flags_collapse_into_one_bit_set() {
        let flags = keyboard_enhancement_flags_from_names(flag_names(&[
            "report_event_types",
            "report_event_types",
        ]))
        .expect("duplicated flags parse");
        assert_eq!(flags, KeyboardEnhancementFlags::REPORT_EVENT_TYPES);
    }

    #[test]
    fn empty_flag_list_is_rejected() {
        let error = keyboard_enhancement_flags_from_names(Vec::new())
            .expect_err("pushing zero flags is a caller bug");
        assert!(error.contains("At least one"));
    }

    #[test]
    fn unknown_flag_name_is_rejected_with_the_name_in_the_message() {
        let error = keyboard_enhancement_flags_from_names(flag_names(&["report_vibes"]))
            .expect_err("unknown flags must not parse");
        assert!(error.contains("report_vibes"));
    }

    #[test]
    fn unimplemented_flag_is_rejected_with_an_explanation() {
        let error = keyboard_enhancement_flags_from_names(flag_names(&["report_associated_text"]))
            .expect_err("crossterm 0.29 cannot push the associated-text flag");
        assert!(error.contains("not supported"));
    }
}
