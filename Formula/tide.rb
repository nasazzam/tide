class Tide < Formula
  desc "Focused terminal IDE for code, shells, and AI agents"
  homepage "https://github.com/nasazzam/tide"
  version "0.4.1"
  license "MIT"

  on_macos do
    if Hardware::CPU.arm?
      url "https://github.com/nasazzam/tide/releases/download/v0.4.1/tide-v0.4.1-aarch64-apple-darwin.tar.gz"
      sha256 "34537b2ba9b03e523a6289b4ed705688a172752dec0d6ce846efa5bae70f16c8"
    else
      url "https://github.com/nasazzam/tide/releases/download/v0.4.1/tide-v0.4.1-x86_64-apple-darwin.tar.gz"
      sha256 "e3af0847675cd071943628b9ab345c149b0775383454a0a9492ab640096e8985"
    end
  end

  depends_on "tmux"

  def install
    bin.install "tide", "tide-editor"
  end

  test do
    assert_match "tide 0.4.1", shell_output("#{bin}/tide --version")
  end
end
