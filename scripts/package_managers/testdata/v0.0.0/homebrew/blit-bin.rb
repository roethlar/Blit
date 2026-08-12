class BlitBin < Formula
  desc "High-performance file transfer CLI and daemon"
  homepage "https://github.com/roethlar/Blit"
  version "0.0.0"
  license "MIT"

  on_macos do
    on_arm do
      url "https://github.com/roethlar/Blit/releases/download/v0.0.0/blit-aarch64-apple-darwin.tar.gz"
      sha256 "bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb"
    end
    on_intel do
      odie "blit-bin ships only the aarch64-apple-darwin archive"
    end
  end

  def install
    bin.install "blit", "blit-daemon"
  end

  test do
    assert_match version.to_s, shell_output("#{bin}/blit --version")
    assert_match version.to_s, shell_output("#{bin}/blit-daemon --version")
  end
end
