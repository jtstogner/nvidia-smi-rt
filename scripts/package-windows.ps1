param(
    [Parameter(Mandatory = $true)]
    [string]$Version,

    [string]$Arch = "x86_64",

    [Parameter(Mandatory = $true)]
    [string]$BinDir,

    [Parameter(Mandatory = $true)]
    [string]$OutDir
)

$ErrorActionPreference = "Stop"

$CleanVersion = $Version.TrimStart("v")
$RepoRoot = (Resolve-Path (Join-Path $PSScriptRoot "..")).Path
$AbsBinDir = (Resolve-Path $BinDir).Path
New-Item -ItemType Directory -Force -Path $OutDir | Out-Null
$AbsOutDir = (Resolve-Path $OutDir).Path

foreach ($Bin in @("nvidia-smi-rt.exe", "nvidia-smi-rt-gui.exe")) {
    $Path = Join-Path $AbsBinDir $Bin
    if (-not (Test-Path $Path)) {
        throw "Missing binary: $Path"
    }
}

$WorkDir = Join-Path ([System.IO.Path]::GetTempPath()) ("nvrt-win-" + [System.Guid]::NewGuid().ToString("N"))
New-Item -ItemType Directory -Force -Path $WorkDir | Out-Null

try {
    # 1. Portable ZIP Archive
    $StageDir = Join-Path $WorkDir "nvidia-smi-rt-v$CleanVersion-windows-$Arch"
    New-Item -ItemType Directory -Force -Path $StageDir | Out-Null
    Copy-Item (Join-Path $AbsBinDir "nvidia-smi-rt.exe") $StageDir
    Copy-Item (Join-Path $AbsBinDir "nvidia-smi-rt-gui.exe") $StageDir
    Copy-Item (Join-Path $RepoRoot "README.md") $StageDir
    Copy-Item (Join-Path $RepoRoot "LICENSE") $StageDir

    $ZipPath = Join-Path $AbsOutDir "nvidia-smi-rt-v$CleanVersion-windows-$Arch.zip"
    if (Test-Path $ZipPath) { Remove-Item -Force $ZipPath }
    Compress-Archive -Path "$StageDir\*" -DestinationPath $ZipPath -CompressionLevel Optimal
    Write-Host "Created $ZipPath"

    # 2. MSI Installer via WiX v4/v5
    if (Get-Command wix -ErrorAction SilentlyContinue) {
        $MsiPath = Join-Path $AbsOutDir "nvidia-smi-rt-v$CleanVersion-windows-$Arch.msi"
        $WxsPath = Join-Path $RepoRoot "packaging\windows\nvidia-smi-rt.wxs"
        & wix build $WxsPath `
            -arch x64 `
            -d "Version=$CleanVersion" `
            -d "BinDir=$AbsBinDir" `
            -d "RepoRoot=$RepoRoot" `
            -o $MsiPath
        if ($LASTEXITCODE -ne 0) {
            throw "wix build failed with exit code $LASTEXITCODE"
        }
        Write-Host "Created $MsiPath"
    } else {
        Write-Warning "Skipping .msi build: 'wix' CLI not found"
    }
}
finally {
    Remove-Item -Recurse -Force -ErrorAction SilentlyContinue $WorkDir
}
