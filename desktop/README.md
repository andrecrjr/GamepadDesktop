# Gamepad Desktop user guide

Use a gamepad to move the pointer, click, scroll, and type in your active app. For a quick introduction, see the [main README](../README.md).

## Installation

Download the files from the [latest release](https://github.com/andrecrjr/GamepadDesktop/releases/latest).

### Windows

1. Download `GamepadDesktop-Windows-x64.exe`.
2. Connect your controller and open the executable.
3. Check the device in the panel and follow the welcome tour.

The tray menu lets you open or hide the panel and quit the app. On first launch, the app creates a per-user Start menu shortcut to identify its notifications.

### Linux

On Debian, Ubuntu, and derivatives, x64, install `GamepadDesktop-Linux-x64.deb` through your software center or terminal:

```sh
sudo apt install ./GamepadDesktop-Linux-x64.deb
```

On other x64 distributions, extract `GamepadDesktop-Linux-x64.tar.gz` and run:

```sh
tar -xzf GamepadDesktop-Linux-x64.tar.gz
chmod +x gamepad-desktop
./gamepad-desktop
```

The app needs access to your controller and `/dev/uinput` to create virtual mouse and keyboard devices. It does not change these permissions automatically. If you get a permission error, check your distribution's device access rules before using the app; opening the panel alone does not confirm that input is working.

To run from source, see [development](docs/development.md).

## First launch

The welcome tour has five stages: **mouse**, **scrolling**, **keyboard**, **modes**, and **getting started**. Each stage may contain short practice steps, such as opening, typing with, and closing the keyboard.

- Try the action or choose **Watch a demo** beside the practice area.
- Success feedback appears above the example, and **Continue** becomes highlighted.
- Practice is optional. You can advance or skip the tour.
- **LB/L1** goes back and **RB/R1** continues in the main lessons. Mouse and Tab/Enter also work.
- Choose **Start in Desktop** or **Start in Game** to finish.

**Learn more** covers right-clicking, dragging, the window switcher, screenshots/fullscreen, and pausing. It is available at the end of the tour and in the panel footer. In the window-switching lesson, R1 cycles through windows and A/✕ continues the tour; the hints show this difference.

The tour is a local simulation: its commands do not reach your other apps. The practice keyboard uses the app's renderer; the keyboard you use outside the tour may be provided by Windows or COSMIC.

Choose **Review welcome tour** to repeat the main path. Completion, language, and settings are saved.

## Controls

The symbols below use Xbox / PlayStation names. On Switch, follow the indicated button position and the labels shown in the panel.

| Controller input             | Action in Desktop mode                        |
| ---------------------------- | --------------------------------------------- |
| Left stick                   | Move the pointer                              |
| Right stick, up/down         | Scroll the page                               |
| A / ✕ — bottom               | Left-click; hold to drag                      |
| B / ○ — right                | Right-click                                   |
| X / □ — left                 | Enter; confirm in the window switcher         |
| Y / △ — top                  | Escape; cancel in the window switcher         |
| L3                           | Open the system launcher — Super / Windows    |
| LB / L1                      | Tab — next field                              |
| RB / R1                      | Open Alt+Tab; repeat to cycle through windows |
| Start/Menu + LB/L1           | Print Screen — start system capture           |
| Start/Menu + RB/R1           | F11 — toggle fullscreen                       |
| R3                           | Open or close the keyboard                    |
| Select / View, double-tap    | Switch between Desktop and Game               |
| Start + Select / Menu + View | Pause or resume Desktop controls              |

In **Game** mode, use **Select / View + R3** to open the keyboard. While it is open, **R3** closes it. Release the buttons and center the sticks to resume controls after closing the keyboard, reconnecting, or resuming from pause.

The D-pad sends arrow keys to the active app on Linux. On Windows, it navigates the open keyboard and does not send regular arrow keys during desktop control.

## On-screen keyboard

### Windows

The app requests the Windows touch keyboard and attempts to use its Gamepad layout. When that layout is accepted, Windows handles controller navigation. With the regular layout fallback, the D-pad moves the pointer between keys, the left stick moves the pointer smoothly, and A selects.

Language and layout follow Windows settings. Automatic openings by the system are observed, but do not by themselves enable the app's native navigation session. See the [Windows navigation details](docs/windows-gamepad-navigation.md).

### COSMIC/Wayland

When the `cosmic-osk` service is available, the app uses the COSMIC keyboard. The desktop handles selection and typing; R3 or B closes it. Without the service, the app attempts to use its own keyboard, subject to compositor support.

The integration tracks requests from the app and uses COSMIC's `show`/`hide` actions. Automatic openings and external visibility changes still need validation.

### The app's Linux keyboard

Use the D-pad or left stick to choose a key. A types, B deletes, X sends Enter, and Y sends Escape. L3 inserts a space, L1 inserts Tab, and R1 toggles Shift. **Start** opens keyboard settings; **R3** closes it.

![The app's Linux on-screen keyboard](keyboard-preview.png)

Customize width, position, opacity, and repeat behavior in the settings. The displayed layout is US QWERTY; symbols and accents also depend on your active system layout.

## Comfort and preferences

The interface starts at 125%. Use **Interface size** at the top of the tour, or the app settings, to choose from 100% to 175%.

Cursor scaling starts at 100% of the size found on your system. The app applies a different size only when the preference requires it and saves customization when you adjust it. On exit, it restores the original size if it changed it.

The panel lets you adjust pointer speed, deadzone, and the selected controller. Controllers without a profile can receive an SDL mapping line, saved for future sessions. The option to start with your session is available in settings.

## Updating and quitting

Choose **Check for updates** in the panel footer. The footer also shows the running version.

- **Windows:** the app downloads the new executable and restarts.
- **Linux `.deb`:** installation requests system authorization.
- **Linux `.tar.gz`:** keep the executable in a directory writable by your user to allow replacement.

Hiding the panel keeps controller input active when the tray is available. To exit, choose **Quit application**. The process waits for cleanup, including cursor restoration, before terminating. If the system keeps rejecting restoration, the app keeps waiting.

## Compatibility and limitations

- The project provides Windows and Linux x64 builds; the physical controller and app compatibility matrix is still being validated.
- Command support depends on the active app. Desktop navigation uses the pointer and shortcuts, without promising accessible-element navigation in every app.
- On Windows, apps running with higher privileges may reject input. The virtual-key filter does not block games that read the controller directly.
- The app's Wayland keyboard requires `wlr-layer-shell`; it is unavailable on GNOME Wayland.
- On Windows, pausing may hide standard cursors while a controller is connected. Force-quitting can prevent restoration; reapplying the scheme in **Mouse settings → Pointers** restores them.
- Navigation and closing behavior for the compact and large Windows keyboard layouts need to be checked with a physical controller.

## Help and documentation

When [reporting an issue](https://github.com/andrecrjr/GamepadDesktop/issues), include the app version, operating system/desktop environment, controller model, USB/Bluetooth connection, and steps to reproduce it.

- [How it works and platform differences](docs/system-overview.md)
- [Development, builds, and assets](docs/development.md)
- [Roadmap and pending validation](docs/roadmap.md)
- [Keyboard research and history](docs/keyboard-research.md)
