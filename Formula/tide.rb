class Tide < Formula
  desc "Focused terminal IDE for code, shells, and AI agents"
  homepage "https://github.com/nasazzam/tide"
  version "0.4.3"
  license "MIT"

  on_macos do
    if Hardware::CPU.arm?
      url "https://github.com/nasazzam/tide/releases/download/v0.4.3/tide-v0.4.3-aarch64-apple-darwin.tar.gz"
      sha256 "7883db7f9fd31c86bdf0349887a92b046be3421f3248a5d48408cf581e887a4d"
    else
      url "https://github.com/nasazzam/tide/releases/download/v0.4.3/tide-v0.4.3-x86_64-apple-darwin.tar.gz"
      sha256 "9a199f8f04ae179eb320843addfbecac454fc35e5a449a1ef6f369f056310f74"
    end
  end

  depends_on "tmux"

  def install
    bin.install "tide", "tide-editor"
  end

  test do
    assert_match "tide 0.4.3", shell_output("#{bin}/tide --version")
  end
end
