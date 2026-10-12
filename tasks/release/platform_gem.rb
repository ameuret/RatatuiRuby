# frozen_string_literal: true

#--
# SPDX-FileCopyrightText: 2026 Kerrick Long <me@kerricklong.com>
# SPDX-License-Identifier: AGPL-3.0-or-later
#++

require "rubygems"
require "rubygems/package"
require "fileutils"

require_relative "versioned_binary"

# A platform-specific gem assembled from pre-compiled versioned binaries.
#
# CI builds produce one compiled binary per Ruby version per OS.
# A PlatformGem collects those binaries and packages them into a single
# installable gem that serves all supported Ruby versions.
class PlatformGem
  LIB_DIR = "lib/ratatui_ruby"

  def initialize(spec = Gem::Specification.load("ratatui_ruby.gemspec"))
    @spec = spec
    @binaries = VersionedBinary.scan(LIB_DIR)
  end

  def build
    abort "No versioned binaries found in #{LIB_DIR}/*/" if @binaries.empty?

    @spec.platform = platform
    @spec.extensions.clear
    @spec.files += @binaries.map(&:path)

    #--
    # SPDX-SnippetBegin
    # SPDX-SnippetCopyrightText: rake-compiler contributors
    # SPDX-License-Identifier: MIT
    #
    # Version constraint pattern derived from rake-compiler's
    # ExtensionTask#define_native_tasks (lib/rake/extensiontask.rb).
    #++
    @spec.required_ruby_version = [
      ">= #{@binaries.first.api_version}",
      "< #{@binaries.last.api_version.succ}.dev",
    ]
    #--
    # SPDX-SnippetEnd
    #++

    FileUtils.mkdir_p("pkg")
    gem_file = Gem::Package.build(@spec)
    FileUtils.mv(gem_file, "pkg")
    puts "Built pkg/#{File.basename(gem_file)}"
  end

  # The platform of the packaging Ruby, without a darwin version.
  #
  # RubyGems installs an <tt>arm64-darwin-23</tt> gem only on a Ruby that
  # reports exactly darwin 23. That number names the CI image's macOS, not
  # anything the binary needs. The binary targets macOS 11, the first
  # release on Apple Silicon, so <tt>arm64-darwin</tt> fits every Mac
  # that can run it.
  private def platform
    local = Gem::Platform.local
    return local unless local.os == "darwin"

    Gem::Platform.new([local.cpu, local.os, nil])
  end
end

if $PROGRAM_NAME == __FILE__
  PlatformGem.new.build
end
