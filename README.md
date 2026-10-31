<p align="center">
  <img src="assets/gamepad-desktop.svg" width="112" alt="Gamepad Desktop logo" />
</p>

# Gamepad Desktop

**Your controller can be your mouse and keyboard, too.**

Navigate your computer, open apps, scroll pages, and type with a gamepad. Built for using your desktop from a distance, on Windows and Linux.

[Download for Windows](https://github.com/andrecrjr/GamepadDesktop/releases/latest/download/GamepadDesktop-Windows-x64.exe) · [Download for Linux](https://github.com/andrecrjr/GamepadDesktop/releases/latest) · [User guide](desktop/README.md)

## What you can do

- **Point, click, and scroll:** use your controller's sticks and buttons to operate your apps.
- **Type:** open the on-screen keyboard to search, fill in a field, or write.
- **Switch between Desktop and Game:** double-tap Select / View to change modes.
- **Learn by doing:** follow the welcome tour with safe practice areas and demos, even without a connected controller.
- **Make it comfortable:** adjust pointer speed, deadzone, and interface size. Your preferences are saved.
- **Keep it running in the tray:** hide the panel and keep using your controller in the background.

## Download and get started

| System                               | Download                                                                                                                  | How to start                                                     |
| ------------------------------------ | ------------------------------------------------------------------------------------------------------------------------- | ---------------------------------------------------------------- |
| Windows x64                          | [Executable `.exe`](https://github.com/andrecrjr/GamepadDesktop/releases/latest/download/GamepadDesktop-Windows-x64.exe)  | Open `GamepadDesktop-Windows-x64.exe`.                           |
| Debian, Ubuntu, and derivatives, x64 | [Package `.deb`](https://github.com/andrecrjr/GamepadDesktop/releases/latest/download/GamepadDesktop-Linux-x64.deb)       | Install the package and open Gamepad Desktop from your launcher. |
| Other Linux distributions, x64       | [Archive `.tar.gz`](https://github.com/andrecrjr/GamepadDesktop/releases/latest/download/GamepadDesktop-Linux-x64.tar.gz) | Extract it and run `gamepad-desktop`.                            |

1. Connect your controller over USB or Bluetooth.
2. Open Gamepad Desktop and check that your controller is recognized.
3. Follow the welcome tour and choose your starting mode.

On Linux, mouse and keyboard control requires access to `/dev/uinput`. See [installation and permissions](desktop/README.md#linux) if the panel reports an input error.

To update, choose **Check for updates** in the panel footer. The footer also shows the version you are running.

## Learn the essentials

The welcome tour covers five stages:

1. **Mouse:** move the pointer and click.
2. **Scrolling:** move through a page.
3. **Keyboard:** open it, type, and close it.
4. **Modes:** switch between Desktop and Game.
5. **Get started:** choose your starting mode.

Each stage contains short practice steps. Try the action or choose **Watch a demo**, then press **Continue**. Practice stays inside the tour without sending commands to your other apps.

Right-clicking, dragging, switching windows, screenshots, and pausing are available under **Learn more**, both at the end of the tour and in the panel footer. You can replay the welcome tour at any time.

## Everyday controls

| Controller input             | Action                                                    |
| ---------------------------- | --------------------------------------------------------- |
| Left stick                   | Move the pointer                                          |
| Right stick, up/down         | Scroll the page                                           |
| A / ✕ — bottom button        | Click; hold to drag                                       |
| B / ○ — right button         | Right-click                                               |
| R3 — press the right stick   | Open the keyboard in Desktop mode; close an open keyboard |
| Select / View + R3           | Open the keyboard in Game mode                            |
| Select / View, double-tap    | Switch between Desktop and Game                           |
| Start + Select / Menu + View | Pause or resume Desktop controls                          |

After connecting or resuming, release the buttons and center the sticks. The panel includes a [complete control guide](desktop/README.md#controls).

## Desktop or Game?

**Desktop** turns controller inputs into mouse and keyboard actions. Use it to navigate your system, websites, and apps.

**Game** leaves input to your game or an app with native controller support. Gamepad Desktop stops sending its regular mouse and keyboard commands while keeping the shortcut to open the on-screen keyboard.

## Compatibility

The app uses SDL3 profiles to recognize different controller families. Controllers without a profile may need a mapping imported through the panel.

- **Windows:** uses the system touch keyboard; available layouts and navigation depend on Windows.
- **Linux/X11:** uses the app's own on-screen keyboard.
- **COSMIC/Wayland:** integrates with the desktop keyboard when its service is available.
- **Other Wayland sessions:** the app's keyboard depends on compositor support. It is unavailable on GNOME Wayland.

The project is in development. Physical controller, app, and environment validation is tracked in the [roadmap](desktop/docs/roadmap.md). See the [compatibility guide](desktop/README.md#compatibility-and-limitations) for details.

## Learn more

- [Installation, controls, and troubleshooting](desktop/README.md)
- [How the app works](desktop/docs/system-overview.md)
- [Development and builds](desktop/docs/development.md)
- [Report an issue](https://github.com/andrecrjr/GamepadDesktop/issues)
- [Support the project](https://ko-fi.com/andrecrjr)

The [browser extension](extension/README.md) is a separate project. The desktop app works without it.

Licensed under [MIT](LICENSE).
