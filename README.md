# tray-info

Small Windows 11 tray app that shows live readings right in the notification area:

- **Bluetooth batteries** — one icon per connected device that reports its level (BLE mice and
  keyboards, classic headsets via Hands-Free): the device symbol (headphones, mouse, keyboard,
  gamepad, otherwise the Bluetooth sign, as Windows classifies the device) inside a ring that
  fills with the level. The ring turns amber at ≤ 40% and red at ≤ 20%; the exact percentage is
  in the tooltip.
- **CPU temperature**, °C — a large number with a small thermometer in the corner; white, amber
  from 75 °C, red from 90 °C.
- **CPU package power**, W — a large light-blue number with a small lightning bolt.

Symbols come from the Segoe Fluent Icons font (Windows 11); numbers use Segoe UI
Light.

Hover an icon for a tooltip with the exact reading. Right-click (or click) opens a menu with
actions only: **Start with Windows**, **Restart as administrator** when the temperature needs
it, and **Exit**.

**Start with Windows** registers a Task Scheduler logon task `tray-info` that starts this
executable with highest privileges, so the CPU temperature works right after sign-in without a
UAC prompt. Registering or removing such a task needs administrator rights, so an unelevated app
asks once through UAC. The check mark is on only when the task points at this very executable;
after moving the binary, turn it on again from the new copy to repoint the task.

Footprint: ~240 KB executable, single UI thread, about 7 MB private memory, wakes every 2 s
(CPU) and 15 s (Bluetooth).

## Download

Prebuilt `tray-info.exe` is attached to each [GitHub release](https://github.com/VladimirTyrin/tray-info/releases).

## Build and run

```powershell
cargo build --release
.\target\release\tray-info.exe
```

`install.ps1` is a local helper for the author's machine: it builds the release binary, stops
the running copy, copies the binary to `C:\Bin` and starts it again (through the autostart task
when it exists, so it comes back elevated).

Windows 11 puts new tray icons into the overflow (`^`) menu. To keep them on the taskbar:
Settings → Personalization → Taskbar → Other system tray icons → enable `tray-info` entries.

## CPU temperature needs administrator rights

There is no unprivileged CPU temperature API on Windows. On Lenovo Legion laptops the value comes
from Lenovo's WMI methods (`LENOVO_OTHER_METHOD.GetFeatureValue`, or
`LENOVO_GAMEZONE_DATA.GetCPUTemp` on older models), which only administrators may call. Without rights the
temperature icon shows `--`; use the menu item **Restart as administrator (CPU temperature)**.
Autostart (see above) starts it elevated.
Other laptops are not supported yet.
