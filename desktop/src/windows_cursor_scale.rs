use windows::{
    Win32::{
        Foundation::{ERROR_FILE_NOT_FOUND, ERROR_SUCCESS},
        System::Registry::{
            HKEY, HKEY_CURRENT_USER, KEY_QUERY_VALUE, KEY_SET_VALUE, REG_SZ, REG_VALUE_TYPE,
            RegCloseKey, RegDeleteValueW, RegOpenKeyExW, RegQueryValueExW, RegSetValueExW,
        },
    },
    core::w,
};

use crate::cursor_scale::{CursorScale, CursorScaleRange, scaled_size};

// Largest mouse pointer size offered by Windows accessibility settings.
const MAX_CURSOR_SIZE: u32 = 256;
const DEFAULT_CURSOR_SIZE: u32 = 32;

pub(crate) struct WindowsCursorScale {
    original: Option<String>,
    base_size: u32,
    max_size: u32,
    current_size: Option<u32>,
    modified: bool,
    supported: bool,
}

impl WindowsCursorScale {
    pub(crate) fn new() -> Self {
        let original = match read_cursor_size() {
            Ok(value) => value,
            Err(_) => return Self::unsupported(),
        };
        let base_size = match original.as_deref() {
            Some(value) => match value.parse::<u32>() {
                Ok(size) if size > 0 => size,
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
        write_cursor_size(&size.to_string())?;
        reload_system_cursors()
    }
}

impl Drop for WindowsCursorScale {
    fn drop(&mut self) {
        if !self.modified {
            return;
        }
        let _ = match self.original.as_deref() {
            Some(value) => write_cursor_size(value),
            None => delete_cursor_size(),
        };
        let _ = reload_system_cursors();
    }
}

fn read_cursor_size() -> Result<Option<String>, String> {
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
        if status != ERROR_SUCCESS || value_type != REG_SZ {
            return Err(format!("Could not read Windows cursor size ({})", status.0));
        }

        let mut value = vec![0_u16; byte_count.div_ceil(2) as usize];
        let status = unsafe {
            RegQueryValueExW(
                key,
                w!("CursorBaseSize"),
                None,
                Some(&mut value_type),
                Some(value.as_mut_ptr().cast()),
                Some(&mut byte_count),
            )
        };
        if status != ERROR_SUCCESS {
            return Err(format!("Could not read Windows cursor size ({})", status.0));
        }
        value.truncate(byte_count as usize / 2);
        Ok(Some(
            String::from_utf16_lossy(&value)
                .trim_end_matches('\0')
                .to_owned(),
        ))
    })
}

fn write_cursor_size(value: &str) -> Result<(), String> {
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
