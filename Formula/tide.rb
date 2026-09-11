class Tide < Formula
  desc "Focused terminal IDE for code, shells, and AI agents"
  homepage "https://github.com/nasazzam/tide"
  version "0.4.4"
  license "MIT"

  on_macos do
    if Hardware::CPU.arm?
      url "https://github.com/nasazzam/tide/releases/download/v0.4.4/tide-v0.4.4-aarch64-apple-darwin.tar.gz"
      sha256 "123d7b5187c96d0fc36fea0dc8ef60688629258d97502660b0bca43a015184bf"
    else
      url "https://github.com/nasazzam/tide/releases/download/v0.4.4/tide-v0.4.4-x86_64-apple-darwin.tar.gz"
      sha256 "4801821471efff382e2807e46de227053275abbfe876fa375789b50d73c934df"
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
