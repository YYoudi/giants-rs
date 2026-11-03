param(
    [Parameter(Mandatory=$true)][string]$OutputDirectory,
    [string]$TargetDirectory,
    [string]$Toolchain = 'stable-x86_64-pc-windows-gnu'
)
$ErrorActionPreference = 'Stop'
. (Join-Path $PSScriptRoot 'dev-env.ps1')
$workspaceRoot = Split-Path -Parent $PSScriptRoot
Push-Location $workspaceRoot
try {
    if (Test-Path -LiteralPath $OutputDirectory) { throw 'Refusing to overwrite a preview build.' }
    git diff --quiet HEAD -- crates Cargo.toml Cargo.lock rust-toolchain.toml
    if ($LASTEXITCODE -ne 0) { throw 'Commit runtime source changes before recording a build receipt.' }
    $sourceCommit = (git rev-parse HEAD).Trim()
    if (-not $TargetDirectory) { $TargetDirectory = Join-Path $workspaceRoot 'target' }
    $TargetDirectory = [System.IO.Path]::GetFullPath($TargetDirectory)
    cargo "+$Toolchain" build --locked --release -p giants-app --target x86_64-pc-windows-gnu --target-dir $TargetDirectory
    if ($LASTEXITCODE -ne 0) { throw 'Preview build failed.' }
    $builtExecutable = Join-Path $TargetDirectory 'x86_64-pc-windows-gnu/release/giants-app.exe'
    $metadata = cargo "+$Toolchain" metadata --locked --filter-platform x86_64-pc-windows-gnu --format-version 1
    if ($LASTEXITCODE -ne 0) { throw 'Dependency metadata failed.' }
    $rustVersion = (rustc "+$Toolchain" --version).Trim()
    if ($LASTEXITCODE -ne 0) { throw 'Cannot record compiler identity.' }
    $linkerVersion = (& gcc --version | Select-Object -First 1)
    if ($LASTEXITCODE -ne 0) { throw 'MinGW GCC is required on PATH (or GIANTS_MINGW_BIN).' }
    New-Item -ItemType Directory -Path $OutputDirectory | Out-Null
    $copiedExecutable = Join-Path $OutputDirectory 'giants-app.exe'
    Copy-Item -LiteralPath $builtExecutable -Destination $copiedExecutable
    $receipt = @{
        source_commit = $sourceCommit
        executable_sha256 = (Get-FileHash -LiteralPath $copiedExecutable -Algorithm SHA256).Hash.ToLowerInvariant()
        target = 'x86_64-pc-windows-gnu'
        toolchain = "$Toolchain / $rustVersion"
        linker = $linkerVersion
    }
    $utf8NoBom = New-Object System.Text.UTF8Encoding($false)
    [System.IO.File]::WriteAllText((Join-Path $OutputDirectory 'build-receipt.json'), ($receipt | ConvertTo-Json), $utf8NoBom)
    [System.IO.File]::WriteAllText((Join-Path $OutputDirectory 'cargo-metadata.json'), ($metadata -join "`n"), $utf8NoBom)
    Write-Output "Preview build and receipt: $OutputDirectory"
} finally { Pop-Location }
