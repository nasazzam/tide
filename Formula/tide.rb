class Tide < Formula
  desc "Focused terminal IDE for code, shells, and AI agents"
  homepage "https://github.com/nasazzam/tide"
  version "0.4.0"
  license "MIT"

  on_macos do
    if Hardware::CPU.arm?
      url "https://github.com/nasazzam/tide/releases/download/v0.4.0/tide-v0.4.0-aarch64-apple-darwin.tar.gz"
      sha256 "8b8181cf9ec4ef485fb0997e2cc8d61bd39e860bae99df7cbf74e08356035e7f"
    else
      url "https://github.com/nasazzam/tide/releases/download/v0.4.0/tide-v0.4.0-x86_64-apple-darwin.tar.gz"
      sha256 "37578aa619c957d039cee6f70f132b2d70fedcaa658fd53f69fadf4ccce2af9b"
    end
  end

  depends_on "tmux"

  def install
    bin.install "tide", "tide-editor"
  end

  test do
    assert_match "tide 0.4.0", shell_output("#{bin}/tide --version")
  end
end
