cask "sotto" do
  version "0.2.0"
  sha256 "115603612709b4dd7c743bcae935bf2506800fe475eab97b149fa57071153693"

  url "https://github.com/jokeuncle/sotto/releases/download/v#{version}/Sotto_#{version}_aarch64.dmg"
  name "Sotto"
  desc "Quiet desktop aphorism companion"
  homepage "https://github.com/jokeuncle/sotto"

  depends_on arch: :arm64
  depends_on macos: :big_sur

  app "Sotto.app"

  postflight do
    system_command "/usr/bin/xattr",
                   args:         ["-dr", "com.apple.quarantine", "#{appdir}/Sotto.app"],
                   must_succeed: false,
                   print_stderr: false
  end

  zap trash: "~/Library/Application Support/app.sotto.daily"

  caveats <<~EOS
    First launch note:
    The cask automatically clears quarantine after install. If macOS still says
    "damaged", "can't be opened", or blocks the first launch, run:

      xattr -dr com.apple.quarantine /Applications/Sotto.app
      open /Applications/Sotto.app

    Only run this for a Sotto release you trust.
  EOS
end
