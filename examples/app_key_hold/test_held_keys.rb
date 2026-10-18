# frozen_string_literal: true

#--
# SPDX-FileCopyrightText: 2026 Kerrick Long <me@kerricklong.com>
# SPDX-License-Identifier: AGPL-3.0-or-later
#++

$LOAD_PATH.unshift File.expand_path("../../lib", __dir__)
require "ratatui_ruby"
require "minitest/autorun"
require_relative "held_keys"
require_relative "event_log"

module AppKeyHold
  ##
  # Tests for the HeldKeys tracker and EventLog ring buffer.
  #
  # Both classes are pure: feed them events and timestamps, read state back.
  # These tests drive press, repeat, and release transitions with fixed
  # timestamps so hold durations and release summaries are deterministic.
  class TestHeldKeys < Minitest::Test
    def setup
      @tracker = HeldKeys.new
    end

    def feed(event_kind, code = "right", now: 0.0)
      @tracker.apply(RatatuiRuby::Event::Key.new(code:, event_kind:), now:)
    end

    # --- HeldKeys ---

    def test_press_marks_key_held
      feed(:press, "right", now: 1.0)

      assert @tracker.held?("right")
      refute @tracker.held?("left")
      assert_equal 0, @tracker.repeats("right")
    end

    def test_repeats_accumulate_while_held
      feed(:press)
      feed(:repeat, now: 0.1)
      feed(:repeat, now: 0.25)
      feed(:repeat, now: 0.4)

      assert_equal 3, @tracker.repeats("right")
    end

    def test_repeat_without_press_is_ignored
      feed(:repeat, now: 0.1)

      refute @tracker.held?("right")
      assert_equal 0, @tracker.repeats("right")
    end

    def test_release_closes_the_hold
      feed(:press, now: 10.0)
      feed(:repeat, now: 10.5)
      feed(:release, now: 11.4)

      refute @tracker.held?("right")
      assert_equal 0, @tracker.repeats("right")
      assert_nil @tracker.hold_duration("right", now: 11.5)
    end

    def test_release_without_press_is_ignored
      feed(:release, now: 5.0)

      refute @tracker.held?("right")
      assert_nil @tracker.status_line("right", now: 5.1)
    end

    def test_repress_resets_the_repeat_count
      feed(:press)
      feed(:repeat, now: 0.1)
      feed(:repeat, now: 0.2)
      feed(:release, now: 0.3)
      feed(:press, now: 1.0)

      assert_equal 0, @tracker.repeats("right")
    end

    def test_hold_duration_measures_from_press
      feed(:press, now: 2.0)

      assert_equal 1.4, @tracker.hold_duration("right", now: 3.4)
    end

    def test_status_line_shows_hold_with_repeat_count
      feed(:press)
      feed(:repeat, now: 0.1)
      feed(:repeat, now: 0.1)

      assert_equal "right: HELD (2 repeats)", @tracker.status_line("right", now: 0.3)
    end

    def test_status_line_shows_release_summary_within_the_window
      feed(:press, now: 0.0)
      feed(:repeat, now: 0.1)
      feed(:repeat, now: 0.1)
      feed(:repeat, now: 0.1)
      feed(:release, now: 1.4)

      assert_equal "right: released after 1.4s / 3 repeats",
        @tracker.status_line("right", now: 1.5)
    end

    def test_status_line_hides_stale_releases
      feed(:press)
      feed(:release, now: 1.0)

      assert_nil @tracker.status_line("right", now: 3.1)
    end

    def test_apply_ignores_non_key_events
      @tracker.apply(RatatuiRuby::Event::None.new, now: 0.0)
      @tracker.apply(
        RatatuiRuby::Event::Mouse.new(kind: "down", x: 1, y: 1, button: "left", modifiers: []),
        now: 0.0
      )

      assert_empty @tracker.instance_variable_get(:@holds)
    end

    # --- EventLog ---

    def test_event_log_formats_each_event_as_one_line
      log = EventLog.new(size: 3)
      log.push(RatatuiRuby::Event::Key.new(code: "right", modifiers: []), now: 0.123)
      log.push(
        RatatuiRuby::Event::Key.new(code: "n", event_kind: :release, modifiers: ["ctrl"]),
        now: 1.5
      )

      lines = log.lines
      assert_match(/\A\+1500ms\s+n\s+release\s+mods: \["ctrl"\]/, lines[0])
      assert_match(/\A\+123ms\s+right\s+press\s+mods: \[\]/, lines[1])
    end

    def test_event_log_keeps_only_the_newest_events
      log = EventLog.new(size: 2)
      ("a".."e").each_with_index do |code, index|
        log.push(RatatuiRuby::Event::Key.new(code:), now: index.to_f)
      end

      codes = log.lines.map { |line| line[/\A\+\d+ms\s+(\S+)/, 1] }
      assert_equal %w[e d], codes
    end

    def test_event_log_ignores_non_key_events
      log = EventLog.new(size: 2)
      log.push(RatatuiRuby::Event::None.new, now: 0.0)

      assert_empty log.lines
    end
  end
end
