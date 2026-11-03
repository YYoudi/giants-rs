# Optional shared cache root; otherwise respect the caller's normal toolchain/cache.
$workspaceTemporary = Join-Path (Split-Path -Parent $PSScriptRoot) '.local/tmp'
New-Item -ItemType Directory -Path $workspaceTemporary -Force | Out-Null
$env:TEMP = $workspaceTemporary
$env:TMP = $workspaceTemporary
$env:PYTHONPYCACHEPREFIX = Join-Path $workspaceTemporary 'pycache'
if ($env:GIANTS_CACHE_ROOT) {
    foreach ($entry in @{CARGO_HOME='cargo';RUSTUP_HOME='rustup'}.GetEnumerator()) {
        [Environment]::SetEnvironmentVariable($entry.Key,(Join-Path $env:GIANTS_CACHE_ROOT $entry.Value),'Process')
    }
}
if ($env:GIANTS_MINGW_BIN) { $env:Path = $env:GIANTS_MINGW_BIN + [IO.Path]::PathSeparator + $env:Path }
