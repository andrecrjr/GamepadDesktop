use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct MagneticCursorSettings {
    pub enabled: bool,
    pub strength: u8,
    pub center_hold: Option<u8>,
    pub detection_sensitivity: u8,
}

impl Default for MagneticCursorSettings {
    fn default() -> Self {
        Self {
            enabled: false,
            strength: 50,
            center_hold: None,
            detection_sensitivity: 50,
        }
    }
}

impl MagneticCursorSettings {
    pub fn normalized(mut self) -> Self {
        self.strength = self.strength.clamp(1, 100);
        self.center_hold = Some(self.center_hold.unwrap_or(self.strength).min(100));
        self.detection_sensitivity = self.detection_sensitivity.clamp(1, 100);
        self
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct MagneticCursorDiagnostics {
    pub tracking: bool,
    pub scanner_ready: bool,
    pub scanner_failed: bool,
    pub visual_candidates: usize,
    pub correction_px: u16,
}

#[cfg(target_os = "windows")]
pub struct VisualPreview {
    pub pixels: Vec<u8>,
    pub width: usize,
    pub height: usize,
}
#[cfg(target_os = "windows")]
static PREVIEW: std::sync::Mutex<Option<std::sync::Arc<VisualPreview>>> =
    std::sync::Mutex::new(None);
#[cfg(target_os = "windows")]
pub fn visual_preview() -> Option<std::sync::Arc<VisualPreview>> {
    PREVIEW.lock().ok()?.clone()
}

#[cfg(any(target_os = "windows", test))]
fn assisted_motion(
    cursor: (i32, i32),
    movement: (i32, i32),
    targets: &[crate::placement::Rect],
    radius: f64,
    settings: MagneticCursorSettings,
    selected: &mut Option<crate::placement::Rect>,
) -> (i32, i32) {
    let strength = settings.strength;
    let (mx, my) = (f64::from(movement.0), f64::from(movement.1));
    let speed = mx.hypot(my);
    if strength == 0 || !radius.is_finite() || speed >= radius {
        *selected = None;
        return movement;
    }
    if speed == 0.0 {
        return movement;
    }
    // Hold a reached center against gentle input; a clear push releases it.
    if speed <= f64::from(settings.center_hold.unwrap_or(strength).min(100)) / 25.0
        && let Some(target) = targets.iter().find(|target| {
            target.valid() && cursor == (target.x + target.width / 2, target.y + target.height / 2)
        })
    {
        *selected = Some(*target);
        return (0, 0);
    }
    let previous = *selected;
    let target = targets
        .iter()
        .filter(|button| button.valid())
        .filter_map(|button| {
            let right = button.x + button.width;
            let bottom = button.y + button.height;
            // Use a reachable pixel: fractional centers can round outside tiny targets.
            let dx = f64::from(button.x + button.width / 2) - f64::from(cursor.0);
            let dy = f64::from(button.y + button.height / 2) - f64::from(cursor.1);
            let approaching = dx * mx > 0.0 || dy * my > 0.0;
            let inside = cursor.0 >= button.x
                && cursor.0 < right
                && cursor.1 >= button.y
                && cursor.1 < bottom;
            let sliding_inside = inside && dx * mx == 0.0 && dy * my == 0.0;
            if (dx == 0.0 && dy == 0.0) || !(approaching || sliding_inside) {
                return None;
            }

            let nearest_x = cursor.0.clamp(button.x, right);
            let nearest_y = cursor.1.clamp(button.y, bottom);
            let distance = f64::from(nearest_x - cursor.0).hypot(f64::from(nearest_y - cursor.1));
            let short_side = f64::from(button.width.min(button.height));
            let size_factor = (48.0 / short_side).sqrt().clamp(1.0, 2.4);
            let target_radius = radius * size_factor;
            // Rank centers so a containing rectangle cannot hide a nearby icon.
            let mut priority = dx.hypot(dy) / target_radius;
            // A challenger must be clearly better, not just win a noisy near-tie.
            if previous.is_some_and(|previous| same_target(previous, *button)) {
                priority *= 0.75;
            }
            (distance < target_radius).then_some((
                dx,
                dy,
                distance,
                target_radius,
                priority,
                size_factor,
                *button,
            ))
        })
        .min_by(|a, b| a.4.total_cmp(&b.4));
    let Some((dx, dy, distance, target_radius, _, size_factor, button)) = target else {
        *selected = None;
        return movement;
    };
    *selected = Some(button);
    let (dx, dy) = (dx - mx, dy - my);
    let remaining = dx.hypot(dy);
    if remaining == 0.0 {
        return movement;
    }
    let amount = (speed.max(4.0) * f64::from(strength.min(100)) / 100.0
        * 2.8
        * size_factor
        * (1.0 - distance / target_radius).sqrt())
    .min(remaining)
        / remaining;
    (
        (mx + dx * amount).round() as i32,
        (my + dy * amount).round() as i32,
    )
}

#[cfg(any(target_os = "windows", test))]
fn same_target(previous: crate::placement::Rect, current: crate::placement::Rect) -> bool {
    let Some(overlap) = previous.intersection(current) else {
        return false;
    };
    let intersection = i64::from(overlap.width) * i64::from(overlap.height);
    let union = i64::from(previous.width) * i64::from(previous.height)
        + i64::from(current.width) * i64::from(current.height)
        - intersection;
    intersection * 2 > union
}

#[cfg(any(target_os = "windows", test))]
fn detect_small_rectangles(
    gray: &image::GrayImage,
    sensitivity: u8,
) -> Vec<crate::placement::Rect> {
    use crate::placement::Rect;
    use imageproc::contours::{BorderType, find_contours};

    let mut inverted = gray.clone();
    image::imageops::invert(&mut inverted);
    let threshold = detection_threshold(25, sensitivity) as u8;
    let mut targets = Vec::new();
    // Detect both dark-on-light and light-on-dark outlines at native resolution.
    for source in [gray, &inverted] {
        let mut mask = imageproc::contrast::adaptive_threshold(
            source,
            7,
            detection_threshold(8, sensitivity) as i32,
        );
        image::imageops::invert(&mut mask);
        for contour in find_contours::<i32>(&mask) {
            if contour.border_type != BorderType::Outer || contour.points.len() < 8 {
                continue;
            }
            let left = contour.points.iter().map(|p| p.x).min().unwrap();
            let right = contour.points.iter().map(|p| p.x).max().unwrap();
            let top = contour.points.iter().map(|p| p.y).min().unwrap();
            let bottom = contour.points.iter().map(|p| p.y).max().unwrap();
            let (width, height) = (right - left + 1, bottom - top + 1);
            if !(4..=40).contains(&width)
                || !(4..=40).contains(&height)
                || width > height * 4
                || height > width * 4
                || left < 2
                || top < 2
                || right + 2 >= gray.width() as i32
                || bottom + 2 >= gray.height() as i32
            {
                continue;
            }
            let edge = |x: i32, y: i32, dx: i32, dy: i32| {
                let pixel = gray.get_pixel(x as u32, y as u32)[0];
                // Check two outside pixels so the inside of an outline is not a second target.
                (1..=2).all(|step| {
                    pixel
                        .abs_diff(gray.get_pixel((x + dx * step) as u32, (y + dy * step) as u32)[0])
                        >= threshold
                })
            };
            let horizontal = |y, dy| {
                (left + 1..right).filter(|x| edge(*x, y, 0, dy)).count() * 10
                    >= (width - 2) as usize * 9
            };
            let vertical = |x, dx| {
                (top + 1..bottom).filter(|y| edge(x, *y, dx, 0)).count() * 10
                    >= (height - 2) as usize * 9
            };
            if !horizontal(top, -1)
                || !horizontal(bottom, 1)
                || !vertical(left, -1)
                || !vertical(right, 1)
            {
                continue;
            }
            let target = Rect {
                x: left,
                y: top,
                width,
                height,
            };
            if !targets.contains(&target) {
                targets.push(target);
            }
            if targets.len() == 32 {
                return targets;
            }
        }
    }
    targets
}

#[cfg(any(target_os = "windows", test))]
fn detection_threshold(base: u32, sensitivity: u8) -> u32 {
    (f64::from(base) * (1.5 - f64::from(sensitivity.clamp(1, 100)) / 100.0)).round() as u32
}

#[cfg(any(target_os = "windows", test))]
#[derive(Default)]
struct CenterHold {
    center: Option<(i32, i32)>,
    push_started: Option<std::time::Instant>,
    last_push: Option<(std::time::Instant, (i32, i32))>,
}

#[cfg(any(target_os = "windows", test))]
impl CenterHold {
    fn holds(
        &mut self,
        cursor: (i32, i32),
        movement: (i32, i32),
        targets: &[crate::placement::Rect],
        settings: MagneticCursorSettings,
        now: std::time::Instant,
    ) -> bool {
        use std::time::Duration;

        let strength = settings.center_hold.unwrap_or(settings.strength).min(100);
        let speed = f64::from(movement.0).hypot(f64::from(movement.1));
        let centered = targets.iter().any(|target| {
            target.valid() && cursor == (target.x + target.width / 2, target.y + target.height / 2)
        });
        if settings.strength == 0 || strength == 0 || speed == 0.0 || !centered {
            *self = Self::default();
            return false;
        }
        if self.center != Some(cursor) {
            *self = Self {
                center: Some(cursor),
                ..Self::default()
            };
        }
        if speed <= f64::from(strength) / 25.0 {
            self.push_started = None;
            self.last_push = None;
            return true;
        }
        if speed >= 4.0 + f64::from(strength) / 6.25 {
            *self = Self::default();
            return false;
        }
        // Only sustained input in a consistent direction counts toward release.
        let continuing = self.last_push.is_some_and(|(last, previous)| {
            let previous_speed = f64::from(previous.0).hypot(f64::from(previous.1));
            let alignment = f64::from(previous.0) * f64::from(movement.0)
                + f64::from(previous.1) * f64::from(movement.1);
            now.saturating_duration_since(last) <= Duration::from_millis(32)
                && alignment >= previous_speed * speed * 0.8
        });
        let started = if continuing {
            self.push_started.unwrap_or(now)
        } else {
            now
        };
        self.push_started = Some(started);
        self.last_push = Some((now, movement));
        now.saturating_duration_since(started) < Duration::from_millis(u64::from(strength) * 8 / 5)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::placement::Rect;

    const BUTTON: Rect = Rect {
        x: 100,
        y: 100,
        width: 20,
        height: 20,
    };

    fn assisted_motion(
        cursor: (i32, i32),
        movement: (i32, i32),
        targets: &[Rect],
        radius: f64,
        strength: u8,
    ) -> (i32, i32) {
        super::assisted_motion(
            cursor,
            movement,
            targets,
            radius,
            MagneticCursorSettings {
                strength,
                ..Default::default()
            },
            &mut None,
        )
    }

    #[test]
    fn small_outlines_keep_their_native_bounds_on_light_and_dark_backgrounds() {
        for (width, height) in [(4, 4), (5, 7), (9, 5), (15, 9), (31, 21)] {
            for (x, y) in [(11, 13), (12, 14)] {
                for (ink, background) in [(20, 220), (220, 20)] {
                    let target = Rect {
                        x,
                        y,
                        width,
                        height,
                    };
                    let gray = small_outline(target, ink, background);
                    assert_eq!(
                        detect_small_rectangles(&gray, 50),
                        vec![target],
                        "{target:?}, ink {ink}"
                    );
                }
            }
        }
    }

    #[test]
    fn native_detection_keeps_small_targets_inside_rectangles() {
        let outer = Rect {
            x: 301,
            y: 307,
            width: 40,
            height: 24,
        };
        let inner = Rect {
            x: 320,
            y: 314,
            width: 7,
            height: 5,
        };
        let mut gray = image::GrayImage::from_pixel(640, 640, image::Luma([220]));
        for rect in [outer, inner] {
            for y in rect.y..rect.y + rect.height {
                for x in rect.x..rect.x + rect.width {
                    if x == rect.x
                        || x == rect.x + rect.width - 1
                        || y == rect.y
                        || y == rect.y + rect.height - 1
                    {
                        gray.put_pixel(x as u32, y as u32, image::Luma([20]));
                    }
                }
            }
        }
        let targets = detect_small_rectangles(&gray, 50);
        assert_eq!(targets.len(), 2, "{targets:?}");
        assert!(targets.contains(&outer));
        assert!(targets.contains(&inner));
    }

    #[test]
    fn small_outline_detection_respects_sensitivity() {
        let target = Rect {
            x: 11,
            y: 13,
            width: 7,
            height: 9,
        };
        let gray = small_outline(target, 180, 200);
        assert!(detect_small_rectangles(&gray, 50).is_empty());
        assert_eq!(detect_small_rectangles(&gray, 100), vec![target]);
    }

    #[test]
    fn small_outline_detection_rejects_blank_images_dots_and_open_edges() {
        let target = Rect {
            x: 11,
            y: 13,
            width: 7,
            height: 9,
        };
        let mut gray = small_outline(target, 20, 220);
        for y in target.y..target.y + target.height {
            gray.put_pixel(
                (target.x + target.width - 1) as u32,
                y as u32,
                image::Luma([220]),
            );
        }
        assert!(detect_small_rectangles(&gray, 50).is_empty());
        let mut gray = image::GrayImage::from_pixel(64, 64, image::Luma([220]));
        assert!(detect_small_rectangles(&gray, 50).is_empty());
        gray.put_pixel(11, 13, image::Luma([20]));
        assert!(detect_small_rectangles(&gray, 50).is_empty());
    }

    fn small_outline(target: Rect, ink: u8, background: u8) -> image::GrayImage {
        let mut gray = image::GrayImage::from_pixel(64, 64, image::Luma([background]));
        for y in target.y..target.y + target.height {
            for x in target.x..target.x + target.width {
                if x == target.x
                    || x == target.x + target.width - 1
                    || y == target.y
                    || y == target.y + target.height - 1
                {
                    gray.put_pixel(x as u32, y as u32, image::Luma([ink]));
                }
            }
        }
        gray
    }

    #[test]
    fn beta_is_disabled_by_default_and_old_preferences_remain_valid() {
        let settings: MagneticCursorSettings = serde_json::from_str("{}").unwrap();
        assert!(!settings.enabled);
        assert_eq!(settings.strength, 50);
    }

    #[test]
    fn settings_round_trip_and_normalize_intensity() {
        let settings = MagneticCursorSettings {
            enabled: true,
            strength: 75,
            ..Default::default()
        };
        assert_eq!(
            serde_json::from_str::<MagneticCursorSettings>(
                &serde_json::to_string(&settings).unwrap()
            )
            .unwrap(),
            settings
        );
        assert_eq!(
            MagneticCursorSettings {
                strength: 0,
                ..settings
            }
            .normalized()
            .strength,
            1
        );
        assert_eq!(
            MagneticCursorSettings {
                strength: 255,
                ..settings
            }
            .normalized()
            .strength,
            100
        );
        let old: crate::control::ControlSettings =
            serde_json::from_str("{\"speed\":1200}").unwrap();
        assert!(!old.magnetic_cursor.enabled);
    }

    #[test]
    fn independent_controls_are_saved_and_legacy_hold_is_preserved() {
        let old: MagneticCursorSettings =
            serde_json::from_str(r#"{"enabled":true,"strength":75}"#).unwrap();
        let old = serde_json::to_value(old.normalized()).unwrap();
        assert_eq!(old["center_hold"], 75);
        assert_eq!(old["detection_sensitivity"], 50);

        let custom: MagneticCursorSettings = serde_json::from_str(
            r#"{"enabled":true,"strength":30,"center_hold":80,"detection_sensitivity":65}"#,
        )
        .unwrap();
        let custom = serde_json::to_value(custom.normalized()).unwrap();
        assert_eq!(custom["strength"], 30);
        assert_eq!(custom["center_hold"], 80);
        assert_eq!(custom["detection_sensitivity"], 65);
    }

    #[test]
    fn hold_can_be_tuned_without_changing_attraction() {
        let settings = MagneticCursorSettings {
            strength: 10,
            center_hold: Some(100),
            ..Default::default()
        };
        assert_eq!(
            super::assisted_motion((110, 110), (3, 0), &[BUTTON], 180.0, settings, &mut None),
            (0, 0)
        );
        let settings = MagneticCursorSettings {
            strength: 100,
            center_hold: Some(0),
            ..settings
        };
        assert_eq!(
            super::assisted_motion((110, 110), (1, 0), &[BUTTON], 180.0, settings, &mut None),
            (1, 0)
        );
        let approach =
            super::assisted_motion((90, 100), (8, 0), &[BUTTON], 180.0, settings, &mut None);
        assert_eq!(
            approach,
            super::assisted_motion(
                (90, 100),
                (8, 0),
                &[BUTTON],
                180.0,
                MagneticCursorSettings {
                    center_hold: Some(100),
                    ..settings
                },
                &mut None,
            )
        );
    }

    #[test]
    fn sensitivity_defaults_preserve_thresholds_and_higher_values_admit_fainter_edges() {
        for base in [10, 15, 18, 25] {
            assert_eq!(detection_threshold(base, 50), base);
            assert!(detection_threshold(base, 100) < base);
            assert!(detection_threshold(base, 1) > base);
        }
        let settings = MagneticCursorSettings {
            center_hold: Some(255),
            detection_sensitivity: 0,
            ..Default::default()
        }
        .normalized();
        assert_eq!(settings.center_hold, Some(100));
        assert_eq!(settings.detection_sensitivity, 1);
        let settings = MagneticCursorSettings {
            center_hold: Some(0),
            detection_sensitivity: 255,
            ..settings
        }
        .normalized();
        assert_eq!(settings.center_hold, Some(0));
        assert_eq!(settings.detection_sensitivity, 100);
    }

    #[test]
    fn a_brief_stick_flick_does_not_release_the_center() {
        let mut hold = CenterHold::default();
        let settings = MagneticCursorSettings::default();
        let now = std::time::Instant::now();
        assert!(hold.holds((110, 110), (6, 0), &[BUTTON], settings, now));
        assert!(hold.holds(
            (110, 110),
            (6, 0),
            &[BUTTON],
            settings,
            now + std::time::Duration::from_millis(8)
        ));
        // Releasing the stick between flicks must not accumulate escape pressure.
        assert!(hold.holds(
            (110, 110),
            (6, 0),
            &[BUTTON],
            settings,
            now + std::time::Duration::from_millis(100)
        ));
    }

    #[test]
    fn sustained_pressure_releases_at_the_same_time_at_different_poll_rates() {
        for interval in [4, 8, 16] {
            let mut hold = CenterHold::default();
            let now = std::time::Instant::now();
            for elapsed in (0..80).step_by(interval) {
                assert!(hold.holds(
                    (110, 110),
                    (6, 0),
                    &[BUTTON],
                    MagneticCursorSettings::default(),
                    now + std::time::Duration::from_millis(elapsed as u64)
                ));
            }
            assert!(!hold.holds(
                (110, 110),
                (6, 0),
                &[BUTTON],
                MagneticCursorSettings::default(),
                now + std::time::Duration::from_millis(80)
            ));
        }
    }

    #[test]
    fn strong_push_disabled_hold_and_missing_targets_release_immediately() {
        let now = std::time::Instant::now();
        let mut hold = CenterHold::default();
        let settings = MagneticCursorSettings::default();
        for movement in [(16, 0), (-16, 0), (0, 16), (0, -16)] {
            assert!(!hold.holds((110, 110), movement, &[BUTTON], settings, now));
        }
        assert!(!hold.holds((110, 110), (6, 0), &[], settings, now));
        assert!(!hold.holds((111, 110), (6, 0), &[BUTTON], settings, now));
        assert!(!hold.holds(
            (110, 110),
            (1, 0),
            &[BUTTON],
            MagneticCursorSettings {
                center_hold: Some(0),
                ..settings
            },
            now
        ));
    }

    #[test]
    fn reversing_direction_restarts_the_release_delay() {
        let mut hold = CenterHold::default();
        let now = std::time::Instant::now();
        for elapsed in (0..80).step_by(8) {
            assert!(hold.holds(
                (110, 110),
                (6, 0),
                &[BUTTON],
                MagneticCursorSettings::default(),
                now + std::time::Duration::from_millis(elapsed)
            ));
        }
        assert!(hold.holds(
            (110, 110),
            (-6, 0),
            &[BUTTON],
            MagneticCursorSettings::default(),
            now + std::time::Duration::from_millis(80)
        ));
    }

    #[test]
    fn center_hold_setting_controls_the_release_delay() {
        for strength in [25, 50, 100] {
            let mut hold = CenterHold::default();
            let now = std::time::Instant::now();
            let settings = MagneticCursorSettings {
                center_hold: Some(strength),
                ..Default::default()
            };
            let release_ms = u64::from(strength) * 8 / 5;
            for elapsed in (0..release_ms).step_by(8) {
                assert!(hold.holds(
                    (110, 110),
                    (6, 0),
                    &[BUTTON],
                    settings,
                    now + std::time::Duration::from_millis(elapsed)
                ));
            }
            assert!(!hold.holds(
                (110, 110),
                (6, 0),
                &[BUTTON],
                settings,
                now + std::time::Duration::from_millis(release_ms)
            ));
        }
    }

    #[test]
    fn target_selection_resists_small_variations_and_detection_reordering() {
        let lower = Rect { y: 122, ..BUTTON };
        let settings = MagneticCursorSettings::default();
        let mut selected = Some(BUTTON);
        let expected = assisted_motion((95, 122), (4, 0), &[BUTTON], 180.0, 50);
        let movement = super::assisted_motion(
            (95, 122),
            (4, 0),
            &[lower, BUTTON],
            180.0,
            settings,
            &mut selected,
        );
        assert_eq!(movement, expected);
        assert_eq!(selected, Some(BUTTON));
    }

    #[test]
    fn a_small_detection_shift_is_still_the_same_target() {
        let lower = Rect { y: 122, ..BUTTON };
        let shifted = Rect { x: 101, ..BUTTON };
        let mut selected = Some(BUTTON);
        let expected = assisted_motion((95, 122), (4, 0), &[shifted], 180.0, 50);
        assert_eq!(
            super::assisted_motion(
                (95, 122),
                (4, 0),
                &[lower, shifted],
                180.0,
                MagneticCursorSettings::default(),
                &mut selected
            ),
            expected
        );
        assert_eq!(selected, Some(shifted));
    }

    #[test]
    fn clear_direction_or_a_missing_target_allows_switching() {
        let lower = Rect { y: 122, ..BUTTON };
        for (cursor, movement, targets) in [
            ((95, 130), (0, 4), vec![BUTTON, lower]),
            ((95, 122), (4, 0), vec![lower]),
        ] {
            let mut selected = Some(BUTTON);
            let expected = assisted_motion(cursor, movement, &[lower], 180.0, 50);
            assert_eq!(
                super::assisted_motion(
                    cursor,
                    movement,
                    &targets,
                    180.0,
                    MagneticCursorSettings::default(),
                    &mut selected
                ),
                expected
            );
            assert_eq!(selected, Some(lower));
        }
    }

    #[test]
    fn leaving_the_target_or_losing_detection_clears_selection() {
        for (movement, targets) in [
            ((-4, 0), vec![BUTTON]),
            ((4, 0), vec![]),
            ((180, 0), vec![BUTTON]),
        ] {
            let mut selected = Some(BUTTON);
            assert_eq!(
                super::assisted_motion(
                    (95, 110),
                    movement,
                    &targets,
                    180.0,
                    MagneticCursorSettings::default(),
                    &mut selected
                ),
                movement
            );
            assert_eq!(selected, None);
        }
    }

    #[test]
    fn a_nearby_icon_can_replace_its_parent_rectangle() {
        let rectangle = Rect {
            width: 200,
            height: 60,
            ..BUTTON
        };
        let icon = Rect {
            x: 120,
            y: 120,
            width: 16,
            height: 16,
        };
        let mut selected = Some(rectangle);
        assert_eq!(
            super::assisted_motion(
                (117, 128),
                (4, 1),
                &[rectangle, icon],
                180.0,
                MagneticCursorSettings::default(),
                &mut selected
            ),
            (11, 0)
        );
        assert_eq!(selected, Some(icon));
    }

    #[test]
    fn nearby_nested_icon_wins_but_the_rectangle_wins_elsewhere() {
        let rectangle = Rect {
            x: 100,
            y: 100,
            width: 200,
            height: 60,
        };
        let icon = Rect {
            x: 120,
            y: 120,
            width: 16,
            height: 16,
        };
        for targets in [[rectangle, icon], [icon, rectangle]] {
            for cursor in [(117, 128), (125, 125)] {
                let movement = assisted_motion(cursor, (4, 1), &targets, 180.0, 100);
                assert_eq!((cursor.0 + movement.0, cursor.1 + movement.1), (128, 128));
            }
            assert_eq!(
                assisted_motion((195, 128), (4, 1), &targets, 180.0, 100),
                (5, 2)
            );
        }
        assert_eq!(
            assisted_motion((195, 128), (4, 1), &[rectangle], 180.0, 100),
            (5, 2)
        );
    }

    #[test]
    fn center_holds_against_small_stick_movements_but_releases_for_a_clear_push() {
        for movement in [(1, 0), (-1, 0), (0, 1), (0, -1), (1, 1), (2, 0)] {
            assert_eq!(
                assisted_motion((110, 110), movement, &[BUTTON], 180.0, 50),
                (0, 0),
                "movement {movement:?}"
            );
        }
        for movement in [(6, 0), (-6, 0), (0, 6), (0, -6), (4, 4)] {
            assert_eq!(
                assisted_motion((110, 110), movement, &[BUTTON], 180.0, 50),
                movement
            );
        }
        assert_eq!(
            assisted_motion((110, 110), (3, 0), &[BUTTON], 180.0, 100),
            (0, 0)
        );
        assert_eq!(
            assisted_motion((110, 110), (1, 0), &[BUTTON], 180.0, 0),
            (1, 0)
        );
    }

    #[test]
    fn one_axis_moving_away_does_not_cancel_the_other_axis_approaching() {
        let square = Rect {
            x: 100,
            y: 100,
            width: 6,
            height: 6,
        };
        for (cursor, movement) in [
            ((104, 101), (4, 1)),
            ((101, 104), (1, 4)),
            ((102, 105), (-4, -1)),
            ((105, 102), (-1, -4)),
        ] {
            let adjusted = assisted_motion(cursor, movement, &[square], 180.0, 50);
            assert_eq!(
                (cursor.0 + adjusted.0, cursor.1 + adjusted.1),
                (103, 103),
                "cursor {cursor:?}, movement {movement:?}"
            );
        }
        assert_eq!(
            assisted_motion((104, 101), (4, -1), &[square], 180.0, 50),
            (4, -1)
        );
    }

    #[test]
    fn tiny_squares_snap_to_a_reachable_center_pixel() {
        for size in 1..=8 {
            for origin in [-100, 100] {
                let square = Rect {
                    x: origin,
                    y: origin,
                    width: size,
                    height: size,
                };
                let cursor = (origin - 2, origin - 1);
                let movement = assisted_motion(cursor, (1, 0), &[square], 180.0, 50);
                assert_eq!(
                    (cursor.0 + movement.0, cursor.1 + movement.1),
                    (origin + size / 2, origin + size / 2),
                    "size {size}, origin {origin}"
                );
            }
        }
    }

    #[test]
    fn tiny_target_is_not_skipped_by_a_movement_larger_than_its_width() {
        let square = Rect {
            x: 100,
            y: 100,
            width: 3,
            height: 3,
        };
        assert_eq!(
            assisted_motion((95, 99), (12, 0), &[square], 180.0, 50),
            (6, 2)
        );
    }

    #[test]
    fn tiny_square_centers_when_motion_is_perpendicular_to_the_remaining_offset() {
        let square = Rect {
            x: 100,
            y: 100,
            width: 6,
            height: 6,
        };
        for (cursor, movement, expected) in
            [((103, 101), (1, 0), (0, 2)), ((101, 103), (0, 1), (2, 0))]
        {
            assert_eq!(
                assisted_motion(cursor, movement, &[square], 180.0, 50),
                expected
            );
        }
        assert_eq!(
            assisted_motion((103, 103), (1, 0), &[square], 180.0, 50),
            (0, 0)
        );
    }

    #[test]
    fn attraction_continues_inside_the_square_until_its_center() {
        for strength in [50, 100] {
            let mut cursor = (101, 104);
            for _ in 0..10 {
                if cursor == (110, 110) {
                    break;
                }
                let movement = assisted_motion(cursor, (1, 0), &[BUTTON], 48.0, strength);
                cursor = (cursor.0 + movement.0, cursor.1 + movement.1);
            }
            assert_eq!(cursor, (110, 110), "strength {strength}");
        }
    }

    #[test]
    fn movement_away_from_the_center_inside_the_square_is_unchanged() {
        assert_eq!(
            assisted_motion((101, 104), (-1, 0), &[BUTTON], 48.0, 100),
            (-1, 0)
        );
    }

    #[test]
    fn strong_assistance_reaches_the_center_of_a_nearby_button() {
        assert_eq!(
            assisted_motion((90, 100), (8, 0), &[BUTTON], 48.0, 100),
            (20, 10)
        );
    }

    #[test]
    fn slow_approach_still_has_noticeable_attraction() {
        let movement = assisted_motion((90, 100), (1, 0), &[BUTTON], 48.0, 100);
        assert!(movement.0 >= 8 && movement.1 >= 3, "{movement:?}");
    }

    #[test]
    fn steers_an_approaching_stick_toward_a_nearby_button() {
        let movement = assisted_motion((90, 100), (8, 0), &[BUTTON], 48.0, 100);
        assert!(movement.0 >= 8 && movement.1 > 0, "{movement:?}");
    }

    #[test]
    fn movement_away_from_a_button_and_a_stationary_cursor_are_unchanged() {
        for movement in [(-8, 0), (0, -8), (0, 0)] {
            assert_eq!(
                assisted_motion((90, 100), movement, &[BUTTON], 48.0, 100),
                movement
            );
        }
        assert_eq!(
            assisted_motion((110, 110), (8, 0), &[BUTTON], 48.0, 100),
            (8, 0)
        );
    }

    #[test]
    fn distant_targets_and_fast_escape_movements_are_ignored() {
        assert_eq!(
            assisted_motion((0, 0), (8, 0), &[BUTTON], 48.0, 100),
            (8, 0)
        );
        assert_eq!(
            assisted_motion((90, 100), (80, 0), &[BUTTON], 48.0, 100),
            (80, 0)
        );
    }

    #[test]
    fn strength_changes_assistance_without_reversing_the_requested_motion() {
        let weak = assisted_motion((90, 100), (10, 0), &[BUTTON], 48.0, 10);
        let strong = assisted_motion((90, 100), (10, 0), &[BUTTON], 48.0, 100);
        assert!(strong.1 > weak.1);
        assert!(strong.0 > 0);
        assert_eq!(
            assisted_motion((90, 100), (10, 0), &[BUTTON], 48.0, 0),
            (10, 0)
        );
    }
}

#[cfg(target_os = "windows")]
pub(crate) use native::MagneticCursor;

#[cfg(target_os = "windows")]
mod native {
    use super::*;
    use crate::placement::Rect;
    use std::{
        sync::{
            Arc, Mutex,
            atomic::{AtomicBool, Ordering},
        },
        thread::JoinHandle,
        time::{Duration, Instant},
    };
    use windows::Win32::{
        Foundation::{HWND, POINT, RECT},
        Graphics::Gdi::{
            BI_RGB, BITMAPINFO, BITMAPINFOHEADER, ClientToScreen, CreateCompatibleDC,
            CreateDIBSection, DIB_RGB_COLORS, DeleteDC, DeleteObject, GetDC, HALFTONE, HGDIOBJ,
            ReleaseDC, SRCCOPY, ScreenToClient, SelectObject, SetStretchBltMode, StretchBlt,
        },
        UI::{
            HiDpi::{
                DPI_AWARENESS_CONTEXT_PER_MONITOR_AWARE_V2, GetDpiForWindow,
                SetThreadDpiAwarenessContext,
            },
            Input::KeyboardAndMouse::{GetAsyncKeyState, VK_LBUTTON, VK_RBUTTON},
            WindowsAndMessaging::*,
        },
    };

    struct Snapshot {
        owner: isize,
        time: Instant,
        targets: Vec<Rect>,
    }
    struct Tracker {
        latest: Arc<Mutex<Option<Snapshot>>>,
        diagnostics: Arc<Mutex<MagneticCursorDiagnostics>>,
        stop: Arc<AtomicBool>,
        worker: Option<JoinHandle<()>>,
    }
    impl Drop for Tracker {
        fn drop(&mut self) {
            self.stop.store(true, Ordering::Relaxed);
            if let Some(worker) = self.worker.take() {
                let _ = worker.join();
            }
        }
    }
    #[derive(Default)]
    pub(crate) struct MagneticCursor {
        settings: MagneticCursorSettings,
        tracker: Option<Tracker>,
        center_hold: CenterHold,
        target: Option<Rect>,
    }
    impl MagneticCursor {
        pub(crate) fn configure(&mut self, settings: MagneticCursorSettings, active: bool) {
            let settings = settings.normalized();
            if self.settings != settings || !active || !settings.enabled {
                self.center_hold = CenterHold::default();
                self.target = None;
            }
            if self.settings.detection_sensitivity != settings.detection_sensitivity {
                self.tracker = None;
            }
            self.settings = settings;
            if !active || !settings.enabled {
                self.tracker = None;
                return;
            }
            if self.tracker.is_some() {
                return;
            }
            let latest = Arc::new(Mutex::new(None));
            let diagnostics = Arc::new(Mutex::new(MagneticCursorDiagnostics {
                tracking: true,
                ..Default::default()
            }));
            let stop = Arc::new(AtomicBool::new(false));
            let (value, stopping) = (Arc::clone(&latest), Arc::clone(&stop));
            let scanner_status = Arc::clone(&diagnostics);
            let worker = std::thread::spawn(move || unsafe {
                let previous =
                    SetThreadDpiAwarenessContext(DPI_AWARENESS_CONTEXT_PER_MONITOR_AWARE_V2);
                while !stopping.load(Ordering::Relaxed) {
                    match scan(settings.detection_sensitivity) {
                        Ok(snapshot) => {
                            let mut status =
                                scanner_status.lock().expect("magnetic cursor diagnostics");
                            status.scanner_ready = true;
                            status.scanner_failed = false;
                            status.visual_candidates =
                                snapshot.as_ref().map_or(0, |s| s.targets.len());
                            drop(status);
                            *value.lock().expect("magnetic cursor targets") = snapshot;
                        }
                        Err(_) => {
                            let mut status =
                                scanner_status.lock().expect("magnetic cursor diagnostics");
                            status.scanner_ready = false;
                            status.scanner_failed = true;
                            status.visual_candidates = 0;
                            drop(status);
                            *value.lock().expect("magnetic cursor targets") = None;
                        }
                    }
                    std::thread::sleep(Duration::from_millis(200));
                }
                SetThreadDpiAwarenessContext(previous);
            });
            self.tracker = Some(Tracker {
                latest,
                diagnostics,
                stop,
                worker: Some(worker),
            });
        }
        pub(crate) fn diagnostics(&self) -> MagneticCursorDiagnostics {
            self.tracker
                .as_ref()
                .map_or_else(Default::default, |tracker| {
                    *tracker
                        .diagnostics
                        .lock()
                        .expect("magnetic cursor diagnostics")
                })
        }
        pub(crate) fn adjust(&mut self, movement: (i32, i32)) -> (i32, i32) {
            let Some(tracker) = &self.tracker else {
                self.center_hold = CenterHold::default();
                self.target = None;
                return movement;
            };
            tracker
                .diagnostics
                .lock()
                .expect("magnetic cursor diagnostics")
                .correction_px = 0;
            let latest = tracker.latest.lock().expect("magnetic cursor targets");
            let Some(snapshot) = latest
                .as_ref()
                .filter(|s| s.time.elapsed() < Duration::from_millis(600))
            else {
                self.center_hold = CenterHold::default();
                self.target = None;
                return movement;
            };
            unsafe {
                let mut point = POINT::default();
                if GetPhysicalCursorPos(&mut point).is_err() {
                    self.center_hold = CenterHold::default();
                    self.target = None;
                    return movement;
                }
                let owner = GetAncestor(WindowFromPhysicalPoint(point), GA_ROOT);
                if owner.is_invalid()
                    || owner.0 as isize != snapshot.owner
                    || GetAsyncKeyState(i32::from(VK_LBUTTON.0)) < 0
                    || GetAsyncKeyState(i32::from(VK_RBUTTON.0)) < 0
                {
                    self.center_hold = CenterHold::default();
                    self.target = None;
                    return movement;
                }
                let dpi = GetDpiForWindow(owner).max(96);
                let adjusted = if self.center_hold.holds(
                    (point.x, point.y),
                    movement,
                    &snapshot.targets,
                    self.settings,
                    Instant::now(),
                ) {
                    self.target = snapshot.targets.iter().copied().find(|target| {
                        target.valid()
                            && (point.x, point.y)
                                == (target.x + target.width / 2, target.y + target.height / 2)
                    });
                    (0, 0)
                } else {
                    assisted_motion(
                        (point.x, point.y),
                        movement,
                        &snapshot.targets,
                        180.0 * f64::from(dpi) / 96.0,
                        self.settings,
                        &mut self.target,
                    )
                };
                let correction = f64::from(adjusted.0 - movement.0)
                    .hypot(f64::from(adjusted.1 - movement.1))
                    .round() as u16;
                tracker
                    .diagnostics
                    .lock()
                    .expect("magnetic cursor diagnostics")
                    .correction_px = correction;
                adjusted
            }
        }
    }
    const CAPTURE_SIZE: i32 = 640;
    const CAPTURE_SCALE: i32 = 1;
    const MAX_CANDIDATES: usize = 64;
    const MAX_RAW_CANDIDATES: usize = 512;

    struct Capture {
        pixels: Vec<u8>,
        width: usize,
        height: usize,
        screen_origin: POINT,
    }

    fn scan(sensitivity: u8) -> windows::core::Result<Option<Snapshot>> {
        unsafe {
            let mut cursor = POINT::default();
            GetPhysicalCursorPos(&mut cursor)?;
            let owner = GetAncestor(WindowFromPhysicalPoint(cursor), GA_ROOT);
            if owner.is_invalid() {
                return Ok(None);
            }
            let capture = capture_region(owner, cursor)?;
            let targets = detect_rectangles(
                &capture.pixels,
                capture.width,
                capture.height,
                capture.screen_origin,
                sensitivity,
            );
            let mut current = POINT::default();
            GetPhysicalCursorPos(&mut current)?;
            Ok(
                (GetAncestor(WindowFromPhysicalPoint(current), GA_ROOT) == owner).then_some(
                    Snapshot {
                        owner: owner.0 as isize,
                        time: Instant::now(),
                        targets,
                    },
                ),
            )
        }
    }

    unsafe fn capture_region(hwnd: HWND, mut cursor: POINT) -> windows::core::Result<Capture> {
        let mut client = RECT::default();
        unsafe { GetClientRect(hwnd, &mut client)? };
        if client.right <= client.left || client.bottom <= client.top {
            return Err(windows::core::Error::from_thread());
        }
        if !unsafe { ScreenToClient(hwnd, &mut cursor) }.as_bool() {
            return Err(windows::core::Error::from_thread());
        }
        let client_width = client.right - client.left;
        let client_height = client.bottom - client.top;
        let source_x = (cursor.x - CAPTURE_SIZE / 2).clamp(0, (client_width - CAPTURE_SIZE).max(0));
        let source_y =
            (cursor.y - CAPTURE_SIZE / 2).clamp(0, (client_height - CAPTURE_SIZE).max(0));
        let source_width = CAPTURE_SIZE.min(client_width);
        let source_height = CAPTURE_SIZE.min(client_height);
        let mut screen_origin = POINT {
            x: source_x,
            y: source_y,
        };
        if !unsafe { ClientToScreen(hwnd, &mut screen_origin) }.as_bool() {
            return Err(windows::core::Error::from_thread());
        }
        let width = (source_width / CAPTURE_SCALE).max(1) as usize;
        let height = (source_height / CAPTURE_SCALE).max(1) as usize;

        // Capture the composed desktop image; window DCs can be blank for GPU-rendered apps.
        let source_dc = unsafe { GetDC(None) };
        if source_dc.0.is_null() {
            return Err(windows::core::Error::from_thread());
        }
        let memory_dc = unsafe { CreateCompatibleDC(Some(source_dc)) };
        if memory_dc.0.is_null() {
            unsafe { ReleaseDC(None, source_dc) };
            return Err(windows::core::Error::from_thread());
        }

        let info = BITMAPINFO {
            bmiHeader: BITMAPINFOHEADER {
                biSize: std::mem::size_of::<BITMAPINFOHEADER>() as u32,
                biWidth: width as i32,
                biHeight: -(height as i32),
                biPlanes: 1,
                biBitCount: 32,
                biCompression: BI_RGB.0,
                ..Default::default()
            },
            ..Default::default()
        };
        let mut bits = std::ptr::null_mut();
        let bitmap = match unsafe {
            CreateDIBSection(Some(source_dc), &info, DIB_RGB_COLORS, &mut bits, None, 0)
        } {
            Ok(bitmap) => bitmap,
            Err(error) => {
                unsafe {
                    let _ = DeleteDC(memory_dc);
                    ReleaseDC(None, source_dc);
                }
                return Err(error);
            }
        };
        let previous = unsafe { SelectObject(memory_dc, HGDIOBJ(bitmap.0)) };
        if previous.0.is_null() || previous.0 as isize == -1 {
            unsafe {
                let _ = DeleteObject(HGDIOBJ(bitmap.0));
                let _ = DeleteDC(memory_dc);
                ReleaseDC(None, source_dc);
            }
            return Err(windows::core::Error::from_thread());
        }

        let result = (|| {
            unsafe { SetStretchBltMode(memory_dc, HALFTONE) };
            if !unsafe {
                StretchBlt(
                    memory_dc,
                    0,
                    0,
                    width as i32,
                    height as i32,
                    Some(source_dc),
                    screen_origin.x,
                    screen_origin.y,
                    source_width,
                    source_height,
                    SRCCOPY,
                )
            }
            .as_bool()
            {
                return Err(windows::core::Error::from_thread());
            }
            let byte_count = width * height * 4;
            let pixels = unsafe { std::slice::from_raw_parts(bits.cast::<u8>(), byte_count) };
            Ok(pixels.to_vec())
        })();

        unsafe {
            SelectObject(memory_dc, previous);
            let _ = DeleteObject(HGDIOBJ(bitmap.0));
            let _ = DeleteDC(memory_dc);
            ReleaseDC(None, source_dc);
        }
        result.map(|pixels| Capture {
            pixels,
            width,
            height,
            screen_origin,
        })
    }

    #[derive(Clone, Copy, Debug, PartialEq, Eq)]
    struct Candidate {
        score: u16,
        area: u32,
        bounds: Rect,
    }

    impl PartialOrd for Candidate {
        fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
            Some(self.cmp(other))
        }
    }

    impl Ord for Candidate {
        fn cmp(&self, other: &Self) -> std::cmp::Ordering {
            self.score
                .cmp(&other.score)
                .then_with(|| other.area.cmp(&self.area))
                .then_with(|| self.bounds.x.cmp(&other.bounds.x))
                .then_with(|| self.bounds.y.cmp(&other.bounds.y))
        }
    }

    fn detect_rectangles(
        pixels: &[u8],
        width: usize,
        height: usize,
        origin: POINT,
        sensitivity: u8,
    ) -> Vec<Rect> {
        let gray = image::GrayImage::from_fn(width as u32, height as u32, |x, y| {
            let i = (y as usize * width + x as usize) * 4;
            image::Luma([((u32::from(pixels[i + 2]) * 77
                + u32::from(pixels[i + 1]) * 150
                + u32::from(pixels[i]) * 29)
                >> 8) as u8])
        });
        let binary = imageproc::contrast::adaptive_threshold(&gray, 7, 8);
        let contours = imageproc::contours::find_contours::<i32>(&binary);
        let mut icons = Vec::new();
        for contour in contours {
            if contour.points.len() < 8 {
                continue;
            }
            let left = contour.points.iter().map(|p| p.x).min().unwrap();
            let right = contour.points.iter().map(|p| p.x).max().unwrap();
            let top = contour.points.iter().map(|p| p.y).min().unwrap();
            let bottom = contour.points.iter().map(|p| p.y).max().unwrap();
            let (w, h) = (right - left + 1, bottom - top + 1);
            if !(6..=40).contains(&w) || !(6..=40).contains(&h) || (w - h).abs() > w.max(h) / 3 {
                continue;
            }
            let sample = |x: i32, y: i32| {
                f64::from(
                    gray.get_pixel(
                        x.clamp(0, width as i32 - 1) as u32,
                        y.clamp(0, height as i32 - 1) as u32,
                    )[0],
                )
            };
            // Side midpoints lie between the arms of an X, unlike its corners.
            let background = (sample(left - 2, (top + bottom) / 2)
                + sample(right + 2, (top + bottom) / 2)
                + sample((left + right) / 2, top - 2)
                + sample((left + right) / 2, bottom + 2))
                / 4.0;
            let mut diagonal = 0.0;
            let mut empty = 0.0;
            for step in 0..=8 {
                let x = left + (w - 1) * step / 8;
                let y1 = top + (h - 1) * step / 8;
                let y2 = bottom - (h - 1) * step / 8;
                for y in [y1, y2] {
                    diagonal += (-1..=1)
                        .map(|dy| (sample(x, y + dy) - background).abs())
                        .fold(0.0, f64::max);
                }
            }
            for (fx, fy) in [(0.5, 0.12), (0.5, 0.88), (0.12, 0.5), (0.88, 0.5)] {
                empty += (sample(
                    left + (f64::from(w - 1) * fx) as i32,
                    top + (f64::from(h - 1) * fy) as i32,
                ) - background)
                    .abs();
            }
            if diagonal / 18.0 < f64::from(detection_threshold(25, sensitivity))
                || empty / 4.0 > diagonal / 18.0 * 0.45
            {
                continue;
            }
            // Nearby glyphs on the same baseline indicate a word, rather than an isolated icon.
            let mut neighbors = 0;
            for y in top..=bottom {
                for dx in 2..=w.clamp(6, 18) {
                    for x in [left - dx, right + dx] {
                        if (sample(x, y) - background).abs() > 35.0 {
                            neighbors += 1;
                        }
                    }
                }
            }
            if neighbors > h {
                continue;
            }
            let target = Rect {
                x: origin.x + left,
                y: origin.y + top,
                width: w,
                height: h,
            };
            if icons
                .iter()
                .all(|existing| overlap_ratio(*existing, target) < 0.3)
            {
                icons.push(target);
            }
            if icons.len() == 32 {
                break;
            }
        }
        for target in detect_small_rectangles(&gray, sensitivity) {
            if icons.len() >= MAX_CANDIDATES {
                break;
            }
            let target = Rect {
                x: origin.x + target.x,
                y: origin.y + target.y,
                ..target
            };
            if icons
                .iter()
                .all(|existing| overlap_ratio(*existing, target) < 0.7)
            {
                icons.push(target);
            }
        }
        // Keep the broad rectangle pass cheap; small targets were checked at native resolution.
        let small = image::imageops::resize(
            &gray,
            (width / 2).max(1) as u32,
            (height / 2).max(1) as u32,
            image::imageops::FilterType::Triangle,
        );
        let rgba: Vec<u8> = small
            .pixels()
            .flat_map(|p| [p[0], p[0], p[0], 255])
            .collect();
        let coarse = detect_rectangles_coarse(
            &rgba,
            small.width() as usize,
            small.height() as usize,
            POINT::default(),
            sensitivity,
        );
        for rect in coarse {
            if icons.len() >= MAX_CANDIDATES {
                break;
            }
            let rect = Rect {
                x: origin.x + rect.x * 2,
                y: origin.y + rect.y * 2,
                width: rect.width * 2,
                height: rect.height * 2,
            };
            if !continuous_border(
                &gray,
                rect.x - origin.x,
                rect.y - origin.y,
                rect.width,
                rect.height,
                sensitivity,
            ) {
                continue;
            }
            if icons
                .iter()
                .all(|existing| overlap_ratio(*existing, rect) < 0.2)
            {
                icons.push(rect);
            }
            if icons.len() == MAX_CANDIDATES {
                break;
            }
        }
        let mut annotated = pixels.to_vec();
        for rect in &icons {
            let left = (rect.x - origin.x).max(0) as usize;
            let top = (rect.y - origin.y).max(0) as usize;
            let right = (left + rect.width as usize).min(width - 1);
            let bottom = (top + rect.height as usize).min(height - 1);
            for y in top..=bottom {
                for x in left..=right {
                    if x == left || x == right || y == top || y == bottom {
                        let i = (y * width + x) * 4;
                        annotated[i..i + 4].copy_from_slice(&[0, 255, 0, 255]);
                    }
                }
            }
        }
        if let Ok(mut preview) = PREVIEW.lock() {
            *preview = Some(Arc::new(VisualPreview {
                pixels: annotated,
                width,
                height,
            }));
        }
        icons
    }

    fn continuous_border(
        gray: &image::GrayImage,
        x: i32,
        y: i32,
        w: i32,
        h: i32,
        sensitivity: u8,
    ) -> bool {
        let threshold = detection_threshold(15, sensitivity) as u8;
        let sample = |x: i32, y: i32| {
            gray.get_pixel(
                x.clamp(0, gray.width() as i32 - 1) as u32,
                y.clamp(0, gray.height() as i32 - 1) as u32,
            )[0]
        };
        // Text has scattered edges; a control border must continue along every side.
        let horizontal = |row: i32| {
            (2..w - 2)
                .filter(|dx| {
                    (-2..=2).any(|offset| {
                        sample(x + dx, row + offset - 1).abs_diff(sample(x + dx, row + offset + 1))
                            >= threshold
                    })
                })
                .count() as f64
                / f64::from((w - 4).max(1))
        };
        let vertical = |column: i32| {
            (2..h - 2)
                .filter(|dy| {
                    (-2..=2).any(|offset| {
                        sample(column + offset - 1, y + dy)
                            .abs_diff(sample(column + offset + 1, y + dy))
                            >= threshold
                    })
                })
                .count() as f64
                / f64::from((h - 4).max(1))
        };
        horizontal(y) >= 0.85
            && horizontal(y + h) >= 0.85
            && vertical(x) >= 0.85
            && vertical(x + w) >= 0.85
    }

    fn detect_rectangles_coarse(
        pixels: &[u8],
        width: usize,
        height: usize,
        screen_origin: POINT,
        sensitivity: u8,
    ) -> Vec<Rect> {
        use std::cmp::Reverse;
        use std::collections::BinaryHeap;

        if width < 24 || height < 24 || pixels.len() < width * height * 4 {
            return Vec::new();
        }
        let mut gray = vec![0u8; width * height];
        for (index, pixel) in pixels.chunks_exact(4).take(gray.len()).enumerate() {
            gray[index] =
                ((u32::from(pixel[2]) * 77 + u32::from(pixel[1]) * 150 + u32::from(pixel[0]) * 29)
                    >> 8) as u8;
        }

        let mut horizontal = vec![0u32; height * (width + 1)];
        let mut vertical = vec![0u32; width * (height + 1)];
        for y in 0..height {
            for x in 0..width {
                let current = gray[y * width + x];
                let horizontal_edge = if y > 0 {
                    current.abs_diff(gray[(y - 1) * width + x])
                } else {
                    0
                };
                let vertical_edge = if x > 0 {
                    current.abs_diff(gray[y * width + x - 1])
                } else {
                    0
                };
                horizontal[y * (width + 1) + x + 1] =
                    horizontal[y * (width + 1) + x] + u32::from(horizontal_edge);
                vertical[x * (height + 1) + y + 1] =
                    vertical[x * (height + 1) + y] + u32::from(vertical_edge);
            }
        }

        let weakest_threshold = detection_threshold(10, sensitivity);
        let average_threshold = detection_threshold(18, sensitivity);
        let widths = [8, 10, 12, 16, 20, 24, 32, 40, 48, 64, 80, 96, 128, 160, 192];
        let heights = [8, 10, 12, 16, 20, 24, 32, 40, 48, 64, 80];
        let mut strongest = BinaryHeap::<Reverse<Candidate>>::new();
        for box_width in widths.into_iter().filter(|value| *value < width - 2) {
            for box_height in heights.into_iter().filter(|value| *value < height - 2) {
                let ratio = box_width as f32 / box_height as f32;
                if !(0.55..=5.5).contains(&ratio) {
                    continue;
                }
                for y in (1..height - box_height - 1).step_by(6) {
                    for x in (1..width - box_width - 1).step_by(6) {
                        let top = horizontal_line(&horizontal, width, height, x, y, box_width);
                        let bottom = horizontal_line(
                            &horizontal,
                            width,
                            height,
                            x,
                            y + box_height,
                            box_width,
                        );
                        let left = vertical_line(&vertical, width, height, x, y, box_height);
                        let right =
                            vertical_line(&vertical, width, height, x + box_width, y, box_height);
                        let weakest = top.min(bottom).min(left).min(right);
                        let average = (top + bottom + left + right) / 4;
                        if weakest < weakest_threshold || average < average_threshold {
                            continue;
                        }
                        let bounds = Rect {
                            x: screen_origin.x + x as i32 * CAPTURE_SCALE,
                            y: screen_origin.y + y as i32 * CAPTURE_SCALE,
                            width: box_width as i32 * CAPTURE_SCALE,
                            height: box_height as i32 * CAPTURE_SCALE,
                        };
                        let candidate = Candidate {
                            score: (average * 2 + weakest) as u16,
                            area: (box_width * box_height) as u32,
                            bounds,
                        };
                        if strongest.len() < MAX_RAW_CANDIDATES {
                            strongest.push(Reverse(candidate));
                        } else if strongest.peek().is_some_and(|worst| candidate > worst.0) {
                            strongest.pop();
                            strongest.push(Reverse(candidate));
                        }
                    }
                }
            }
        }

        let mut candidates: Vec<_> = strongest.into_iter().map(|Reverse(item)| item).collect();
        candidates.sort_unstable_by(|a, b| b.cmp(a));
        let mut targets = Vec::with_capacity(MAX_CANDIDATES);
        for candidate in candidates {
            if targets.len() == MAX_CANDIDATES {
                break;
            }
            if targets
                .iter()
                .all(|existing| overlap_ratio(*existing, candidate.bounds) < 0.55)
            {
                targets.push(candidate.bounds);
            }
        }
        targets
    }

    fn horizontal_line(
        prefix: &[u32],
        width: usize,
        height: usize,
        x: usize,
        y: usize,
        len: usize,
    ) -> u32 {
        (-1..=1)
            .map(|offset| y as isize + offset)
            .filter(|row| *row >= 0 && *row < height as isize)
            .map(|row| {
                let start = row as usize * (width + 1) + x;
                (prefix[start + len] - prefix[start]) / len as u32
            })
            .max()
            .unwrap_or(0)
    }

    fn vertical_line(
        prefix: &[u32],
        width: usize,
        height: usize,
        x: usize,
        y: usize,
        len: usize,
    ) -> u32 {
        (-1..=1)
            .map(|offset| x as isize + offset)
            .filter(|column| *column >= 0 && *column < width as isize)
            .map(|column| {
                let start = column as usize * (height + 1) + y;
                (prefix[start + len] - prefix[start]) / len as u32
            })
            .max()
            .unwrap_or(0)
    }

    fn overlap_ratio(a: Rect, b: Rect) -> f32 {
        let left = a.x.max(b.x);
        let top = a.y.max(b.y);
        let right = (a.x + a.width).min(b.x + b.width);
        let bottom = (a.y + a.height).min(b.y + b.height);
        let intersection = (right - left).max(0) as f32 * (bottom - top).max(0) as f32;
        let union = (a.width * a.height + b.width * b.height) as f32 - intersection;
        if union <= 0.0 {
            0.0
        } else {
            intersection / union
        }
    }
}
