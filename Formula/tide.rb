class Tide < Formula
  desc "Focused terminal IDE for code, shells, and AI agents"
  homepage "https://github.com/nasazzam/tide"
  version "0.5.0"
  license "MIT"

  on_macos do
    if Hardware::CPU.arm?
      url "https://github.com/nasazzam/tide/releases/download/v0.5.0/tide-v0.5.0-aarch64-apple-darwin.tar.gz"
      sha256 "70ec94c1710e0839d0bd32872ee3628601cba888c8b88422eba644df7fca9e04"
    else
      url "https://github.com/nasazzam/tide/releases/download/v0.5.0/tide-v0.5.0-x86_64-apple-darwin.tar.gz"
      sha256 "bd73c151472b9867bee64ee9c9a3812a17fe73c85b188ee9584ecf8a3ea26452"
    end
  end

  depends_on "tmux"

  def install
    bin.install "tide", "tide-editor"
  end

  test do
    assert_match "tide 0.5.0", shell_output("#{bin}/tide --version")
  end
end
