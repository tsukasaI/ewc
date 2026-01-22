class Ewc < Formula
  desc "Enhanced Word Count - A modern alternative to wc"
  homepage "https://github.com/tsukasaI/ewc"
  url "https://github.com/tsukasaI/ewc/archive/refs/tags/v0.1.0.tar.gz"
  sha256 "PLACEHOLDER_SHA256"
  license "MIT"

  depends_on "rust" => :build

  def install
    system "cargo", "install", *std_cargo_args
  end

  test do
    (testpath/"test.txt").write("hello world\n")
    output = shell_output("#{bin}/ewc #{testpath}/test.txt")
    assert_match(/1\s+2\s+12/, output)
  end
end
