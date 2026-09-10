class Tide < Formula
  desc "Focused terminal IDE for code, shells, and AI agents"
  homepage "https://github.com/nasazzam/tide"
  version "0.2.0"
  license "MIT"

  on_macos do
    if Hardware::CPU.arm?
      url "https://github.com/nasazzam/tide/releases/download/v0.2.0/tide-v0.2.0-aarch64-apple-darwin.tar.gz"
      sha256 "702e163cea1ad3764d74cf231a031ce44bff84b67d32d401cff7fff3d77547d8"
    else
      url "https://github.com/nasazzam/tide/releases/download/v0.2.0/tide-v0.2.0-x86_64-apple-darwin.tar.gz"
      sha256 "ed7011bbd9c7d17c13216f18fcc17a52f6bf2199a535561fc16f6fdad7cc5baa"
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
