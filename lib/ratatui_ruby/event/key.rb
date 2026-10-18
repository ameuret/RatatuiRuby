# frozen_string_literal: true

#--
# SPDX-FileCopyrightText: 2026 Kerrick Long <me@kerricklong.com>
# SPDX-License-Identifier: LGPL-3.0-or-later
#++

require_relative "key/character"
require_relative "key/dwim"
require_relative "key/media"
require_relative "key/modifier"
require_relative "key/navigation"
require_relative "key/system"

module RatatuiRuby
  class Event
    # Captures a keyboard interaction.
    #
    # The keyboard is the primary interface for your terminal application. Raw key codes are often cryptic,
    # and handling modifiers manually is error-prone.
    #
    # This event creates clarity. It encapsulates the interaction, providing a normalized +code+ and
    # a list of active +modifiers+.
    #
    # Compare it directly to strings or symbols for rapid development, or use pattern matching for
    # complex control schemes.
    #
    # === Examples
    #
    # Using predicates:
    #--
    # SPDX-SnippetBegin
    # SPDX-FileCopyrightText: 2025 Kerrick Long
    # SPDX-License-Identifier: MIT-0
    #++
    #   if event.key? && event.ctrl? && event.code == "c"
    #     exit
    #   end
    #
    #--
    # SPDX-SnippetEnd
    #++
    # Using symbol comparison:
    #--
    # SPDX-SnippetBegin
    # SPDX-FileCopyrightText: 2025 Kerrick Long
    # SPDX-License-Identifier: MIT-0
    #++
    #   if event == :ctrl_c
    #     exit
    #   end
    #
    #--
    # SPDX-SnippetEnd
    #++
    # Using pattern matching:
    #--
    # SPDX-SnippetBegin
    # SPDX-FileCopyrightText: 2026 Kerrick Long
    # SPDX-License-Identifier: MIT-0
    #++
    #   case event
    #   in type: :key, code: "c", modifiers: ["ctrl"]
    #     exit
    #   end
    #
    #--
    # SPDX-SnippetEnd
    #++
    # === Terminal Compatibility
    #
    # Some key combinations never reach your application. Terminal emulators intercept them for
    # built-in features like tab switching. Common culprits:
    #
    # * Ctrl+PageUp/PageDown (tab switching in Terminal.app, iTerm2)
    # * Ctrl+Tab (tab switching)
    # * Cmd+key combinations (macOS system shortcuts)
    #
    # If modifiers appear missing, test with a different terminal. Kitty, WezTerm, and Alacritty
    # pass more keys through. See <tt>doc/terminal_limitations.md</tt> for details.
    #
    # === Enhanced Keys (Kitty Protocol)
    #
    # Terminals supporting the Kitty keyboard protocol report additional keys:
    #
    # * Media keys: <tt>:play</tt>, <tt>:play_pause</tt>, <tt>:track_next</tt>, <tt>:mute_volume</tt>
    # * Individual modifiers: <tt>:left_shift</tt>, <tt>:right_control</tt>, <tt>:left_super</tt>
    #
    # These keys will not work in Terminal.app, iTerm2, or GNOME Terminal.
    #
    # === Key Event Kinds (Press, Repeat, Release)
    #
    # Every Key event carries an +event_kind+: <tt>:press</tt>,
    # <tt>:repeat</tt> (the operating system re-sending a held key), or
    # <tt>:release</tt> (the key lifting back up). The <tt>press?</tt>,
    # <tt>repeat?</tt>, and <tt>release?</tt> predicates answer directly.
    #
    # Two conditions gate whether repeat and release events ever arrive:
    # the terminal must speak the Kitty keyboard protocol, and the
    # application must push the <tt>:report_event_types</tt> flag via
    # {RatatuiRuby::Terminal.push_keyboard_enhancement_flags}. Without
    # both, every event is a press. On Windows, crossterm reports a
    # release for every keystroke regardless, so filter on
    # <tt>event_kind</tt> before acting on a key.
    class Key < Event
      include Character
      include Dwim
      include Media
      include Modifier
      include Navigation
      include System

      # The key code (e.g., <tt>"a"</tt>, <tt>"enter"</tt>, <tt>"up"</tt>).
      #
      #   puts event.code # => "enter"
      attr_reader :code

      # List of active modifiers (<tt>"ctrl"</tt>, <tt>"alt"</tt>, <tt>"shift"</tt>).
      #
      #   puts event.modifiers # => ["ctrl", "shift"]
      attr_reader :modifiers

      # The category of the key.
      #
      # One of: <tt>:standard</tt>, <tt>:function</tt>, <tt>:media</tt>, <tt>:modifier</tt>, <tt>:system</tt>.
      #
      # This allows grouping keys by their logical type without parsing the code string.
      #
      #   event.kind # => :media
      attr_reader :kind

      # The kind of keyboard event.
      #
      # One of: <tt>:press</tt>, <tt>:repeat</tt>, <tt>:release</tt>.
      #
      # Terminals that report event types distinguish a fresh press from the
      # operating system's auto-repeat stream, and from the key lifting back
      # up. Use it to drive hold-to-scroll behavior or act-on-release
      # interactions.
      #
      #--
      # SPDX-SnippetBegin
      # SPDX-FileCopyrightText: 2026 Kerrick Long
      # SPDX-License-Identifier: MIT-0
      #++
      #   puts event.event_kind # => :release
      #
      #--
      # SPDX-SnippetEnd
      #++
      # Terminals only send <tt>:repeat</tt> and <tt>:release</tt> when both
      # of these hold: the terminal understands the Kitty keyboard protocol,
      # and the application pushed the
      # <tt>:report_event_types</tt> enhancement flag with
      # {RatatuiRuby::Terminal.push_keyboard_enhancement_flags}. On terminals
      # without the protocol, every event arrives as <tt>:press</tt>. On
      # Windows, crossterm reports both a press and a release for each
      # keystroke, so filter on this value before acting.
      attr_reader :event_kind

      # Returns true for Key events.
      #
      #--
      # SPDX-SnippetBegin
      # SPDX-FileCopyrightText: 2025 Kerrick Long
      # SPDX-License-Identifier: MIT-0
      #++
      #   event.key?    # => true
      #   event.mouse?  # => false
      #   event.resize? # => false
      #--
      # SPDX-SnippetEnd
      #++
      def key?
        true
      end

      # Checks whether this event is a fresh key press.
      #
      # Users press keys once and expect one action. Auto-repeat is a
      # separate stream of events, and key release another. Responding to
      # every event as if it were a fresh press makes held keys fire dozens
      # of times.
      #
      # This predicate is +true+ only when +event_kind+ is <tt>:press</tt>.
      # On terminals without the Kitty keyboard protocol (or without the
      # enhancement flags pushed) every event is a press, so this returns
      # +true+ for all of them. On Windows, both a press and a release
      # arrive for each keystroke; filter on this predicate to act once.
      #
      # === Example
      #
      #--
      # SPDX-SnippetBegin
      # SPDX-FileCopyrightText: 2026 Kerrick Long
      # SPDX-License-Identifier: MIT-0
      #++
      #   case event
      #   in type: :key, code: "right" unless event.press?
      #     next # ignore auto-repeat and release
      #   in type: :key, code: "right"
      #     move_cursor_right
      #   end
      #--
      # SPDX-SnippetEnd
      #++
      def press?
        @event_kind == :press
      end

      # Checks whether this event comes from the operating system's
      # auto-repeat.
      #
      # Holding a key down makes the terminal re-send the key on a timer.
      # Terminals that report event types label those re-sends as repeats,
      # so applications can implement their own repeat cadence or count
      # how long a key has been held.
      #
      # This predicate is +true+ only when +event_kind+ is <tt>:repeat</tt>.
      # It requires a Kitty-protocol-capable terminal with the
      # <tt>:report_event_types</tt> enhancement flag pushed; otherwise no
      # repeat events ever arrive.
      #
      # === Example
      #
      #--
      # SPDX-SnippetBegin
      # SPDX-FileCopyrightText: 2026 Kerrick Long
      # SPDX-License-Identifier: MIT-0
      #++
      #   fast_scroll if event.repeat?
      #--
      # SPDX-SnippetEnd
      #++
      def repeat?
        @event_kind == :repeat
      end

      # Checks whether this event is the key lifting back up.
      #
      # Acting on release instead of press opens interactions a press-only
      # terminal cannot offer: confirm-on-release, hold-to-preview, and
      # measuring how long a key was held.
      #
      # This predicate is +true+ only when +event_kind+ is <tt>:release</tt>.
      # It requires a Kitty-protocol-capable terminal with the
      # <tt>:report_event_types</tt> enhancement flag pushed. On Windows,
      # every keystroke reports a release even without the protocol; filter
      # with this predicate rather than assuming.
      #
      # === Example
      #
      #--
      # SPDX-SnippetBegin
      # SPDX-FileCopyrightText: 2026 Kerrick Long
      # SPDX-License-Identifier: MIT-0
      #++
      #   arm_action(event.code) if event.press?
      #   fire_action(event.code) if event.release?
      #--
      # SPDX-SnippetEnd
      #++
      def release?
        @event_kind == :release
      end

      # Creates a new Key event.
      #
      # [code]
      #   The key code (String).
      # [modifiers]
      #   List of modifiers (Array<String>).
      # [kind]
      #--
      # SPDX-SnippetBegin
      # SPDX-FileCopyrightText: 2026 Kerrick Long
      # SPDX-License-Identifier: MIT-0
      #++
      #   The key category (Symbol). One of: <tt>:standard</tt>, <tt>:function</tt>,
      #   <tt>:media</tt>, <tt>:modifier</tt>, <tt>:system</tt>. Defaults to <tt>:standard</tt>.
      # [event_kind]
      #--
      # SPDX-SnippetBegin
      # SPDX-FileCopyrightText: 2026 Kerrick Long
      # SPDX-License-Identifier: MIT-0
      #++
      #   The keyboard event kind (Symbol). One of: <tt>:press</tt>,
      #   <tt>:repeat</tt>, <tt>:release</tt>. Defaults to <tt>:press</tt>.
      #--
      # SPDX-SnippetEnd
      #++
      #--
      # SPDX-SnippetEnd
      #++
      def initialize(code:, modifiers: [], kind: :standard, event_kind: :press)
        @code = code.freeze
        @modifiers = modifiers.map(&:freeze).sort.freeze
        @kind = kind
        @event_kind = event_kind.freeze
      end

      # Compares the event with another object.
      #
      # - If +other+ is a +Symbol+, compares against #to_sym.
      # - If +other+ is a +String+, compares against #to_s.
      # - If +other+ is a +Key+, compares as a value object.
      # - Otherwise, compares using standard equality.
      def ==(other)
        case other
        when Symbol
          to_sym == other
        when String
          to_s == other
        when Key
          code == other.code && modifiers == other.modifiers
        else
          super
        end
      end

      # Converts the event to a Symbol representation.
      #
      # The format is <tt>[modifiers_]code</tt>. Modifiers are sorted alphabetically (alt, ctrl, shift)
      # and joined by underscores.
      #
      # === Supported Keys
      #
      # [Standard]
      #   <tt>:enter</tt>, <tt>:backspace</tt>, <tt>:tab</tt>, <tt>:back_tab</tt>, <tt>:esc</tt>, <tt>:null</tt>
      # [Navigation]
      #--
      # SPDX-SnippetBegin
      # SPDX-FileCopyrightText: 2026 Kerrick Long
      # SPDX-License-Identifier: MIT-0
      #++
      #   <tt>:up</tt>, <tt>:down</tt>, <tt>:left</tt>, <tt>:right</tt>, <tt>:home</tt>, <tt>:end</tt>,
      #   <tt>:page_up</tt>, <tt>:page_down</tt>, <tt>:insert</tt>, <tt>:delete</tt>
      #--
      # SPDX-SnippetEnd
      #++
      # [Function Keys]
      #   <tt>:f1</tt> through <tt>:f12</tt> (and beyond, e.g. <tt>:f24</tt>)
      # [Lock Keys]
      #   <tt>:caps_lock</tt>, <tt>:scroll_lock</tt>, <tt>:num_lock</tt>
      # [System Keys]
      #   <tt>:print_screen</tt>, <tt>:pause</tt>, <tt>:menu</tt>, <tt>:keypad_begin</tt>
      # [Media Keys]
      #--
      # SPDX-SnippetBegin
      # SPDX-FileCopyrightText: 2026 Kerrick Long
      # SPDX-License-Identifier: MIT-0
      #++
      #   <tt>:play</tt>, <tt>:media_pause</tt>, <tt>:play_pause</tt>, <tt>:reverse</tt>, <tt>:stop</tt>,
      #   <tt>:fast_forward</tt>, <tt>:rewind</tt>, <tt>:track_next</tt>, <tt>:track_previous</tt>,
      #   <tt>:record</tt>, <tt>:lower_volume</tt>, <tt>:raise_volume</tt>, <tt>:mute_volume</tt>
      #--
      # SPDX-SnippetEnd
      #++
      # [Modifier Keys]
      #--
      # SPDX-SnippetBegin
      # SPDX-FileCopyrightText: 2026 Kerrick Long
      # SPDX-License-Identifier: MIT-0
      #++
      #   <tt>:left_shift</tt>, <tt>:left_control</tt>, <tt>:left_alt</tt>, <tt>:left_super</tt>,
      #   <tt>:left_hyper</tt>, <tt>:left_meta</tt>, <tt>:right_shift</tt>, <tt>:right_control</tt>,
      #   <tt>:right_alt</tt>, <tt>:right_super</tt>, <tt>:right_hyper</tt>, <tt>:right_meta</tt>,
      #   <tt>:iso_level3_shift</tt>, <tt>:iso_level5_shift</tt>
      #--
      # SPDX-SnippetEnd
      #++
      # [Characters]
      #--
      # SPDX-SnippetBegin
      # SPDX-FileCopyrightText: 2025 Kerrick Long
      # SPDX-License-Identifier: MIT-0
      #++
      #   <tt>:a</tt>, <tt>:b</tt>, <tt>:1</tt>, <tt>:space</tt>, etc.
      #
      #--
      # SPDX-SnippetEnd
      #++
      # === Modifier Examples
      #
      # * <tt>:ctrl_c</tt>
      # * <tt>:alt_enter</tt>
      # * <tt>:shift_left</tt>
      # * <tt>:ctrl_alt_delete</tt>
      def to_sym
        mods = @modifiers.join("_")
        if mods.empty?
          @code.to_sym
        else
          :"#{mods}_#{@code}"
        end
      end

      # Converts the event to its String representation.
      #
      # [Printable Characters]
      #   Returns the character itself (e.g., <tt>"a"</tt>, <tt>"1"</tt>, <tt>" "</tt>).
      # [Special Keys]
      #   Returns an empty string (e.g., <tt>"enter"</tt>, <tt>"up"</tt>, <tt>"f1"</tt> all return <tt>""</tt>).
      # [Modifiers]
      #--
      # SPDX-SnippetBegin
      # SPDX-FileCopyrightText: 2025 Kerrick Long
      # SPDX-License-Identifier: MIT-0
      #++
      #   Returns the character if printable, ignoring modifiers unless they alter the character code itself.
      #   Note that <tt>ctrl+c</tt> typically returns <tt>"c"</tt> as the code, so +to_s+ will return <tt>"c"</tt>.
      #--
      # SPDX-SnippetEnd
      #++
      def to_s
        if text?
          @code
        else
          ""
        end
      end

      # Returns inspection string.
      def inspect
        "#<#{self.class} code=#{@code.inspect} modifiers=#{@modifiers.inspect} kind=#{@kind.inspect}>"
      end

      # Supports dynamic key predicate methods via method_missing.
      #
      # Allows convenient checking for specific keys or key combinations:
      #
      #--
      # SPDX-SnippetBegin
      # SPDX-FileCopyrightText: 2025 Kerrick Long
      # SPDX-License-Identifier: MIT-0
      #++
      #   event.ctrl_c?     # => true if Ctrl+C
      #   event.enter?      # => true if Enter
      #   event.shift_up?   # => true if Shift+Up
      #   event.q?          # => true if "q"
      #
      #--
      # SPDX-SnippetEnd
      #++
      # The method name is converted to a symbol and compared against the event.
      # This works for any key code or modifier+key combination.
      #
      # === Smart Predicates (DWIM)
      #
      # For convenience, generic predicates match both system and media variants:
      #
      #--
      # SPDX-SnippetBegin
      # SPDX-FileCopyrightText: 2026 Kerrick Long
      # SPDX-License-Identifier: MIT-0
      #++
      #   event.pause?      # => true for BOTH system "pause" AND "media_pause"
      #   event.play?       # => true for "media_play"
      #   event.stop?       # => true for "media_stop"
      #
      #--
      # SPDX-SnippetEnd
      #++
      # This "Do What I Mean" behavior reduces boilerplate when you just want to
      # respond to a conceptual action (e.g., "pause the playback") regardless of
      # whether the user pressed a keyboard key or a media button.
      #
      # For strict matching, use the full predicate or compare the code directly:
      #
      #--
      # SPDX-SnippetBegin
      # SPDX-FileCopyrightText: 2026 Kerrick Long
      # SPDX-License-Identifier: MIT-0
      #++
      #   event.media_pause?  # => true ONLY for media pause
      #   event.code == "pause"  # => true ONLY for system pause
      #
      #--
      # SPDX-SnippetEnd
      #++
      # === Arrow Key Aliases
      #
      # Arrow keys respond to <tt>arrow_up?</tt> and <tt>up_arrow?</tt> variants.
      # This disambiguates from Mouse events, which also respond to <tt>up?</tt>
      # and <tt>down?</tt>:
      #
      #--
      # SPDX-SnippetBegin
      # SPDX-FileCopyrightText: 2026 Kerrick Long
      # SPDX-License-Identifier: MIT-0
      #++
      #   event.arrow_up?     # => true for up arrow key
      #   event.up_arrow?     # => true for up arrow key
      #   event.arrow_down?   # => true for down arrow key
      #
      #--
      # SPDX-SnippetEnd
      #++
      # === Key Prefix and Suffix
      #
      # Predicates accept <tt>key_</tt> prefix or <tt>_key</tt> suffix for explicit
      # key event matching in mixed event contexts:
      #
      #--
      # SPDX-SnippetBegin
      # SPDX-FileCopyrightText: 2026 Kerrick Long
      # SPDX-License-Identifier: MIT-0
      #++
      #   event.key_up?       # => true for up arrow key
      #   event.key_q?        # => true for "q" key
      #   event.q_key?        # => true for "q" key
      #   event.enter_key?    # => true for enter key
      #
      #--
      # SPDX-SnippetEnd
      #++
      # === Capital Letters and Shift
      #
      # Capital letter predicates match shifted keys naturally. The terminal reports
      # the produced character with shift in the modifiers:
      #
      #--
      # SPDX-SnippetBegin
      # SPDX-FileCopyrightText: 2026 Kerrick Long
      # SPDX-License-Identifier: MIT-0
      #++
      #   event.G?            # => true for code="G" modifiers=["shift"]
      #   event.shift_g?      # => true for code="G" modifiers=["shift"]
      #   event.alt_B?        # => true for code="B" modifiers=["alt", "shift"]
      #
      #--
      # SPDX-SnippetEnd
      #++
      def method_missing(name, *args, **kwargs, &block)
        if name.to_s.end_with?("?")
          name_str = name.to_s
          key_name = name_str.chop # Returns String, never nil for non-empty string
          key_sym = key_name.to_sym

          # Fast path: Exact match (e.g., media_pause? for media_pause)
          return true if self == key_sym

          # Delegate category-specific DWIM logic to mixins
          return true if match_media_dwim?(key_name)
          return true if match_modifier_dwim?(key_name, key_sym)
          return true if match_navigation_dwim?(key_name, key_sym)
          return true if match_system_dwim?(key_name, key_sym)

          # DWIM: key_ prefix and _key suffix (disambiguate from mouse events)
          # key_up? → up?, q_key? → q?, etc.
          key_name = key_name.delete_prefix("key_").delete_suffix("_key")

          # Fast path after prefix/suffix stripping
          return true if self == key_name.to_sym

          # DWIM: Single character codes match even with shift modifier present
          # G? matches code="G" modifiers=["shift"], B? matches code="B" modifiers=["alt","shift"]
          # @? matches code="@" modifiers=["shift"]
          # The terminal reports the produced character, shift is implicit for these
          if key_name.length == 1 && @code == key_name && @modifiers.include?("shift")
            return true
          end

          # DWIM: Uppercase in predicate implies shift, so alt_B? matches alt_shift_B
          # Parse predicate to extract modifiers and final letter
          if key_name.match?(/\A([a-z_]+_)?([A-Z])\z/)
            pred_letter = key_name[-1]
            pred_mods = key_name.chop.delete_suffix("_").split("_").reject(&:empty?)
            expected_mods = (pred_mods + ["shift"]).sort
            return true if @code == pred_letter && @modifiers == expected_mods
          end

          # DWIM: Case-insensitive letter matching with modifiers
          # shift_g? matches code="G" modifiers=["shift"]
          if @code.length == 1 && @code.match?(/[A-Za-z]/) && (to_sym.to_s.downcase == key_name.downcase)
            return true
          end

          # DWIM: Universal underscore-insensitivity
          # Normalize both predicate and code by stripping underscores
          normalized_predicate = key_name.delete("_")
          normalized_code = @code.delete("_")
          return true if normalized_predicate == normalized_code && @modifiers.empty?

          # DWIM: Underscore variants delegate to existing methods
          # space_bar? → spacebar? → space?, sig_int? → sigint?
          normalized_method = :"#{normalized_predicate}?"
          if normalized_method != name && respond_to?(normalized_method)
            return public_send(normalized_method)
          end

          false
        else
          super
        end
      end

      # Declares that this class responds to dynamic predicate methods.
      def respond_to_missing?(name, *args)
        name.to_s.end_with?("?") || super
      end

      # Deconstructs the event for pattern matching.
      #
      #--
      # SPDX-SnippetBegin
      # SPDX-FileCopyrightText: 2026 Kerrick Long
      # SPDX-License-Identifier: MIT-0
      #++
      #   case event
      #   in type: :key, code: "c", modifiers: ["ctrl"]
      #     puts "Ctrl+C pressed"
      #   in type: :key, kind: :media
      #     puts "Media key pressed"
      #   in type: :key, event_kind: :release
      #     puts "Key released"
      #   end
      #--
      # SPDX-SnippetEnd
      #++
      def deconstruct_keys(keys)
        { type: :key, code: @code, modifiers: @modifiers, kind: @kind, event_kind: @event_kind }
      end
    end
  end
end
