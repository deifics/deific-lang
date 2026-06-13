# Registers .df files to run with `deific run` on double-click or from Explorer.
# Run once from the repo root after building: .\register_df.ps1

$exe = (Resolve-Path ".\target\release\deific.exe").Path

# File type registration (per-user, no elevation needed)
New-Item -Path "HKCU:\SOFTWARE\Classes\.df"                                   -Force | Out-Null
Set-ItemProperty -Path "HKCU:\SOFTWARE\Classes\.df" -Name "(Default)" -Value "deific.file"

New-Item -Path "HKCU:\SOFTWARE\Classes\deific.file"                           -Force | Out-Null
Set-ItemProperty -Path "HKCU:\SOFTWARE\Classes\deific.file" -Name "(Default)" -Value "Deific Script"

New-Item -Path "HKCU:\SOFTWARE\Classes\deific.file\shell\open\command"        -Force | Out-Null
# cmd /k keeps the window open after the program exits (like .py on Windows with pause)
Set-ItemProperty -Path "HKCU:\SOFTWARE\Classes\deific.file\shell\open\command" `
    -Name "(Default)" -Value "cmd /k `"`"$exe`" run `"%1`"`""

# Tell Explorer to pick up the new association immediately
if (Get-Command "ie4uinit.exe" -ErrorAction SilentlyContinue) {
    ie4uinit.exe -show
}

Write-Host "Registered: .df -> `"$exe`" run <file>"
Write-Host "You can now double-click .df files or run them from Explorer."
