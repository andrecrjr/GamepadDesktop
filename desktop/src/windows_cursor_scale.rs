use windows::{
    Win32::{
        Foundation::{ERROR_FILE_NOT_FOUND, ERROR_SUCCESS},
        System::Registry::{
            HKEY, HKEY_CURRENT_USER, KEY_QUERY_VALUE, KEY_SET_VALUE, REG_DWORD, REG_SZ,
            REG_VALUE_TYPE, RegCloseKey, RegDeleteValueW, RegOpenKeyExW, RegQueryValueExW,
            RegSetValueExW,
        },
        UI::WindowsAndMessaging::{
            SPIF_SENDCHANGE, SYSTEM_PARAMETERS_INFO_ACTION, SystemParametersInfoW,
        },
    },
    core::w,
};

use crate::cursor_scale::{CursorScale, CursorScaleRange, retry_cursor_restore, scaled_size};

// Largest mouse pointer size offered by Windows accessibility settings.
const MAX_CURSOR_SIZE: u32 = 256;
const DEFAULT_CURSOR_SIZE: u32 = 32;
const SPI_SET_CURSOR_BASE_SIZE: SYSTEM_PARAMETERS_INFO_ACTION =
    SYSTEM_PARAMETERS_INFO_ACTION(0x2029);

#[derive(Clone, Debug, PartialEq, Eq)]
enum RegistryCursorSize {
    Dword(u32),
    String(String),
}

impl RegistryCursorSize {
    fn pixels(&self) -> Option<u32> {
        match self {
            Self::Dword(size) => Some(*size),
            Self::String(size) => size.parse().ok(),
        }
    }
}

fn parse_cursor_size_value(
    value_type: REG_VALUE_TYPE,
    bytes: &[u8],
) -> Result<RegistryCursorSize, String> {
    if value_type == REG_DWORD {
        let bytes: [u8; 4] = bytes
            .try_into()
            .map_err(|_| "Invalid Windows cursor size registry value".to_owned())?;
        return Ok(RegistryCursorSize::Dword(u32::from_le_bytes(bytes)));
    }

    if value_type != REG_SZ || !bytes.len().is_multiple_of(2) {
        return Err("Unsupported Windows cursor size registry value".to_owned());
    }
    let units = bytes
        .chunks_exact(2)
        .map(|pair| u16::from_le_bytes([pair[0], pair[1]]))
        .collect::<Vec<_>>();
    Ok(RegistryCursorSize::String(
        String::from_utf16_lossy(&units)
            .trim_end_matches('\0')
            .to_owned(),
    ))
}

fn encode_cursor_size(size: u32) -> (REG_VALUE_TYPE, [u8; 4]) {
    (REG_DWORD, size.to_le_bytes())
}

pub(crate) struct WindowsCursorScale {
    original: Option<RegistryCursorSize>,
    base_size: u32,
    max_size: u32,
    current_size: Option<u32>,
    modified: bool,
    supported: bool,
}

#[cfg(test)]
mod tests {
    use super::{RegistryCursorSize, encode_cursor_size, parse_cursor_size_value};
    use windows::Win32::System::Registry::{REG_DWORD, REG_SZ};

    #[test]
    fn reads_windows_cursor_base_size_as_dword() {
        assert_eq!(
            parse_cursor_size_value(REG_DWORD, &32_u32.to_le_bytes()),
            Ok(RegistryCursorSize::Dword(32))
        );
    }

    #[test]
    fn preserves_legacy_string_cursor_size_values() {
        let value = "48\0"
            .encode_utf16()
            .flat_map(u16::to_le_bytes)
            .collect::<Vec<_>>();
        assert_eq!(
            parse_cursor_size_value(REG_SZ, &value),
            Ok(RegistryCursorSize::String("48".to_owned()))
        );
    }

    #[test]
    fn writes_windows_cursor_base_size_as_dword() {
        let (value_type, bytes) = encode_cursor_size(43);
        assert_eq!(value_type, REG_DWORD);
        assert_eq!(bytes, 43_u32.to_le_bytes());
    }
}

impl WindowsCursorScale {
    pub(crate) fn new() -> Self {
        let original = match read_cursor_size() {
            Ok(value) => value,
            Err(_) => return Self::unsupported(),
        };
        let base_size = match original.as_ref() {
            Some(value) => match value.pixels() {
                Some(size) if size > 0 => size,
                _ => return Self::unsupported(),
            },
            None => DEFAULT_CURSOR_SIZE,
        };
        Self {
            original,
            base_size,
            max_size: MAX_CURSOR_SIZE.max(base_size),
            current_size: Some(base_size),
            modified: false,
            supported: true,
        }
    }

    fn unsupported() -> Self {
        Self {
            original: None,
            base_size: 0,
            max_size: 0,
            current_size: None,
            modified: false,
            supported: false,
        }
    }

    pub(crate) fn range(&self) -> CursorScaleRange {
        if self.supported {
            CursorScaleRange::from_sizes(self.base_size, self.max_size)
        } else {
            CursorScaleRange::unsupported()
        }
    }

    pub(crate) fn set(&mut self, scale: CursorScale) -> Result<bool, String> {
        if !self.supported {
            return Ok(false);
        }
        let size = scaled_size(self.base_size, scale.percent(), self.max_size);
        if self.current_size == Some(size) {
            return Ok(false);
        }
        self.current_size = Some(size);
        self.modified = true;
        self.apply_size(size)?;
        Ok(true)
    }

