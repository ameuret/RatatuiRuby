# frozen_string_literal: true

#--
# SPDX-FileCopyrightText: 2026 Kerrick Long <me@kerricklong.com>
# SPDX-License-Identifier: AGPL-3.0-or-later
#++

module AppKeyHold
  # Tracks which keys are currently held.
  #
  # Key event kinds turn a stream of key events into an answerable question:
  # is the user still holding this key, how long, and how many auto-repeats
  # landed while they did? Terminals that report event kinds answer it
  # directly; deriving it from press-only events is guesswork.
  #
  # This tracker answers it from the event kinds. Feed it every key event
  # with the time it arrived, then read the state back.
  #
  # It is pure bookkeeping with no terminal access, so tests drive it with
  # hand-built events and fixed timestamps.
  class HeldKeys
    # One key currently down: when it went down and how many repeats landed.
    Hold = Data.define(:code, :pressed_at, :repeats)

    # One completed hold: how long it lasted and how many repeats landed.
    Release = Data.define(:code, :duration, :repeats, :released_at)

    # Seconds a release stays interesting enough to show on screen.
    RELEASE_NOTE_WINDOW = 2.0

    # Creates an empty tracker.
    def initialize
      @holds = {}
      @releases = {}
    end

    # Applies one key event at +now+ (seconds, monotonic).
    #
    # Press records a fresh hold, repeat counts it, release closes the hold
    # and remembers how long it lasted. Non-key events are ignored.
    def apply(event, now:)
      return unless event.key?

      code = event.code
      case event.event_kind
      when :press
        @holds[code] = Hold.new(code:, pressed_at: now, repeats: 0)
      when :repeat
        hold = @holds[code]
        @holds[code] = hold.with(repeats: hold.repeats + 1) if hold
      when :release
        hold = @holds.delete(code)
        return unless hold

        @releases[code] = Release.new(
          code:,
          duration: now - hold.pressed_at,
          repeats: hold.repeats,
          released_at: now
        )
      end
    end

    # Checks whether +code+ is currently held.
    def held?(code)
      @holds.key?(code)
    end

    # Counts auto-repeats that landed while +code+ is held.
    #
    # Returns 0 for keys that are not held or that never repeated.
    def repeats(code)
      @holds[code]&.repeats || 0
    end

    # Measures how long +code+ has been held at +now+ (seconds).
    #
    # Returns nil when the key is not held.
    def hold_duration(code, now:)
      hold = @holds[code]
      return nil unless hold

      now - hold.pressed_at
    end

    # Builds one status line for +code+.
    #
    # While held: <tt>right: HELD (37 repeats)</tt>. Briefly after release:
    # <tt>right: released after 1.4s / 37 repeats</tt>. Returns nil when the
    # key is neither held nor recently released.
    def status_line(code, now:)
      hold = @holds[code]
      if hold
        "#{code}: HELD (#{hold.repeats} repeats)"
      else
        release = @releases[code]
        return nil unless release && now - release.released_at <= RELEASE_NOTE_WINDOW

        "#{code}: released after #{format('%.1f', release.duration)}s / #{release.repeats} repeats"
      end
    end
  end
end
