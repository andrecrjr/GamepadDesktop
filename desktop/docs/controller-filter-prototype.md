# Controller filter prototype

Status: device discovery implemented; kernel driver and background-input blocking
are not implemented. This diagnostic neither installs a driver nor changes devices.

The requested behavior is to select the first available mapped controller without
brand configuration, keep reading it for desktop navigation, and stop a game that
is already running from reacting after the user switches windows with R1.

## Discover the actual Windows interface

Run on Windows with the controller connected:

```powershell
cargo run --manifest-path desktop/Cargo.toml --example controller_identity
```

The diagnostic uses the same `Gamepads::poll(None)` selection as the app. Its JSON
contains the SDL session identifier and backend path, followed by the Windows
device instance ID and interface/device class GUIDs when SetupAPI resolves that
exact path. A missing or unresolvable path is an error; there is no fallback to a
different device, brand name, VID/PID pair, or XInput slot. This matters when two
identical controllers are connected. SDL instance IDs are not persistent Windows
hardware identities, and a resolved interface is not proof of isolation support.

Some SDL backends expose a synthetic path. Those require an additional verified
mapping to the Windows device before a filter can attach. The diagnostic exposes
that limitation rather than guessing a target.

## Remaining implementation boundary

A filter that denies only new device opens is insufficient: a game may already
have a handle. The next implementation must demonstrate filtering ongoing input
on the detected interface while our reader retains the real input. Processes
that broker input and separate interfaces of the same device must be accounted
for. Device-specific report neutralization or a virtual output device remains a
design decision until the interface is verified.

Before connecting a kernel filter to the app, validate on a Windows development
machine with WDK and the physical controller:

1. A separate consumer already reading the controller sees nonneutral input.
2. Isolation makes that consumer see released buttons and centered axes while
   desktop navigation still receives real input. Merely stopping reports is not
   sufficient because a game may retain the last pressed state.
3. Returning to Game mode restores input without restarting the consumer.
4. Disconnect, app termination, and reconnect restore normal device operation.
5. A second controller and keyboard/mouse interfaces remain unaffected.

No installed driver, signing package, or production isolation toggle is provided
by this discovery milestone. See the [earlier feasibility research](windows-controller-isolation-research.md)
for Windows API boundaries and driver distribution requirements.

SetupAPI resolves the path using
[SetupDiGetDeviceInterfaceDetailW](https://learn.microsoft.com/en-us/windows/win32/api/setupapi/nf-setupapi-setupdigetdeviceinterfacedetailw)
and `SetupDiGetDeviceInstanceIdW`; symbolic device paths are not manually parsed.
