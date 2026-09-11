class Tide < Formula
  desc "Focused terminal IDE for code, shells, and AI agents"
  homepage "https://github.com/nasazzam/tide"
  version "0.4.4"
  license "MIT"

  on_macos do
    if Hardware::CPU.arm?
      url "https://github.com/nasazzam/tide/releases/download/v0.4.4/tide-v0.4.4-aarch64-apple-darwin.tar.gz"
      sha256 "8c6d0fab83424a73350351516d19255cf21855217038e6d346f1428694d8933c"
    else
      url "https://github.com/nasazzam/tide/releases/download/v0.4.4/tide-v0.4.4-x86_64-apple-darwin.tar.gz"
      sha256 "a8c03536b59eb3aaba011c07d6cf3c2313f62f24dc0a3794bbc839a5783c9114"
    end
  end

  depends_on "tmux"

  def install
    bin.install "tide", "tide-editor"
  end

  test do
    assert_match "tide 0.4.4", shell_output("#{bin}/tide --version")
  end
end
