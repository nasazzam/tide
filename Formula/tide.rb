class Tide < Formula
  desc "Focused terminal IDE for code, shells, and AI agents"
  homepage "https://github.com/nasazzam/tide"
  version "0.5.4"
  license "MIT"

  on_macos do
    if Hardware::CPU.arm?
      url "https://github.com/nasazzam/tide/releases/download/v0.5.4/tide-v0.5.4-aarch64-apple-darwin.tar.gz"
      sha256 "74e50e45a1db106bd11e558bb30b27601a28d907c6b33c1f52b19a3344aca7b1"
    else
      url "https://github.com/nasazzam/tide/releases/download/v0.5.4/tide-v0.5.4-x86_64-apple-darwin.tar.gz"
      sha256 "32f7f3c128c6da8c6f415cef2fd38ecb3a9b7cc6c319f6190026b87e91c50ce8"
    end
  end

  depends_on "tmux"

  def install
    bin.install "tide", "tide-editor"
  end

  test do
    assert_match "tide 0.5.4", shell_output("#{bin}/tide --version")
  end
end
