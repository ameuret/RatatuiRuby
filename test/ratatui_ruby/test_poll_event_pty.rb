# frozen_string_literal: true

#--
# SPDX-FileCopyrightText: 2026 Kerrick Long <me@kerricklong.com>
# SPDX-License-Identifier: LGPL-3.0-or-later
#++

require "test_helper"
require "tmpdir"

begin
  require "pty"
rescue LoadError
  # PTY is unavailable; the test skips.
end

##
# End-to-end proof of the released crossterm parse path on Unix.
#
# Injection tests cover the queue, not the parser. This test runs a real
# child process on a real pseudo-terminal, writes Kitty keyboard protocol
# escape sequences to the master side, and asserts which key codes and
# event kinds the child observed. It is the only test that proves repeat
# and release events survive the whole path from terminal bytes to Ruby
# event objects.
class TestPollEventPty < Minitest::Test
  SEQUENCES = [
    ["legacy right press", "\e[C"],
    ["kitty right press", "\e[1;1:1C"],
    ["kitty right repeat", "\e[1;1:2C"],
    ["kitty right release", "\e[1;1:3C"],
    ["kitty n press", "\e[110;1:1u"],
    ["kitty n release", "\e[110;1:3u"],
    ["quit", "q"],
  ].freeze

  EXPECTED_KEYS = [
    ["right", :press],
    ["right", :press],
    ["right", :repeat],
    ["right", :release],
    ["n", :press],
    ["n", :release],
    ["q", :press],
  ].freeze

  WAIT_TIMEOUT = 15.0

  def test_crossterm_parse_path_reports_every_event_kind
    skip "Windows has no PTY module" if Gem.win_platform?
    skip "PTY module unavailable" unless defined?(PTY)

    Dir.mktmpdir("ratatui_ruby_pty") do |tmpdir|
      log_path = File.join(tmpdir, "child.log")
      script_path = File.join(tmpdir, "child.rb")
      File.write(script_path, child_script(log_path))

      lib_dir = File.expand_path("../../lib", __dir__)
      PTY.spawn(RbConfig.ruby, "-I", lib_dir, script_path) do |read_io, write_io, pid|
        drain = Thread.new do
          loop do
            read_io.readpartial(4096)
          rescue
            break
          end
        end

        begin
          wait_until("child to start") { File.exist?(log_path) && File.read(log_path).include?("READY") }

          SEQUENCES.each do |_label, sequence|
            write_io.write(sequence)
            sleep 0.05
          end

          wait_until("child to finish") { File.read(log_path).include?("DONE") }
        ensure
          drain.kill
          begin
            Process.kill("TERM", pid)
          rescue
            nil
          end
        end
      end

      observed = File.readlines(log_path).grep(/Event::Key/).map do |line|
        [line[/code="(.*?)"/, 1], line[/event_kind=:(\w+)/, 1].to_sym]
      end

      assert_equal EXPECTED_KEYS, observed
    end
  end

  private def child_script(log_path)
    <<~RUBY
      require "ratatui_ruby"

      log = File.open(#{log_path.inspect}, "a")
      log.sync = true

      RatatuiRuby.run do |tui|
        log.puts "READY"
        30.times do
          event = tui.poll_event(timeout: 5)
          code = event.respond_to?(:code) ? event.code.inspect : "-"
          event_kind = event.respond_to?(:event_kind) ? event.event_kind.inspect : "-"
          modifiers = event.respond_to?(:modifiers) ? event.modifiers.inspect : "-"
          log.puts("EVENT \#{event.class} code=\#{code} event_kind=\#{event_kind} " \\
                   "modifiers=\#{modifiers}")
          break if event.is_a?(RatatuiRuby::Event::Key) && event.code == "q"
        end
        log.puts "DONE"
      end
    RUBY
  end

  # Polls a condition until it holds, and fails instead of hanging forever.
  private def wait_until(description)
    deadline = Process.clock_gettime(Process::CLOCK_MONOTONIC) + WAIT_TIMEOUT
    until yield
      flunk "Timed out waiting for #{description}" if Process.clock_gettime(Process::CLOCK_MONOTONIC) > deadline
      sleep 0.05
    end
  end
end
