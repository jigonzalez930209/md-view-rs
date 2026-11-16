$ErrorActionPreference = "Stop"
$Root = Resolve-Path "$PSScriptRoot\..\.."
Set-Location $Root
cargo build --release --locked -p mdview-app
wix build packaging\windows\mdview.wxs -o dist\MD-View-1.0.0-rc.0.msi
Get-FileHash dist\MD-View-1.0.0-rc.0.msi -Algorithm SHA256 | Out-File dist\MD-View-1.0.0-rc.0.msi.sha256
