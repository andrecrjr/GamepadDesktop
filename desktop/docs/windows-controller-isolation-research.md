# Windows controller isolation: can we build it ourselves?

Research date: 2026-10-26. Scope: keep mouse control in another Windows application
while preventing that application from also acting on the physical controller.
This is a feasibility report, not an approved architecture or an implemented fix.

**Finding:** we can build our own device filter. I found no documented ordinary
background-app API that guarantees this isolation across all controller APIs.
The difficult parts are device coverage, already-open handles, mode changes, and
shipping a signed driver. A small diagnostic experiment should precede a driver.

## What our app currently does

- `src/gamepads.rs`: SDL reads the selected controller, with background events
  enabled. Reading a controller does not consume its state for other processes.
- `src/windows_gamepad_hook.rs`: a keyboard hook suppresses Windows gamepad
  virtual-key messages. It does not intercept another process's controller API.
- `src/windows.rs`: mouse mode enables that hook and filters synthetic D-pad
  actions. This does not establish exclusive ownership of the physical device.
- `src/runtime.rs`: Desktop mode and the touch-keyboard session determine when
  synthesized mouse input is active.

GitNexus repository: `Gamepad-Navigator`, commit
`325237dc4158474a040d5bfd480227e2f25ab838`. All four files above matched the
index hashes during research. The graph reports a missing caller edge for
`native_mouse_state`; the runtime call was also verified in source.

The reported double input is consistent with independent physical-controller
reads. It does not establish which API each affected application uses, or rule
out an additional remapper generating ordinary keyboard/mouse input.

