# frozen_string_literal: true

#--
# SPDX-FileCopyrightText: 2026 Kerrick Long <me@kerricklong.com>
# SPDX-License-Identifier: AGPL-3.0-or-later
#++

module AppKeyHold
  # Keeps the most recent key events for display.
  #
  # Watching key events stream by is the fastest way to trust them. A
  # scrolling log of the last events, with their event kinds, shows press,
  # repeat, and release arriving in order while a key is held.
  #
  # This ring buffer stores the newest events and renders each as one line,
  # newest first.
  class EventLog
    # One logged key event: when it arrived and what it carried.
    Entry = Data.define(:at, :code, :event_kind, :modifiers)

    # Creates an empty log holding at most +size+ events.
    def initialize(size: 20, start: 0.0)
      @entries = []
      @size = size
      @start = start
    end

    # Records one key event at +now+ (seconds since start).
    #
    # Non-key events are ignored. The oldest entry is dropped once the log
    # is full.
    def push(event, now:)
      return unless event.key?

      @entries.unshift(
        Entry.new(at: now, code: event.code, event_kind: event.event_kind, modifiers: event.modifiers)
      )
      @entries.slice!(@size..) if @entries.size > @size
    end

    # Renders each logged event as one line, newest first.
    #
    # Format: <tt>+123ms  right  press   mods: []</tt>
    def lines
      @entries.map do |entry|
        format("+%<milli>dms  %<code>-6s  %<kind>s   mods: %<mods>s",
          milli: ((entry.at - @start) * 1000).round,
          code: entry.code,
          kind: entry.event_kind,
          mods: entry.modifiers.inspect)
      end
    end
  end
end
