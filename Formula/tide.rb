class Tide < Formula
  desc "Focused terminal IDE for code, shells, and AI agents"
  homepage "https://github.com/nasazzam/tide"
  version "0.2.0"
  license "MIT"

  on_macos do
    if Hardware::CPU.arm?
      url "https://github.com/nasazzam/tide/releases/download/v0.2.0/tide-v0.2.0-aarch64-apple-darwin.tar.gz"
      sha256 "1087e4f88a54d321a50599eafe7c0b425160e98a8596835072a9a1605bfd5571"
    else
      url "https://github.com/nasazzam/tide/releases/download/v0.2.0/tide-v0.2.0-x86_64-apple-darwin.tar.gz"
      sha256 "cd76d0df9c66a6dec075c353203a8972081bef0594b3bf729bbba744b161b87e"
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
