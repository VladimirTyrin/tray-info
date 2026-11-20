# Builds the release binary and installs it to C:\Bin, this machine's tools folder; autostart is
# then turned on from that copy's tray menu. Not a general installer.
$ErrorActionPreference = 'Stop'
$installDir = 'C:\Bin'
$installed = Join-Path $installDir 'tray-info.exe'

Push-Location $PSScriptRoot
try {
    cargo build --release
    if ($LASTEXITCODE -ne 0) { throw 'cargo build --release failed' }

    $task = Get-ScheduledTask -TaskName tray-info -ErrorAction SilentlyContinue
    # A copy started by the elevated autostart task cannot be killed from an unelevated shell,
    # but the task owner may stop the task.
    if ($task) {
        Stop-ScheduledTask -TaskName tray-info
        # Stopping returns before the process is gone, and an elevated process cannot be waited
        # on from here, so look for it by name until it disappears.
        for ($i = 0; $i -lt 50 -and (Get-Process tray-info -ErrorAction SilentlyContinue); $i++) {
            Start-Sleep -Milliseconds 100
        }
    }
    $running = Get-Process tray-info -ErrorAction SilentlyContinue
    if ($running) {
        try {
            $running | Stop-Process -Force -ErrorAction Stop
        } catch {
            throw 'tray-info runs as administrator and cannot be stopped from here: choose Exit in its tray menu, then run this script again.'
        }
        # The executable stays locked until the process is gone.
        $running | Wait-Process
    }

    New-Item -ItemType Directory -Force $installDir | Out-Null
    Copy-Item target\release\tray-info.exe $installed -Force

    # The autostart task starts it elevated without a UAC prompt; without the task, start it
    # through Explorer so it does not end with this shell.
    if ($task) {
        Start-ScheduledTask -TaskName tray-info
    } else {
        Start-Process explorer.exe -ArgumentList "`"$installed`""
    }
    Write-Host "Installed $installed"
} finally {
    Pop-Location
}
