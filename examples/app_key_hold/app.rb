# frozen_string_literal: true

#--
# SPDX-FileCopyrightText: 2026 Kerrick Long <me@kerricklong.com>
# SPDX-License-Identifier: AGPL-3.0-or-later
#++

$LOAD_PATH.unshift File.expand_path("../../lib", __dir__)
$LOAD_PATH.unshift File.expand_path(__dir__)

require "ratatui_ruby"
require_relative "held_keys"
require_relative "event_log"

module AppKeyHold
  # Live check for key event kinds: press, repeat, and release.
  #
  # Hold-to-fast-forward and act-on-release only work when repeat and
  # release events actually arrive. Whether they do depends on the terminal
  # and on pushing the Kitty keyboard protocol's report_event_types flag.
  #
  # This app pushes the flags when the terminal supports them, then shows
  # every key event it receives with its event kind, and tracks which keys
  # are currently held. Run it in kitty, WezTerm, or ghostty and hold a key:
  # a press, a stream of repeats, and a release should appear, and the
  # status line should show the hold live.
  class App
    # Keys with their own status line at the bottom of the screen.
    INTERESTING_KEYS = %w[right left up down n p].freeze

    # Number of events kept in the scrolling log.
    LOG_SIZE = 20

    # Creates the app with empty hold tracking and event log.
    def initialize
      @held_keys = HeldKeys.new
      @event_log = EventLog.new(size: LOG_SIZE)
      @enhanced = false
    end

    # Runs the app until the user quits.
    def run
      RatatuiRuby.run do |tui|
        @tui = tui
        @start = Process.clock_gettime(Process::CLOCK_MONOTONIC)
        @enhanced = RatatuiRuby::Terminal.supports_keyboard_enhancement?
        if @enhanced
          # report_all_keys_as_escape_codes is required for plain-text keys
          # (n, p) to report repeat and release; the arrows report them with
          # report_event_types alone.
          RatatuiRuby::Terminal.push_keyboard_enhancement_flags(
            :disambiguate, :report_event_types, :report_all_keys_as_escape_codes
          )
        end
        begin
          loop do
            render
            break if handle_input == :quit
          end
        ensure
          RatatuiRuby::Terminal.pop_keyboard_enhancement_flags if @enhanced
        end
      end
    end

    private def elapsed
      Process.clock_gettime(Process::CLOCK_MONOTONIC) - @start
    end

    private def handle_input
      event = @tui.poll_event
      return :continue if event.none?

      @held_keys.apply(event, now: elapsed)
      @event_log.push(event, now: elapsed)

      case event
      in { type: :key, code: "q", event_kind: :press }
        :quit
      in { type: :key, code: "c", modifiers: ["ctrl"], event_kind: :press }
        :quit
      else
        :continue
      end
    end

    private def render
      @tui.draw do |frame|
        header_area, log_area, status_area = @tui.layout_split(
          frame.area,
          direction: :vertical,
          constraints: [
            @tui.constraint_length(2),
            @tui.constraint_fill(1),
            @tui.constraint_length(INTERESTING_KEYS.size),
          ]
        )

        frame.render_widget(@tui.paragraph(text: header_text), header_area)
        frame.render_widget(@tui.paragraph(text: @event_log.lines.join("\n")), log_area)
        frame.render_widget(@tui.paragraph(text: status_text), status_area)
      end
    end

    private def header_text
      status = if @enhanced
        "Kitty protocol: supported. Hold a key: press, a stream of repeats, then a release."
      else
        "Kitty protocol: NOT detected. Every event shows as press; repeat/release " \
          "need a protocol-capable terminal (kitty, WezTerm, ghostty)."
      end
      "#{status}  q or ctrl+c quits."
    end

    private def status_text
      now = elapsed
      INTERESTING_KEYS.filter_map { |code| @held_keys.status_line(code, now:) }.join("\n")
    end
  end
end

AppKeyHold::App.new.run if $PROGRAM_NAME == __FILE__
