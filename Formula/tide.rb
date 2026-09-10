class Tide < Formula
  desc "Focused terminal IDE for code, shells, and AI agents"
  homepage "https://github.com/nasazzam/tide"
  version "0.2.0"
  license "MIT"

  on_macos do
    if Hardware::CPU.arm?
      url "https://github.com/nasazzam/tide/releases/download/v0.2.0/tide-v0.2.0-aarch64-apple-darwin.tar.gz"
      sha256 "2e0edb55d7fba3aa20c71bde0a048d3f81ab8fedc1525451fc6d6ecfe50cdbd1"
    else
      url "https://github.com/nasazzam/tide/releases/download/v0.2.0/tide-v0.2.0-x86_64-apple-darwin.tar.gz"
      sha256 "ec731d4cab64ae48b2a005d1346996bccd84758f06c1c676df19b497f385258c"
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
