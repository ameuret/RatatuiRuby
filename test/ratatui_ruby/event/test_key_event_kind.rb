# frozen_string_literal: true

#--
# SPDX-FileCopyrightText: 2026 Kerrick Long <me@kerricklong.com>
# SPDX-License-Identifier: LGPL-3.0-or-later
#++

require "test_helper"

module RatatuiRuby
  ##
  # Tests for the Key `event_kind` attribute and its predicates.
  #
  # Verifies that press, repeat, and release survive the round trip from
  # injected event to polled Event::Key, that the press?/repeat?/release?
  # predicates agree with event_kind, and that pattern matching sees it.
  class TestKeyEventKind < Minitest::Test
    include RatatuiRuby::TestHelper

    # --- Defaults ---

    def test_event_kind_defaults_to_press
      event = Event::Key.new(code: "right")

      assert_equal :press, event.event_kind
      assert_predicate event, :press?
      refute_predicate event, :repeat?
      refute_predicate event, :release?
    end

    def test_constructor_freezes_event_kind
      event = Event::Key.new(code: "right", event_kind: "repeat")

      assert_equal "repeat", event.event_kind
      assert_predicate event.event_kind, :frozen?
    end

    # --- Each kind round-trips through injection ---

    def test_inject_and_poll_each_event_kind
      with_test_terminal do
        {
          press: [true, false, false],
          repeat: [false, true, false],
          release: [false, false, true],
        }.each do |kind, (press, repeat, release)|
          inject_keys({ code: "right", event_kind: kind })
          event = RatatuiRuby.poll_event

          assert_predicate event, :key?
          assert_equal kind, event.event_kind, "wrong event_kind for #{kind}"
          assert_equal press, event.press?, "press? disagrees for #{kind}"
          assert_equal repeat, event.repeat?, "repeat? disagrees for #{kind}"
          assert_equal release, event.release?, "release? disagrees for #{kind}"
        end
      end
    end

    def test_inject_event_carries_event_kind
      with_test_terminal do
        inject_event(Event::Key.new(code: "n", event_kind: :release))
        event = RatatuiRuby.poll_event

        assert_equal :release, event.event_kind
        assert_predicate event, :release?
      end
    end

    def test_inject_keys_hash_form_without_event_kind_is_press
      with_test_terminal do
        inject_keys({ code: "n" })
        event = RatatuiRuby.poll_event

        assert_equal :press, event.event_kind
        assert_predicate event, :press?
      end
    end

    def test_injected_key_object_keeps_its_event_kind
      with_test_terminal do
        inject_keys(Event::Key.new(code: "down", event_kind: :repeat))
        event = RatatuiRuby.poll_event

        assert_equal :repeat, event.event_kind
        assert_predicate event, :repeat?
      end
    end

    # --- Injection boundary rejects unknown kinds ---

    def test_inject_unknown_event_kind_raises
      with_test_terminal do
        error = assert_raises ArgumentError do
          RatatuiRuby.inject_test_event("key", { code: "n", event_kind: "held" })
        end

        assert_includes error.message, "held"
      end
    end

    # --- deconstruct_keys and pattern matching ---

    def test_deconstruct_keys_includes_event_kind
      event = Event::Key.new(code: "right", event_kind: :repeat)
      hash = event.deconstruct_keys(nil)

      assert_equal :repeat, hash[:event_kind]
      assert_equal :key, hash[:type]
      assert_equal "right", hash[:code]
      assert_equal :standard, hash[:kind]
    end

    def test_pattern_matching_on_event_kind
      event = Event::Key.new(code: "right", event_kind: :release)

      result = case event
              in { type: :key, event_kind: :release }
                :matched_release
              else
                :no_match
      end

      assert_equal :matched_release, result
    end

    # --- Other code paths are unchanged ---

    def test_event_kind_does_not_disturb_code_or_category_kind
      with_test_terminal do
        inject_keys({ code: "media_play", kind: :media, event_kind: :release })
        event = RatatuiRuby.poll_event

        assert_equal "media_play", event.code
        assert_equal :media, event.kind
        assert_predicate event, :media?
        assert_equal :release, event.event_kind
      end
    end

    def test_non_key_events_do_not_expose_event_kind_predicates
      none = Event::None.new
      refute_predicate none, :press?
      refute_predicate none, :repeat?
      refute_predicate none, :release?

      resize = Event::Resize.new(width: 80, height: 24)
      refute_predicate resize, :press?
      refute_predicate resize, :repeat?
      refute_predicate resize, :release?
    end
  end
end
