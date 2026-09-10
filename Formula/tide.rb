class Tide < Formula
  desc "Focused terminal IDE for code, shells, and AI agents"
  homepage "https://github.com/nasazzam/tide"
  version "0.4.0"
  license "MIT"

  on_macos do
    if Hardware::CPU.arm?
      url "https://github.com/nasazzam/tide/releases/download/v0.4.0/tide-v0.4.0-aarch64-apple-darwin.tar.gz"
      sha256 "b78e157b41ecf174647457b823c591b67cfd3e89ed9cdba49651656bddb4dacf"
    else
      url "https://github.com/nasazzam/tide/releases/download/v0.4.0/tide-v0.4.0-x86_64-apple-darwin.tar.gz"
      sha256 "53963467d9b6752e6f35e024e370eb33c39628681d5c8bb9b296b6ad85230b8c"
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
