class Tide < Formula
  desc "Focused terminal IDE for code, shells, and AI agents"
  homepage "https://github.com/nasazzam/tide"
  version "0.5.3"
  license "MIT"

  on_macos do
    if Hardware::CPU.arm?
      url "https://github.com/nasazzam/tide/releases/download/v0.5.3/tide-v0.5.3-aarch64-apple-darwin.tar.gz"
      sha256 "8e4e77df2d599bfebaa6e665db588d1902a9f3f803af9b3685e282df74f64c63"
    else
      url "https://github.com/nasazzam/tide/releases/download/v0.5.3/tide-v0.5.3-x86_64-apple-darwin.tar.gz"
      sha256 "df4230393d2c0e4823755ec6fba3ef9b60be9b8753682a9c4dcfca39d0df6cda"
    end
  end

  depends_on "tmux"

  def install
    bin.install "tide", "tide-editor"
  end

  test do
    assert_match "tide 0.5.3", shell_output("#{bin}/tide --version")
  end
end
