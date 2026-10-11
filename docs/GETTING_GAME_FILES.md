# Getting the game files for a local playtest

The source repository and its releases contain no MW2 or Skate 3 files. Each
player supplies their own copies during first run.

## Modern Warfare 2 (2009)

The [official Steam listing](https://store.steampowered.com/app/10180/Call_of_Duty_Modern_Warfare_2_2009/)
is for the 2009 game. Install it through Steam, including its multiplayer
component. The folder selected in IW4L must contain `iw4mp.exe` and
`zone/**/common_mp.ff`. The campaign files alone do not satisfy the launcher.

The Steam client can open the correct listing with `steam://store/10180`.
Buying or installing the game happens in Steam under the player's own account.

## Skate 3

IW4L needs the Xbox 360 game's extracted `default.xex` with its `data/` folder
beside it. The [official Xbox listing](https://www.xbox.com/en-us/games/store/Skate-3/BNKDKQXMXRR2)
shows where the game is sold, but a cloud-play entitlement alone does not
provide files to the converter. Use your own Xbox 360 disc image or an
extractable Games on Demand copy from your Xbox 360 hard drive.

The `extract-xiso` utility from [XboxDev](https://github.com/XboxDev/extract-xiso)
can extract a disc image. A verified Windows build was placed locally at
`context/tools/extract-xiso/bin/artifacts/extract-xiso.exe`; `context/` is
ignored by Git. Run it against your own ISO, then confirm the extracted folder
contains `default.xex` and `data/`. Select `default.xex` in IW4L's first-run
setup. An ISO by itself cannot be selected there.

## Sharing with friends

Friends will need their own MW2 multiplayer files, their own Skate 3 files for
skating, a controller, and the same version of this project. The public source
repository will not include game assets. Internet rooms also need an IW4L
master/relay configured for the host and joining players; see
[the upstream master guide](MASTER.md). Multiplayer skatepark features are
still in development; see [the implementation plan](SKATEPARK_EDITOR_PLAN.md).
