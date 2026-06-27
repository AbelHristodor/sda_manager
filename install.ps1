# Hymn Finder installer for Windows.
#
#   irm https://raw.githubusercontent.com/AbelHristodor/sda_manager/main/install.ps1 | iex
#
# Downloads the latest released hymnal-gui.exe and installs it to
# %LOCALAPPDATA%\Programs\hymnal-gui, adding that directory to the user PATH.

$ErrorActionPreference = "Stop"

$repo   = "AbelHristodor/sda_manager"
$target = "x86_64-pc-windows-msvc"
$asset  = "hymnal-gui-$target.zip"
$url    = "https://github.com/$repo/releases/latest/download/$asset"

$installDir = Join-Path $env:LOCALAPPDATA "Programs\hymnal-gui"
New-Item -ItemType Directory -Force -Path $installDir | Out-Null

$tmp = New-Item -ItemType Directory -Force -Path (Join-Path $env:TEMP "hymnal-gui-install")
$zip = Join-Path $tmp $asset

Write-Host "Downloading $asset ..."
try {
    Invoke-WebRequest -Uri $url -OutFile $zip -UseBasicParsing
} catch {
    Write-Error "Download failed from $url. Make sure a release has been published for $repo."
    exit 1
}

Expand-Archive -Path $zip -DestinationPath $tmp -Force
Copy-Item (Join-Path $tmp "hymnal-gui-$target\hymnal-gui.exe") (Join-Path $installDir "hymnal-gui.exe") -Force
Remove-Item -Recurse -Force $tmp

Write-Host "Installed hymnal-gui.exe to $installDir"

# Add to the user PATH if it isn't already there.
$userPath = [Environment]::GetEnvironmentVariable("Path", "User")
if ($userPath -notlike "*$installDir*") {
    $newPath = if ($userPath) { "$userPath;$installDir" } else { $installDir }
    [Environment]::SetEnvironmentVariable("Path", $newPath, "User")
    Write-Host "Added $installDir to your user PATH. Restart your terminal to pick it up."
}
