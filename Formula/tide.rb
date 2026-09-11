class Tide < Formula
  desc "Focused terminal IDE for code, shells, and AI agents"
  homepage "https://github.com/nasazzam/tide"
  version "0.5.2"
  license "MIT"

  on_macos do
    if Hardware::CPU.arm?
      url "https://github.com/nasazzam/tide/releases/download/v0.5.2/tide-v0.5.2-aarch64-apple-darwin.tar.gz"
      sha256 "5f3881f4493c32db7a2ef09cf128a13d63798f31cc06be6453a7d2e5c5858ff3"
    else
      url "https://github.com/nasazzam/tide/releases/download/v0.5.2/tide-v0.5.2-x86_64-apple-darwin.tar.gz"
      sha256 "ef03272e4be61ca4397ccd9e3f6b110b166c0fc4a6c7824f3d6b389146477435"
    end
  end

  depends_on "tmux"

  def install
    bin.install "tide", "tide-editor"
  end

  test do
    assert_match "tide 0.5.2", shell_output("#{bin}/tide --version")
  end
end
