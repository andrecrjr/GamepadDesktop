//! Read-only discovery for the experimental controller filter.
//! Run on Windows with the controller connected; no driver is installed.

#[cfg(not(target_os = "windows"))]
fn main() {
    eprintln!("Run this diagnostic on Windows with a controller connected.");
    std::process::exit(1);
}

#[cfg(target_os = "windows")]
fn main() -> Result<(), Box<dyn std::error::Error>> {
    use gamepad_desktop::gamepads::Gamepads;
    let mut gamepads = Gamepads::new()?;
    gamepads.poll(None)?;
    let identity = gamepads
        .active_identity()
        .ok_or("No mapped controller connected")?;
    let native = identity
        .path
        .as_deref()
        .ok_or_else(|| "SDL did not expose a device path".to_owned())
        .and_then(|path| resolve(path).map_err(|error| error.to_string()));
    println!(
        "{}",
        serde_json::to_string_pretty(&serde_json::json!({
            "controller": gamepads.name(),
            "selected": identity,
            "windows_device": native.as_ref().ok(),
            "resolution_error": native.as_ref().err(),
            "isolation_active": false,
            "driver_implemented": false,
        }))?
    );
    native.map(|_| ()).map_err(Into::into)
}

#[cfg(target_os = "windows")]
fn resolve(path: &str) -> windows::core::Result<serde_json::Value> {
    use windows::{
        Win32::{
            Devices::DeviceAndDriverInstallation::*,
            Foundation::{ERROR_INSUFFICIENT_BUFFER, ERROR_INVALID_DATA},
        },
        core::{Error, PCWSTR},
    };
    struct DeviceSet(HDEVINFO);
    impl Drop for DeviceSet {
        fn drop(&mut self) {
            // SAFETY: this set is owned here and destroyed exactly once.
            unsafe {
                let _ = SetupDiDestroyDeviceInfoList(self.0);
            }
        }
    }
    if path.contains('\0') {
        return Err(Error::from(ERROR_INVALID_DATA));
    }
    let path: Vec<u16> = path.encode_utf16().chain(Some(0)).collect();
    // SAFETY: all structures have their required sizes; buffers remain alive
    // throughout synchronous SetupAPI calls. No device settings are modified.
    unsafe {
        let set = DeviceSet(SetupDiCreateDeviceInfoList(None, None)?);
        let mut interface = SP_DEVICE_INTERFACE_DATA {
            cbSize: size_of::<SP_DEVICE_INTERFACE_DATA>() as u32,
            ..Default::default()
        };
        SetupDiOpenDeviceInterfaceW(set.0, PCWSTR(path.as_ptr()), 0, Some(&mut interface))?;
        let mut device = SP_DEVINFO_DATA {
            cbSize: size_of::<SP_DEVINFO_DATA>() as u32,
            ..Default::default()
        };
        // SetupAPI fills DeviceInfoData even when the detail buffer is too small.
        let result =
            SetupDiGetDeviceInterfaceDetailW(set.0, &interface, None, 0, None, Some(&mut device));
        if let Err(error) = result
            && error.code() != ERROR_INSUFFICIENT_BUFFER.to_hresult()
        {
            return Err(error);
        }
        let mut required = 0;
        let result = SetupDiGetDeviceInstanceIdW(set.0, &device, None, Some(&mut required));
        if let Err(error) = result
            && error.code() != ERROR_INSUFFICIENT_BUFFER.to_hresult()
        {
            return Err(error);
        }
        if required == 0 || required > 32768 {
            return Err(Error::from(ERROR_INVALID_DATA));
        }
        let mut id = vec![0u16; required as usize];
        SetupDiGetDeviceInstanceIdW(set.0, &device, Some(&mut id), None)?;
        let end = id
            .iter()
            .position(|value| *value == 0)
            .ok_or_else(|| Error::from(ERROR_INVALID_DATA))?;
        let instance_id =
            String::from_utf16(&id[..end]).map_err(|_| Error::from(ERROR_INVALID_DATA))?;
        Ok(serde_json::json!({
            "instance_id": instance_id,
            "interface_class": format!("{:?}", interface.InterfaceClassGuid),
            "device_class": format!("{:?}", device.ClassGuid),
        }))
    }
}
