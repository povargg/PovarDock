param(
    [Parameter(Mandatory)]
    [string]$Executable,

    [Parameter(Mandatory)]
    [string]$OutputDirectory
)

$ErrorActionPreference = 'Stop'
$binary = Get-Item -LiteralPath $Executable
if ($binary.PSIsContainer -or $binary.Extension -ne '.exe' -or $binary.Length -eq 0) {
    throw 'Supply the compiled WinDock executable.'
}

$directory = New-Item -ItemType Directory -Path $OutputDirectory -Force
$name = 'windock-x64.exe'
$destination = Join-Path $directory.FullName $name
if ($binary.FullName -ne $destination) {
    Copy-Item -LiteralPath $binary.FullName -Destination $destination
}

$hash = (Get-FileHash -LiteralPath $destination -Algorithm SHA256).Hash.ToLowerInvariant()
Set-Content -LiteralPath (Join-Path $directory.FullName 'SHA256SUMS.txt') -Encoding ascii -Value "$hash  $name"
