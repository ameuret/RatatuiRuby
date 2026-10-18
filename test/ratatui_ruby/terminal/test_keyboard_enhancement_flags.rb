# frozen_string_literal: true

#--
# SPDX-FileCopyrightText: 2026 Kerrick Long <me@kerricklong.com>
# SPDX-License-Identifier: LGPL-3.0-or-later
#++

require "test_helper"

module RatatuiRuby
  ##
  # Tests for pushing and popping Kitty keyboard protocol enhancement flags.
  #
  # Verifies argument validation, the no-active-terminal error, and that the
  # pair is a harmless no-op under the test backend so applications can run
  # the same code under test as in production.
  class TestKeyboardEnhancementFlags < Minitest::Test
    include RatatuiRuby::TestHelper

    def test_push_rejects_unknown_flag
      error = assert_raises ArgumentError do
        Terminal.push_keyboard_enhancement_flags(:report_vibes)
      end

      assert_includes error.message, "report_vibes"
    end

    def test_push_rejects_empty_flag_list
      error = assert_raises ArgumentError do
        Terminal.push_keyboard_enhancement_flags
      end

      assert_includes error.message, "At least one"
    end

    def test_push_explains_that_associated_text_is_unimplemented
      error = assert_raises ArgumentError do
        Terminal.push_keyboard_enhancement_flags(:report_associated_text)
      end

      assert_includes error.message, "not supported"
    end

    def test_push_without_active_terminal_raises
      error = assert_raises Error::Terminal do
        Terminal.push_keyboard_enhancement_flags(:disambiguate, :report_event_types)
      end

      assert_includes error.message, "not initialized"
    end

    def test_pop_without_active_terminal_raises
      error = assert_raises Error::Terminal do
        Terminal.pop_keyboard_enhancement_flags
      end

      assert_includes error.message, "not initialized"
    end

    def test_push_and_pop_succeed_under_test_terminal
      with_test_terminal do
        Terminal.push_keyboard_enhancement_flags(
          :disambiguate, :report_event_types, :report_alternate_keys,
          :report_all_keys_as_escape_codes
        )
        Terminal.pop_keyboard_enhancement_flags
      end
    end

    def test_push_accepts_string_flags
      with_test_terminal do
        Terminal.push_keyboard_enhancement_flags("disambiguate", "report_event_types")
        Terminal.pop_keyboard_enhancement_flags
      end
    end

    def test_push_during_draw_raises_invariant
      with_test_terminal do
        RatatuiRuby.draw do
          error = assert_raises Error::Invariant do
            Terminal.push_keyboard_enhancement_flags(:disambiguate)
          end

          assert_includes error.message, "during draw"
        end
      end
    end

    def test_pop_during_draw_raises_invariant
      with_test_terminal do
        RatatuiRuby.draw do
          error = assert_raises Error::Invariant do
            Terminal.pop_keyboard_enhancement_flags
          end

          assert_includes error.message, "during draw"
        end
      end
    end
  end
end