    fn apply_size(&self, size: u32) -> Result<(), String> {
        write_cursor_size(size)?;
        apply_system_cursor_size(size)
    }

    fn restore(&mut self) -> Result<(), String> {
        if !self.modified {
            return Ok(());
        }
        match self.original.as_ref() {
            Some(value) => write_cursor_size_value(value)?,
            None => delete_cursor_size()?,
        }
        apply_system_cursor_size(self.base_size)?;
        self.current_size = Some(self.base_size);
        self.modified = false;
        Ok(())
    }
}

impl Drop for WindowsCursorScale {
    fn drop(&mut self) {
        retry_cursor_restore(|| self.restore());
    }
}

fn read_cursor_size() -> Result<Option<RegistryCursorSize>, String> {
    with_cursor_key(KEY_QUERY_VALUE, |key| {
        let mut value_type = REG_VALUE_TYPE(0);
        let mut byte_count = 0;
        let status = unsafe {
            RegQueryValueExW(
                key,
                w!("CursorBaseSize"),
                None,
                Some(&mut value_type),
                None,
                Some(&mut byte_count),
            )
        };
        if status == ERROR_FILE_NOT_FOUND {
            return Ok(None);
        }
        if status != ERROR_SUCCESS {
            return Err(format!("Could not read Windows cursor size ({})", status.0));
        }

        let mut value = vec![0_u8; byte_count as usize];
        let status = unsafe {
            RegQueryValueExW(
                key,
                w!("CursorBaseSize"),
                None,
                Some(&mut value_type),
                Some(value.as_mut_ptr()),
                Some(&mut byte_count),
            )
        };
        if status != ERROR_SUCCESS {
            return Err(format!("Could not read Windows cursor size ({})", status.0));
        }
        value.truncate(byte_count as usize);
        parse_cursor_size_value(value_type, &value).map(Some)
    })
}

fn write_cursor_size(value: u32) -> Result<(), String> {
    let (value_type, bytes) = encode_cursor_size(value);
    with_cursor_key(KEY_SET_VALUE, |key| {
        let status =
            unsafe { RegSetValueExW(key, w!("CursorBaseSize"), None, value_type, Some(&bytes)) };
        if status == ERROR_SUCCESS {
            Ok(())
        } else {
            Err(format!("Could not set Windows cursor size ({})", status.0))
        }
    })
}

fn write_cursor_size_value(value: &RegistryCursorSize) -> Result<(), String> {
    match value {
        RegistryCursorSize::Dword(size) => write_cursor_size(*size),
        RegistryCursorSize::String(value) => write_cursor_size_string(value),
    }
}

fn write_cursor_size_string(value: &str) -> Result<(), String> {
    let value: Vec<u16> = format!("{value}\0").encode_utf16().collect();
    let bytes = unsafe { std::slice::from_raw_parts(value.as_ptr().cast::<u8>(), value.len() * 2) };
    with_cursor_key(KEY_SET_VALUE, |key| {
        let status =
            unsafe { RegSetValueExW(key, w!("CursorBaseSize"), None, REG_SZ, Some(bytes)) };
        if status == ERROR_SUCCESS {
            Ok(())
        } else {
            Err(format!("Could not set Windows cursor size ({})", status.0))
        }
    })
}

fn delete_cursor_size() -> Result<(), String> {
    with_cursor_key(KEY_SET_VALUE, |key| {
        let status = unsafe { RegDeleteValueW(key, w!("CursorBaseSize")) };
        if status == ERROR_SUCCESS || status == ERROR_FILE_NOT_FOUND {
            Ok(())
        } else {
            Err(format!(
                "Could not restore Windows cursor size ({})",
                status.0
            ))
        }
    })
}

fn with_cursor_key<T>(
    access: windows::Win32::System::Registry::REG_SAM_FLAGS,
    operation: impl FnOnce(HKEY) -> Result<T, String>,
) -> Result<T, String> {
    let mut key = HKEY::default();
    let status = unsafe {
        RegOpenKeyExW(
            HKEY_CURRENT_USER,
            w!("Control Panel\\Cursors"),
            None,
            access,
            &mut key,
        )
    };
    if status != ERROR_SUCCESS {
        return Err(format!(
            "Could not open Windows cursor settings ({})",
            status.0
        ));
    }
    let result = operation(key);
    let _ = unsafe { RegCloseKey(key) };
    result
}

fn reload_system_cursors() -> Result<(), String> {
    super::set_cursor_hidden(false)
        .map_err(|error| format!("Could not refresh Windows cursors: {error}"))
}

fn apply_system_cursor_size(size: u32) -> Result<(), String> {
    // Windows exposes this cursor-size action through SystemParametersInfoW
    // without a named constant in the SDK.
    unsafe {
        SystemParametersInfoW(
            SPI_SET_CURSOR_BASE_SIZE,
            0,
            Some(size as usize as *mut std::ffi::c_void),
            SPIF_SENDCHANGE,
        )
    }
    .map_err(|error| format!("Could not apply Windows cursor size ({error})"))?;
    reload_system_cursors()
}
