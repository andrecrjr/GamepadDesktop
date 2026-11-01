use std::time::Duration;

use serde::{Deserialize, Serialize};

pub const MIN_CURSOR_SCALE: u16 = 100;
pub(crate) const LEGACY_DEFAULT_CURSOR_SCALE: u16 = 135;
const CURSOR_RESTORE_RETRY_INTERVAL: Duration = Duration::from_millis(100);

pub(crate) fn retry_cursor_restore<E>(mut restore: impl FnMut() -> Result<(), E>) {
    while restore().is_err() {
        std::thread::sleep(CURSOR_RESTORE_RETRY_INTERVAL);
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(transparent)]
pub struct CursorScale(u16);

impl CursorScale {
    pub const fn new(percent: u16) -> Self {
        Self(if percent < MIN_CURSOR_SCALE {
            MIN_CURSOR_SCALE
        } else {
            percent
        })
    }

    pub const fn percent(self) -> u16 {
        self.0
    }
}

impl Default for CursorScale {
    fn default() -> Self {
        Self(MIN_CURSOR_SCALE)
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct CursorScaleRange {
    pub supported: bool,
    pub max_percent: u16,
}

impl CursorScaleRange {
    pub const fn unsupported() -> Self {
        Self {
            supported: false,
            max_percent: MIN_CURSOR_SCALE,
        }
    }

    pub fn from_sizes(base: u32, max: u32) -> Self {
        Self {
            supported: base > 0,
            max_percent: max_percent(base, max),
        }
    }
}

impl Default for CursorScaleRange {
    fn default() -> Self {
        Self::unsupported()
    }
}

pub fn max_percent(base: u32, max: u32) -> u16 {
    if base == 0 {
        return MIN_CURSOR_SCALE;
    }
    (u64::from(max.max(base)) * 100 / u64::from(base)).min(u64::from(u16::MAX)) as u16
}

pub fn scaled_size(base: u32, percent: u16, max: u32) -> u32 {
    if base == 0 {
        return 0;
    }
    base.saturating_mul(u32::from(percent))
        .saturating_add(50)
        .checked_div(100)
        .unwrap_or(base)
        .clamp(base, max.max(base))
}

#[cfg(test)]
mod tests {
    use super::{CursorScale, CursorScaleRange, max_percent, scaled_size};

    #[test]
    fn scale_range_uses_the_available_system_size() {
        assert_eq!(CursorScaleRange::from_sizes(32, 256).max_percent, 800);
        assert_eq!(CursorScaleRange::from_sizes(24, 96).max_percent, 400);
    }

    #[test]
    fn scaled_size_rounds_and_stays_within_the_system_limit() {
        assert_eq!(scaled_size(24, 135, 96), 32);
        assert_eq!(scaled_size(32, 135, 256), 43);
        assert_eq!(scaled_size(32, 800, 256), 256);
        assert_eq!(scaled_size(32, 900, 256), 256);
        assert_eq!(scaled_size(32, 100, 256), 32);
        assert_eq!(max_percent(0, 256), 100);
    }

    #[test]
    fn cursor_scale_never_goes_below_100_percent() {
        assert_eq!(CursorScale::new(0).percent(), 100);
        assert_eq!(CursorScale::new(135).percent(), 135);
    }

    #[test]
    fn cursor_restore_retries_until_success() {
        let mut attempts = 0;
        super::retry_cursor_restore(|| {
            attempts += 1;
            if attempts == 1 {
                Err("temporary restore failure")
            } else {
                Ok(())
            }
        });

        assert_eq!(attempts, 2);
    }
}
