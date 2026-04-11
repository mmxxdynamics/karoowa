# Homebrew formula for Karoowa.
#
# This is a placeholder — the real formula will be hosted in the
# karoowa/homebrew-tap repository. Update the URLs and SHA256 hashes
# when the first release is published.
#
# Usage:
#   brew install karoowa/tap/karoowa

class Karoowa < Formula
  desc "Agent-native, Rust-based blockchain framework"
  homepage "https://karoowa.io"
  version "0.0.1"

  on_macos do
    on_arm do
      url "https://github.com/karoowa/karoowa/releases/download/v#{version}/karoowa-v#{version}-aarch64-apple-darwin.tar.gz"
      sha256 "TODO"
    end
    on_intel do
      url "https://github.com/karoowa/karoowa/releases/download/v#{version}/karoowa-v#{version}-x86_64-apple-darwin.tar.gz"
      sha256 "TODO"
    end
  end

  on_linux do
    on_arm do
      url "https://github.com/karoowa/karoowa/releases/download/v#{version}/karoowa-v#{version}-aarch64-unknown-linux-musl.tar.gz"
      sha256 "TODO"
    end
    on_intel do
      url "https://github.com/karoowa/karoowa/releases/download/v#{version}/karoowa-v#{version}-x86_64-unknown-linux-musl.tar.gz"
      sha256 "TODO"
    end
  end

  def install
    bin.install "karoowa"
  end

  test do
    assert_match "karoowa", shell_output("#{bin}/karoowa --version")
  end
end
