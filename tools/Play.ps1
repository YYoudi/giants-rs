param(
    [Parameter(Mandatory=$true)][string]$DataRoot,
    [string]$UserDataRoot,
    [ValidateSet('vulkan','dx12')][string]$Backend,
    [switch]$CheckOnly
)
$ErrorActionPreference = 'Stop'
$executable = Join-Path $PSScriptRoot 'giants-app.exe'
if (-not (Test-Path -LiteralPath $executable -PathType Leaf)) {
    throw 'giants-app.exe is missing. Extract the entire preview ZIP first.'
}
if (-not (Test-Path -LiteralPath $DataRoot -PathType Container)) {
    throw 'DataRoot must point to your own Giants installation directory.'
}
$resolvedData = (Resolve-Path -LiteralPath $DataRoot).Path
$archiveDirectory = Join-Path $resolvedData 'Bin'
if (-not (Test-Path -LiteralPath $archiveDirectory -PathType Container) -or
    -not (Get-ChildItem -LiteralPath $archiveDirectory -File -Filter '*.gzp' | Select-Object -First 1)) {
    throw 'No Bin/*.gzp archives found. Select the installation above Bin, not Bin itself or a save folder.'
}
if (-not $UserDataRoot) {
    $UserDataRoot = Join-Path ([Environment]::GetFolderPath('LocalApplicationData')) 'Giants-Recomp\preview'
}
$resolvedUser = [System.IO.Path]::GetFullPath($UserDataRoot)
$dataPrefix = $resolvedData.TrimEnd('\','/') + [System.IO.Path]::DirectorySeparatorChar
if ($resolvedUser.Equals($resolvedData,[System.StringComparison]::OrdinalIgnoreCase) -or
    $resolvedUser.StartsWith($dataPrefix,[System.StringComparison]::OrdinalIgnoreCase)) {
    throw 'UserDataRoot must be outside the original installation.'
}
$launchArguments = @('--data-root',$resolvedData,'--scenario','world','--mecc-dev','--mecc-menu','--user-data-root',$resolvedUser)
if ($CheckOnly) {
    # Validates paths and argument construction without running the game.
    $launchArguments | ConvertTo-Json
    exit 0
}
New-Item -ItemType Directory -Path $resolvedUser -Force | Out-Null
$previousBackend = [Environment]::GetEnvironmentVariable('WGPU_BACKEND','Process')
try {
    if ($Backend) { [Environment]::SetEnvironmentVariable('WGPU_BACKEND',$Backend,'Process') }
    & $executable @launchArguments
    if ($LASTEXITCODE -ne 0) { throw "Preview exited with code $LASTEXITCODE. Keep the console output when reporting a problem." }
} finally {
    [Environment]::SetEnvironmentVariable('WGPU_BACKEND',$previousBackend,'Process')
}
