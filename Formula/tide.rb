class Tide < Formula
  desc "Focused terminal IDE for code, shells, and AI agents"
  homepage "https://github.com/nasazzam/tide"
  version "0.2.0"
  license "MIT"

  on_macos do
    if Hardware::CPU.arm?
      url "https://github.com/nasazzam/tide/releases/download/v0.2.0/tide-v0.2.0-aarch64-apple-darwin.tar.gz"
      sha256 "6e70c9338d78bf3fc6745a26959c8d5f75630aa04b58d0e643ca040606633ece"
    else
      url "https://github.com/nasazzam/tide/releases/download/v0.2.0/tide-v0.2.0-x86_64-apple-darwin.tar.gz"
      sha256 "a38fc3ed921677b480195006987be91abd4aae5599f615a2b3534efa4650c1cd"
    end
  end

  depends_on "tmux"

  def install
    bin.install "tide", "tide-editor"
  end

  test do
    assert_match "tide 0.2.0", shell_output("#{bin}/tide --version")
  end
end
