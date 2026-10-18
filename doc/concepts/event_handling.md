<!--
  SPDX-FileCopyrightText: 2026 Kerrick Long <me@kerricklong.com>
  SPDX-License-Identifier: CC-BY-SA-4.0
-->

# Event Handling

`ratatui_ruby` provides a rich, object-oriented event system that supports multiple coding styles, from simple boolean predicates to modern Ruby pattern matching.

Events are retrieved using `RatatuiRuby.poll_event`. This method returns an instance of a subclass of `RatatuiRuby::Event` (e.g., `RatatuiRuby::Event::Key`, `RatatuiRuby::Event::Mouse`). When no event is available, it returns `RatatuiRuby::Event::None`—a [null object](https://martinfowler.com/eaaCatalog/specialCase.html) that safely responds to all event predicates with `false`.

## 1. Blocking vs. Polling

Most applications run in a loop (e.g., a game loop or UI loop). To prevent high CPU usage, the loop should wait briefly for input.

*   **Default (Polling):** `RatatuiRuby.poll_event` (no args) waits for **0.016s** (approx 60 FPS). If no event occurs, it returns `RatatuiRuby::Event::None`. This keeps the application responsive.
*   **Blocking:** `RatatuiRuby.poll_event(timeout: nil)` waits **forever** until an event occurs. Use this for scripts that only react to input and do not need to update the UI on a timer.
*   **Non-Blocking:** `RatatuiRuby.poll_event(timeout: 0.0)` returns immediately.

## 2. Symbol and String Comparison (Simplest)

For simple key events, `RatatuiRuby::Event::Key` objects can be compared directly to Symbols or Strings. This is often the quickest way to get started.

*   **String**: Matches the key character (e.g., "a", "q").
*   **Symbol**: Matches special keys (e.g., `:enter`, `:esc`) or modifier combinations (e.g., `:ctrl_c`).

> [!NOTE]
> On macOS, the **Option** key is mapped to `alt`. The **Command** key is typically intercepted by the terminal emulator and may not be sent to the application, or it may be mapped to Meta/Alt depending on your terminal settings.

For a complete list of supported keys, modifiers, and event types, please refer to the [API Documentation for RatatuiRuby::Event](../../lib/ratatui_ruby/event.rb).

<!-- SPDX-SnippetBegin -->
<!--
  SPDX-FileCopyrightText: 2025 Kerrick Long
  SPDX-License-Identifier: MIT-0
-->
```ruby
event = RatatuiRuby.poll_event

# 1. Check for quit keys
if event == "q" || event == :ctrl_c
  break
end

# 2. Check for special key
if event == :enter
  submit_form
end
```
<!-- SPDX-SnippetEnd -->

## 3. Predicate Methods (Intermediate)

If you need more control or logic (e.g. `if/elsif`), or need to handle non-key events like Resize or Mouse, use the predicate methods.

### Polymorphic Predicates

Safe to call on *any* event object. They return `true` only for the matching event type.

Available: `key?`, `mouse?`, `resize?`, `paste?`, `focus_gained?`, `focus_lost?`.

<!-- SPDX-SnippetBegin -->
<!--
  SPDX-FileCopyrightText: 2025 Kerrick Long
  SPDX-License-Identifier: MIT-0
-->
```ruby
event = RatatuiRuby.poll_event

if event.key?
  handle_keypress(event)
elsif event.mouse?
  handle_click(event)
elsif event.resize?
  resize_layout(event.width, event.height)
end
```
<!-- SPDX-SnippetEnd -->

### Helper Predicates

Specific to certain event classes to simplify checks.

#### `RatatuiRuby::Event::Key`
*   `ctrl?`, `alt?`, `shift?`: Check if modifier is held.
*   `text?`: Returns `true` if the event is a printable character (length == 1).
*   `press?`, `repeat?`, `release?`: Check the key event kind. See [Key Event Kinds](#key-event-kinds-press-repeat-release) below.

<!-- SPDX-SnippetBegin -->
<!--
  SPDX-FileCopyrightText: 2025 Kerrick Long
  SPDX-License-Identifier: MIT-0
-->
```ruby
if event.key? && event.ctrl? && event.code == "s"
  save_file
end
```
<!-- SPDX-SnippetEnd -->

#### `RatatuiRuby::Event::Mouse`
*   `down?`, `up?`, `drag?`: Check mouse action.
*   `scroll_up?`, `scroll_down?`: Check scroll direction.

<!-- SPDX-SnippetBegin -->
<!--
  SPDX-FileCopyrightText: 2025 Kerrick Long
  SPDX-License-Identifier: MIT-0
-->
```ruby
if event.mouse? && event.scroll_up?
  scroll_view(-1)
end
```
<!-- SPDX-SnippetEnd -->

## 4. Pattern Matching (Powerful)

For complex applications, Ruby 3.0+ Pattern Matching with the `type:` discriminator is the most idiomatic and concise approach.

<!-- SPDX-SnippetBegin -->
<!--
  SPDX-FileCopyrightText: 2025 Kerrick Long
  SPDX-License-Identifier: MIT-0
-->
```ruby
loop do
  case RatatuiRuby.poll_event
  
  # Match specific key code
  in type: :key, code: "q"
    break

  # Match complex combo
  in type: :key, code: "c", modifiers: ["ctrl"]
    break

  # Capture variables
  in type: :key, code: "up" | "down" => direction
    move_cursor(direction)

  # Match mouse events
  in type: :mouse, kind: "down", x:, y:
    handle_click(x, y)

  in type: :none
    # No event available, continue loop
  end
end
```
<!-- SPDX-SnippetEnd -->

## Key Event Kinds (Press, Repeat, Release)

Every `RatatuiRuby::Event::Key` carries an `event_kind`: `:press`, `:repeat` (the operating system re-sending a held key), or `:release` (the key lifting back up). The `press?`, `repeat?`, and `release?` predicates answer directly, and pattern matching sees the kind via `deconstruct_keys`.

Two conditions gate whether repeat and release events ever arrive:

1.  The terminal must speak the [Kitty keyboard protocol](https://sw.kovidgoyal.net/kitty/keyboard-protocol/) (kitty, WezTerm, ghostty do; Terminal.app, iTerm2, and GNOME Terminal do not).
2.  The application must push the `:report_event_types` flag with `RatatuiRuby::Terminal.push_keyboard_enhancement_flags`, and pop it on the way out.

<!-- SPDX-SnippetBegin -->
<!--
  SPDX-FileCopyrightText: 2026 Kerrick Long
  SPDX-License-Identifier: MIT-0
-->
```ruby
RatatuiRuby.run do
  if RatatuiRuby::Terminal.supports_keyboard_enhancement?
    RatatuiRuby::Terminal.push_keyboard_enhancement_flags(
      :disambiguate, :report_event_types
    )
  end
  begin
    loop do
      event = RatatuiRuby.poll_event

      case event
      in { type: :key, code: "q", event_kind: :press }
        break
      in { type: :key, code: "right", event_kind: :repeat }
        fast_scroll
      in { type: :key, event_kind: :release }
        confirm_action(event.code)
      in { type: :key, event_kind: :press }
        handle_first_press(event.code)
      end
    end
  ensure
    RatatuiRuby::Terminal.pop_keyboard_enhancement_flags
  end
end
```
<!-- SPDX-SnippetEnd -->

> [!NOTE]
> On Windows, crossterm reports both a press and a release event for each keystroke, whether or not the protocol is available. Applications that act on every key event should filter on `event_kind` (or `press?`) so each keystroke acts once.

## Summary of Event Classes

| Event Class | Discriminator (`type:`) | Attributes | Predicate |
| :--- | :--- | :--- | :--- |
| `RatatuiRuby::Event::Key` | `:key` | `code`, `modifiers`, `kind`, `event_kind` | `key?` |
| `RatatuiRuby::Event::Mouse` | `:mouse` | `kind`, `x`, `y`, `button`, `modifiers` | `mouse?` |
| `RatatuiRuby::Event::Resize` | `:resize` | `width`, `height` | `resize?` |
| `RatatuiRuby::Event::Paste` | `:paste` | `content` | `paste?` |
| `RatatuiRuby::Event::FocusGained` | `:focus_gained` | (none) | `focus_gained?` |
| `RatatuiRuby::Event::FocusLost` | `:focus_lost` | (none) | `focus_lost?` |
| `RatatuiRuby::Event::None` | `:none` | (none) | `none?` |
