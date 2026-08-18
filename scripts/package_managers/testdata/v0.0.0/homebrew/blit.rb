class Blit < Formula
  desc "High-performance file transfer CLI and daemon"
  homepage "https://github.com/roethlar/Blit"
  url "https://github.com/roethlar/Blit/archive/refs/tags/v0.0.0.tar.gz"
  sha256 "dddddddddddddddddddddddddddddddddddddddddddddddddddddddddddddddd"
  license "MIT"

  depends_on "rust" => :build

  def install
    ENV["BLIT_GIT_SHA"] = "abcdef012345"
    system "cargo", "build", "--release", "--locked", "-p", "blit-transfer", "-p", "blit-daemon"
    bin.install "target/release/blit", "target/release/blit-daemon"
  end

  test do
    assert_match version.to_s, shell_output("#{bin}/blit --version")
    assert_match "abcdef012345", shell_output("#{bin}/blit --version")
    assert_match version.to_s, shell_output("#{bin}/blit-daemon --version")
  end
end
