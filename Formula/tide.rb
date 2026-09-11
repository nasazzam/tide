class Tide < Formula
  desc "Focused terminal IDE for code, shells, and AI agents"
  homepage "https://github.com/nasazzam/tide"
  version "0.5.1"
  license "MIT"

  on_macos do
    if Hardware::CPU.arm?
      url "https://github.com/nasazzam/tide/releases/download/v0.5.1/tide-v0.5.1-aarch64-apple-darwin.tar.gz"
      sha256 "4d2c5f9ec19022d401672b55be052379ea37e83236511fbf7e94bd54eaf5ad2b"
    else
      url "https://github.com/nasazzam/tide/releases/download/v0.5.1/tide-v0.5.1-x86_64-apple-darwin.tar.gz"
      sha256 "236901381b82096d82c7af6d125de9ad291c3f23bee9d396368c6087f0ae48fd"
    end
  end

  depends_on "tmux"

  def install
    bin.install "tide", "tide-editor"
  end

  test do
    assert_match "tide 0.5.1", shell_output("#{bin}/tide --version")
  end
end
