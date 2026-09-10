class Tide < Formula
  desc "Focused terminal IDE for code, shells, and AI agents"
  homepage "https://github.com/nasazzam/tide"
  version "0.2.0"
  license "MIT"

  on_macos do
    if Hardware::CPU.arm?
      url "https://github.com/nasazzam/tide/releases/download/v0.2.0/tide-v0.2.0-aarch64-apple-darwin.tar.gz"
      sha256 "50301cf9730b8b815620451bcbbb90f523b06d7ac300eec992deaa5b45fa05e8"
    else
      url "https://github.com/nasazzam/tide/releases/download/v0.2.0/tide-v0.2.0-x86_64-apple-darwin.tar.gz"
      sha256 "19c06cef17e0530e365e8a123d0551a74d40b717949af5ee441a98def46e8403"
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
