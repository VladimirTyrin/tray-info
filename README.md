# tray-info

A tiny Windows 11 tray app that keeps an eye on what you otherwise have to dig for: how much
charge your Bluetooth mouse, keyboard and headphones have left, and how hot and power-hungry the
CPU is right now. Everything sits in the notification area next to the clock, updates by itself
and never needs a window.

![tray-info icons on the taskbar](docs/tray.png)

![The same icons enlarged: headphones, mouse and keyboard batteries, CPU power and temperature](docs/tray-zoomed.png)

## What it shows

- **Bluetooth batteries** — one icon per connected device that reports its level: BLE mice and
  keyboards and classic headsets alike. The symbol tells which device it is (headphones, mouse,
  keyboard, gamepad, otherwise the Bluetooth sign), the ring around it fills with the charge and
  turns amber at 40% and red at 20%, so a dying battery is noticed before the device goes quiet.
  Icons appear and disappear as devices connect and disconnect. Reading the level never opens a
  connection to the device, so watching it costs the device no battery.
- **CPU temperature** in °C — white normally, amber from 75 °C, red from 90 °C.
- **CPU package power** in W — the whole processor's draw, handy to see what a background task
  or a game costs.

Hovering an icon shows the exact value and the device name. The icons follow the taskbar's light
or dark theme and stay sharp at any display scaling.

The app can start with Windows, and then it starts with administrator rights without a UAC
prompt (CPU temperature needs them, see below).

It is meant to be invisible in every other sense too: ~240 KB executable, one thread, about 6 MB
of memory, and it wakes up only every 2 s for the CPU and every 15 s for Bluetooth.

## Download

Prebuilt `tray-info.exe` is attached to each
[GitHub release](https://github.com/VladimirTyrin/tray-info/releases). No installer: put the
file anywhere and run it.

Windows 11 puts new tray icons into the overflow (`^`) menu. To keep them on the taskbar:
Settings → Personalization → Taskbar → Other system tray icons → enable the `tray-info` entries.

## Requirements and limits

- Windows 11. Windows 10 (1703+) runs it too, but without the symbols: their font, Segoe
  Fluent Icons, ships with Windows 11 only.
- CPU power needs the Windows Energy Meter driver, present on most Intel machines.
- CPU temperature: Windows has no unprivileged generic API for it. The app reads it from
  Lenovo's firmware WMI methods (`LENOVO_OTHER_METHOD.GetFeatureValue`, or
  `LENOVO_GAMEZONE_DATA.GetCPUTemp` on older models), so it works on Lenovo Legion laptops and
  only with administrator rights. Without them the icon shows `--` and the menu offers a restart
  as administrator; autostart runs it elevated. On other machines the temperature icon is
  hidden.

## Build

```powershell
cargo build --release
.\target\release\tray-info.exe
```

`install.ps1` is a local helper for the author's machine: it builds the release binary, stops
the running copy, copies the binary to `C:\Bin` and starts it again (through the autostart task
when it exists, so it comes back elevated).

Autostart is a Task Scheduler logon task named `tray-info` with "run with highest privileges".
Creating or removing it needs administrator rights, so an unelevated app asks once through UAC.
