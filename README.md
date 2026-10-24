# Gamepad Desktop

Use a gamepad to control the active desktop window, move the pointer, navigate accessible controls, and type with an on-screen keyboard. The app supports Windows and Linux; the browser extension is a separate project under [`extension/`](extension/README.md).

## Start

For local development, install Rust and the desktop dependencies described in the [system overview](desktop/docs/system-overview.md#como-iniciar-no-linux), then run:

```bash
make desktop-run
```

The Windows release ZIP and Linux release archive are available from GitHub Releases. The Linux archive contains the `gamepad-desktop` executable; see the [desktop guide](desktop/README.md) for platform setup and known limitations.

## Development commands

The root Makefile provides commands for the desktop app and the browser extension:

```bash
make desktop-check
make desktop-lint
make desktop-format
make desktop-test

make web-install
make web-dev
make web-build
make web-test
```

The browser extension has its own npm project and generated files under `extension/`.

## Brand assets

The approved logo lives in [`assets/gamepad-desktop.svg`](assets/gamepad-desktop.svg).
After editing it, run `make web-install` once, then `make brand-assets` to regenerate
the desktop SVG/PNG/ICO, site favicons and extension logos/icons. These generated
files are committed so Rust builds work without Node. CI checks that every copy
matches the source with `npm --prefix extension run brand:check`.

The Windows ICO includes 16, 20, 24, 32, 48, 64, 128 and 256 px frames. The desktop
uses the same mark in its header, window icon and system tray. Controller button
prompts are separate instructional assets.

## Releases

Conventional Commits on `main` drive semantic-release. A release uses a `desktop-v*` tag and attaches Windows x64 and Linux x64 desktop packages. The extension is validated and built separately; it is not included in desktop releases.
