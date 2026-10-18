# frozen_string_literal: true

#--
# SPDX-FileCopyrightText: 2025 Kerrick Long <me@kerricklong.com>
# SPDX-License-Identifier: AGPL-3.0-or-later
#++

require "minitest/test_task"

namespace :cargo do
  desc "Run cargo tests"
  task :test do
    # Tests that embed a Ruby VM are a process-wide singleton bound to the
    # thread that initialized it, so all tests run serially on one thread.
    # Embedding also needs a shared libruby to link: static libruby builds
    # (mise's prebuilt rubies) embed YJIT's Rust runtime, which cannot be
    # linked into a Rust binary. Enable the feature only when the active
    # Ruby provides one, and point the loader at the libdir for platforms
    # that link libruby as a shared library.
    ruby_libdir = RbConfig::CONFIG["libdir"]
    shared_libruby = RbConfig::CONFIG["LIBRUBY_SO"].then do |libruby_so|
      !libruby_so.to_s.empty? && File.exist?(File.join(ruby_libdir, libruby_so))
    end
    features = shared_libruby ? "--features embed-ruby-tests" : ""
    unless shared_libruby
      puts "Note: Ruby #{RbConfig::CONFIG['RUBY_PROGRAM_VERSION']} has no shared libruby; " \
        "embedded-VM Rust tests are skipped."
    end
    env = {
      "LD_LIBRARY_PATH" => [ruby_libdir, ENV["LD_LIBRARY_PATH"]].compact.reject(&:empty?).join(File::PATH_SEPARATOR),
      "DYLD_LIBRARY_PATH" => [ruby_libdir, ENV["DYLD_LIBRARY_PATH"]].compact.reject(&:empty?).join(File::PATH_SEPARATOR),
    }
    sh(env, "cd ext/ratatui_ruby && cargo test #{features} -- --test-threads=1")
  end
end

# Clear the default test task created by Minitest::TestTask if it exists
Rake::Task["test"].clear if Rake::Task.task_defined?("test")

desc "Run all tests (Ruby and Rust)"
task test: %w[compile test:ruby test:rust]

namespace :test do
  desc "Run Rust tests"
  task :rust do
    Rake::Task["cargo:test"].invoke
  end

  # Create a specific Minitest task for Ruby tests
  Minitest::TestTask.create(:ruby) do |t|
    t.test_globs = ["test/**/*.rb", "examples/**/test_*.rb"]
  end

  # Regenerate predicate tests before running Ruby tests
  Rake::Task["test:ruby"].enhance(["rbs:tests"])
end
