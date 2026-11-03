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
3. Extract the complete ZIP into a writable folder outside the game installation.
4. Open PowerShell in the extracted folder:

   ```powershell
   .\Play.ps1 -DataRoot 'D:\Games\Giants'
   ```

Choose the installation directory containing `Bin` and its `.gzp` archives. Do not select `Bin` itself, the
project source tree, a save directory or the directory above the installation.
The launcher does not copy original data into the download folder. User state is
stored separately under `%LOCALAPPDATA%\Giants-Recomp\preview` by default.

If local PowerShell policy prevents the script from running, use the executable
directly; no policy change is required:

```powershell
.\giants-app.exe --data-root 'D:\Games\Giants' --scenario world --mecc-dev --mecc-menu --user-data-root "$env:LOCALAPPDATA\Giants-Recomp\preview"
```

`Play.ps1 -DataRoot 'D:\Games\Giants' -CheckOnly` checks the selected directory
and prints the launch arguments without opening a game window.

## What to expect

The entry point is the integrated menu. Select a mission there. Openings may be
skippable; resident Mecc controls are available only where the build can admit
the required player state. Other entries may remain inspection views. Mission
completion, active enemies, full combat, squad behavior, Delphi, Kabuto, bases,
multiplayer and saved campaign continuity are incomplete. CPU-validated services
may not yet be connected to this menu-to-mission path.

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
- Keep the console open to retain an error message. Include the preview tag and
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