Microsoft documents the keyboard hook as handling keyboard messages, while Raw
Input follows a separate registration and `WM_INPUT` path.
[Keyboard hook](https://learn.microsoft.com/en-us/windows/win32/winmsg/lowlevelkeyboardproc),
[Raw Input](https://learn.microsoft.com/en-us/windows/win32/inputdev/about-raw-input).

## Alternatives checked

| Approach                           | What it can do                                               | Why it does not establish general isolation                                         |
| ---------------------------------- | ------------------------------------------------------------ | ----------------------------------------------------------------------------------- |
| Extend our keyboard hook           | Suppress additional keyboard messages                        | Cannot stop direct controller polling                                               |
| Change our SDL settings            | Change our process's device handling                         | Does not configure independently running applications                               |
| Register Raw Input                 | Subscribe to device reports                                  | Registration is not a global device lock                                            |
| GameInput exclusive focus policy   | Suppress other GameInput processes while the owner has focus | Our app needs background operation; the guarantee is limited to GameInput           |
| DirectInput exclusive cooperation  | Request a DirectInput cooperative mode                       | Microsoft explicitly allows nonexclusive access alongside it                        |
| Exclusive HID handle               | Potentially reserve a particular device interface            | Can fail because of existing handles; coverage of other interfaces/APIs is unproven |
| Filter driver we own               | Mediate access below application input APIs                  | Requires driver development, signing, device coverage, and transition tests         |
| Configure a cooperating target app | Disable that app's controller support                        | Useful for specific apps, not arbitrary programs                                    |

**GameInput:** `GameInputExclusiveForegroundInput` applies only while the owner
has focus and only to other GameInput consumers. Combining it with background
input does not remove the documented foreground condition. Keeping a transparent
window focused would interfere with the destination application's normal input.
[Microsoft focus-policy definitions](https://learn.microsoft.com/en-us/gaming/gdk/docs/reference/input/gameinput/enums/gameinputfocuspolicy?view=gdk-2604).

**DirectInput:** its exclusive cooperative flag is not a general physical-device
lock. SDL's current DirectInput backend already requests exclusive/background
cooperation, with a comment distinguishing shared input from force-feedback
exclusivity. This is upstream evidence, not proof of our bundled SDL version's
runtime behavior.
[Microsoft cooperative levels](<https://learn.microsoft.com/en-us/previous-versions/windows/desktop/ee417921(v=vs.85)>),
[SDL implementation](https://github.com/libsdl-org/SDL/blob/main/src/joystick/windows/SDL_dinputjoystick.c).

**Direct HID:** `CreateFileW` with zero sharing can reserve an interface if the
open succeeds; incompatible existing handles cause failure. SDL's current
Windows HIDAPI implementation opens with read/write sharing. Replacing that
path would require proving that our reader still works and that XInput, Raw
Input, and other exposed interfaces stop receiving input. A successful HID open
alone does not prove that, especially for Xbox transports.
[CreateFile sharing rules](https://learn.microsoft.com/en-us/windows/win32/api/fileapi/nf-fileapi-createfilew),
[SDL HIDAPI source](https://github.com/libsdl-org/SDL/blob/main/src/hidapi/windows/hid.c).

**Cooperating SDL apps:** SDL3 offers `SDL_JOYSTICK_BLACKLIST_DEVICES` using
VID/PID pairs. A launcher could supply it to compatible child applications.
That can exclude two identical controllers together, depends on the SDL version
and target behavior, and does not disable independent XInput/HID reads. Setting
it in our app does not change an already-running application's environment.
[SDL hint](https://wiki.libsdl.org/SDL3/SDL_HINT_JOYSTICK_BLACKLIST_DEVICES),
[SDL environment variables](https://wiki.libsdl.org/SDL3/EnvironmentVariables).

## What a filter we own would involve

This is a candidate to evaluate, not an architecture selection:

1. Identify the selected physical device and every relevant interface it exposes.
   Our SDL instance ID and mapping GUID are not sufficient proof of a unique
   Windows device-instance identity, particularly across reconnects.
2. Attach a Windows device filter at the appropriate stack location.
3. Authenticate the controller-owning client and permit its access; deny other
   clients for the selected device while isolation is requested.
4. Tie isolation ownership to a live session. Release on clean shutdown, process
   death, disconnect, and mode changes. Define recovery for an unresponsive app.
5. Leave unrelated controllers and keyboard/mouse interfaces available.

The concrete mechanism exists: WDF provides `EvtDeviceFileCreate` to mediate
device opens and APIs to inspect requestor identity. HidHide's `Device.c`
registers that callback; its `Logic.c` checks device and process policy, denies
with `STATUS_ACCESS_DENIED`, or forwards the open. The examined revision is
`2b950fd9393e1644b4199f6eb4999e1720f0c6e9`.
[WDF callback](https://learn.microsoft.com/en-us/windows-hardware/drivers/ddi/wdfdevice/nc-wdfdevice-evt_wdf_device_file_create),
[Requestor identity](https://learn.microsoft.com/en-us/windows-hardware/drivers/ddi/wdfrequest/nf-wdfrequest-wdfrequestgetrequestorprocessid),
[Device setup](https://github.com/nefarius/HidHide/blob/2b950fd9393e1644b4199f6eb4999e1720f0c6e9/HidHide/src/Device.c),
[Access policy](https://github.com/nefarius/HidHide/blob/2b950fd9393e1644b4199f6eb4999e1720f0c6e9/HidHide/src/Logic.c).

**Mode-switching trap:** that implementation checks access when a handle opens.
Inference: changing the policy does not itself revoke an already-open handle.
Blocking an app launched afterward is therefore a weaker result than stopping
an app already reading the controller. Restart/reconnect, filtering ongoing I/O,
or hiding continuously and forwarding through a virtual controller are separate
designs to investigate. None has been validated here. Brokered/system reads also
need testing: the process opening the device may not be the final consumer.
[Examined access-check implementation](https://github.com/nefarius/HidHide/blob/2b950fd9393e1644b4199f6eb4999e1720f0c6e9/HidHide/src/Logic.c).

Xbox coverage needs more than assuming every device is generic HID. HidHide's
installer manages filters for HIDClass, XnaComposite, and XboxComposite. USB,
Bluetooth, and a wireless adapter must be evaluated separately.
[Installer source](https://github.com/nefarius/HidHide/blob/master/HidHide/ReinstallDriver.cmd).

## Framework, language, and distribution constraints

- **KMDF is a demonstrated candidate.** It is what HidHide uses. UMDF supports
  device-specific filters, but not class filters, and imposes restrictions on
  kernel clients. Its suitability for a narrow controller must be demonstrated;
  “user-mode driver” still means an installed driver, not a portable app feature.
  [UMDF filter installation](https://learn.microsoft.com/en-us/windows-hardware/drivers/wdf/installing-a-umdf-filter-driver),
  [Kernel-client restrictions](https://learn.microsoft.com/en-us/windows-hardware/drivers/wdf/supporting-kernel-mode-clients-in-umdf-drivers).
- **Rust is possible but adds a tooling choice.** Microsoft's
  `windows-drivers-rs` supports driver experimentation, but its README currently
  says it is not recommended for production. C with KMDF is another candidate;
  the existing desktop app can remain Rust in either case.
  [Microsoft driver tooling](https://github.com/microsoft/windows-drivers-rs).
- **A virtual controller is additional work.** Microsoft's VHF creates virtual
  HID devices using a kernel-mode source driver. It does not hide a physical pad
  or promise Xbox/XInput compatibility. It is unnecessary merely to prove that
  we can isolate a controller while generating mouse input.
  [VHF](https://learn.microsoft.com/en-us/windows-hardware/drivers/hid/virtual-hid-framework--vhf-).
- **Our own production driver needs its own signing process.** Microsoft's
  documented submission path requires Hardware Developer registration and an
  associated EV certificate. Rebuilding an open-source driver does not inherit
  its publisher's signature. Test signing is a development configuration, with
  reboot and Secure Boot implications; it is not the normal-user delivery path.
  [Signing requirements](https://learn.microsoft.com/en-us/windows-hardware/drivers/dashboard/code-signing-reqs),
  [Test-signed code](https://learn.microsoft.com/en-us/windows-hardware/drivers/install/the-testsigning-boot-configuration-option).
- **Source reuse is possible.** HidHide's MIT license permits modification and
  redistribution subject to preserving its notices. Building from that source
  and writing a fresh filter are different maintenance choices; neither avoids
  driver packaging, validation, or signing for a rebuilt binary.
  [License](https://github.com/nefarius/HidHide/blob/2b950fd9393e1644b4199f6eb4999e1720f0c6e9/LICENSE).

## Smallest useful experiment before deciding

Recommended next research step: a Windows-only diagnostic harness, without
changing the production input architecture.

1. Record controller model, transport, Windows version, and an affected program.
   Disable our synthesized actions temporarily to distinguish physical reads
   from ordinary keys/mouse events produced by another remapper.
2. Run independent controller readers in separate processes: SDL, XInput,
   GameInput, Raw Input, and direct HID where supported. Record enumeration and
   nonneutral state, not just whether a window visibly responds.
3. Try the narrow no-driver candidates: GameInput focus policy and exclusive
   access to the actual HID interface, where available. Keep the destination app
   focused and test with readers both already running and launched afterward.
4. If those fail, prototype a filter for one identified controller/transport in
   a dedicated Windows test environment. Start with pass-through behavior, then
   allow our reader and deny a second process's new opens.
5. Test Desktop → Game → Desktop while a consumer already holds the controller.
   Failure here means startup isolation works, but the product requirement does
   not. Treat smooth switching as a separate acceptance criterion.
6. Repeat after process termination, disconnect/reconnect, sleep/wake, and with
   a second identical controller. Confirm keyboard and mouse remain usable.

Use RED → GREEN for the eventual implementation: first demonstrate nonneutral
state in the second process during Desktop mode, then require it to stop while
our process still reads motion. Require native input to return in Game mode and
after owner termination. Unit tests alone cannot establish this OS behavior.

No Windows hardware experiments or driver installation were performed during
this research. The decision remains open: limited controller-specific support,
source-based filter ownership, or a fresh filter with a defined support matrix.
