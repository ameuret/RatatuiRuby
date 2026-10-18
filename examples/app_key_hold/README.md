<!--
  SPDX-FileCopyrightText: 2026 Kerrick Long <me@kerricklong.com>
  SPDX-License-Identifier: CC-BY-SA-4.0
-->

# Key Hold Example

Live check for key event kinds: press, repeat, and release.

This example demonstrates **Key Event Kinds** and **Keyboard Enhancement Flags**:

1.  **Event kinds:** Every `RatatuiRuby::Event::Key` carries an `event_kind` of `:press`, `:repeat`, or `:release`, with matching `press?`/`repeat?`/`release?` predicates.
2.  **Enhancement flags:** Terminals only send repeat and release events when the application pushes the Kitty keyboard protocol's `:report_event_types` flag via `RatatuiRuby::Terminal.push_keyboard_enhancement_flags`.

## What You Should See

Run it in a protocol-capable terminal (kitty, WezTerm, ghostty, Alacritty 0.17+):

<!-- SPDX-SnippetBegin -->
<!--
  SPDX-FileCopyrightText: 2026 Kerrick Long
  SPDX-License-Identifier: MIT-0
-->
```
bin/agent_rake compile
bundle exec ruby examples/app_key_hold/app.rb
```
<!-- SPDX-SnippetEnd -->

*   Hold the right arrow for two seconds. The log shows one press, a stream of repeats, then a release within milliseconds of letting go. The status line counts repeats live: `right: HELD (37 repeats)`.
*   Tap `n`. A press is followed immediately by a release, and the status line briefly shows `n: released after 0.1s / 0 repeats`. Plain-text keys like `n` only report repeat and release when `:report_all_keys_as_escape_codes` is pushed alongside `:report_event_types`, which is why this app pushes both.
*   The header states whether the terminal reported Kitty protocol support. Without it, every event shows as `press` — that means the terminal is incapable, not that the tool is broken. Try the same hold in a terminal without the protocol (e.g. Terminal.app or GNOME Terminal): a stream of indistinguishable presses, with no repeats and no release.

The push is paired with a pop in an `ensure` block, so the terminal's keyboard state is restored even when the app crashes.

## Hotkeys

| Key | Action |
| --- | --- |
| Any key | Logged with its event kind; tracked while held |
| `q` | Quit |
| `ctrl+c` | Quit |

## Where the Logic Lives

`held_keys.rb` holds the pure tracker (which keys are held, repeat counts, release durations) and `event_log.rb` holds the scrolling log. Both are tested in `test_held_keys.rb` with injected events and fixed timestamps; `app.rb` stays a thin shell.

[Read the source code →](app.rb)
