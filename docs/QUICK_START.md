# Windows preview: quick start

This is an incomplete, unsigned Windows x64 development preview. Approximately
99% of the current project content is AI-generated according to the owner's
estimate, and has not received comprehensive human review. Automated checks
do not establish complete playable missions or fidelity.

## Download and start

1. Obtain your own legal PC installation of Giants: Citizen Kabuto.
2. Download a published Windows x64 ZIP from
   [Releases](https://github.com/YYoudi/giants-rs/releases).
   Drafts are not public downloads. Read that preview's limitations first.
3. Extract the ZIP and keep its bundled notices. Move **giants-rs.exe** beside
   the original **Giants.exe**, in the game folder containing `Bin`. The different
   name preserves the original executable.
4. Double-click **giants-rs.exe**. It detects the adjacent installation and opens
   the integrated menu. No terminal, installer or launch arguments are needed.
5. Alternatively, leave the preview in its extracted folder and run it there.
   A folder picker asks for the installation and remembers that selection.

Choose the installation directory containing `Bin` and its `.gzp` archives. Do not select `Bin` itself, the
project source tree, a save directory or the directory above the installation.
The launcher does not copy original data into the download folder. User state is
stored separately under `%LOCALAPPDATA%\Giants-Recomp\preview` by default.

The independent **Preview All** profile is selected by default, with the mission
catalog unlocked. This is preview progress, not an imported original save,
restored inventory or completed campaign. Other profiles and original saves are
preserved. If all five slots are occupied and Preview All is absent, free a slot
in Player IDs to create it; the program never replaces an existing profile.

Open **Preview tools** in the upper-right corner for **Browse all missions**,
**Mecc test arena** and **Game installation...**. Changing the installation uses
the folder picker and restarts the preview. Escape in the test arena returns to
the menu. These actions do not require debug commands.

Installation selection and the startup log (`installation.txt` and `Giants.log`)
are stored beside the independent profiles in the user-data folder. A missing or
stale selection opens the picker when no adjacent installation is found.
Cancelling exits without loading. Startup errors appear in a dialog. `Play.ps1` is optional and also
supports graphical startup without arguments.

`Play.ps1 -DataRoot 'D:\Games\Giants' -CheckOnly` checks the selected directory
and prints the launch arguments without opening a game window.

## What to expect

The entry point is the integrated menu. Select a mission there. Openings may be
skippable; resident Mecc controls are available only where the build can admit
the required player state. Other entries may remain inspection views. Mission
completion, active enemies, full combat, squad behavior, Delphi, Kabuto, bases,
multiplayer and saved campaign continuity are incomplete. CPU-validated services
may not yet be connected to this menu-to-mission path.

For the current Mecc path, select **Start New Single Player Game**, wait for the
mission to load and use **Escape** to skip the opening. The default Mecc bindings
are **WASD** for movement, mouse motion for aiming, **right mouse held** for thrust,
**Space** for nitro where supported, **left mouse** for the diagnostic pistol and
**R** for the view toggle. **Escape** returns to the menu. Check the profile's
Mecc control options if bindings were changed. The normal controller can present
the source jet models, acquired descent animation, base-pistol projectile and
health/fuel gauges in snapshots that contain those integrations. No replay or
experimental-backend flag is needed to activate supported normal controls.

Read the release notes accompanying your ZIP for its exact runtime source,
included features and actually completed checks. Current main, later captures
and earlier preview acceptance do not update that executable. Brief OS mouse
pulses and supplied-command GPU diagnostics do not establish sustained held-input
flight, nitro, descent and landing as one continuous interaction. The descent
rule requires sufficient vertical speed and floor clearance; releasing thrust
alone does not select it. Combat, impact and native world ownership remain partial.
Historical captures retain their recorded build and do not expand this scope.

Movement uses physical WASD, Space and Ctrl where supported; right mouse controls
the view in inspection modes. Follow the on-screen hints for the active mode.
Do not treat development shortcuts as original-game controls. Report the release
tag, selected mission, GPU and exact steps when a control fails.

## Requirements and troubleshooting

- Windows x64 and a graphics driver supporting the runtime's Vulkan or Direct3D 12
  backend are required. A minimum supported Windows version and GPU are not yet
  established by a hardware compatibility matrix.
- For rendering initialization failures try `-Backend dx12` or `-Backend vulkan`.
- The primary reference is the debug-enabled PC build dated December 18, 2000.
  Earlier comparisons used version 1.449. Broad compatibility across retail/GOG
  editions is not yet certified. A recognized archive directory is not proof
  that every required asset exists or that the edition is supported.
- Keep the startup error and Giants.log locally. Include the preview tag and
  GPU model in a bug report; do not attach original assets, archives, memory
  dumps, personal paths or private save data.
- If Windows blocks the unsigned program, inspect the download source and the
  published SHA-256 checksum. The project does not provide a signing certificate.

## Build without Visual Studio

Install Rust's GNU Windows toolchain and MinGW-w64 on `PATH`, then run from the
repository root:

```powershell
cargo +stable-x86_64-pc-windows-gnu build --locked --release -p giants-app --target x86_64-pc-windows-gnu
.\target\x86_64-pc-windows-gnu\release\giants-app.exe --data-root 'D:\Games\Giants' --scenario world --mecc-dev --mecc-menu --user-data-root "$env:LOCALAPPDATA\Giants-Recomp\preview"
```

Use the recorded toolchain for a specific preview. No game assets are downloaded
by the build. Python is needed for project checks and packaging, not to run the
extracted preview.
