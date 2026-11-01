//! Screen coordinates supplied by native accessibility APIs.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Rect {
    pub x: i32,
    pub y: i32,
    pub width: i32,
    pub height: i32,
}

impl Rect {
    pub fn valid(self) -> bool {
        self.width > 0
            && self.height > 0
            && self.width < 131_072
            && self.height < 131_072
            && self.x.unsigned_abs() < 1_000_000
            && self.y.unsigned_abs() < 1_000_000
    }

    pub fn contains_center(self, other: Self) -> bool {
        other.valid()
            && other.x + other.width / 2 >= self.x
            && other.x + other.width / 2 < self.x + self.width
            && other.y + other.height / 2 >= self.y
            && other.y + other.height / 2 < self.y + self.height
    }

    pub fn intersection(self, other: Self) -> Option<Self> {
        if !self.valid() || !other.valid() {
            return None;
        }
        let x = self.x.max(other.x);
        let y = self.y.max(other.y);
        let right = (self.x + self.width).min(other.x + other.width);
        let bottom = (self.y + self.height).min(other.y + other.height);
        (right > x && bottom > y).then_some(Self {
            x,
            y,
            width: right - x,
            height: bottom - y,
        })
    }
}

pub fn keyboard_rect(screen: Rect, input: Option<Rect>, size: (u32, u32), top: bool) -> Rect {
    let margin = 20.min(screen.width / 4).min(screen.height / 4).max(0);
    let scale = ((screen.width - margin * 2) as f64 / size.0.max(1) as f64)
        .min((screen.height - margin * 2) as f64 / size.1.max(1) as f64)
        .min(1.0);
    let width = (size.0 as f64 * scale).max(1.0) as i32;
    let height = (size.1 as f64 * scale).max(1.0) as i32;
    let left = screen.x + margin;
    let right = screen.x + screen.width - margin - width;
    let upper = screen.y + margin;
    let lower = screen.y + screen.height - margin - height;
    let mut result = Rect {
        x: screen.x + (screen.width - width) / 2,
        y: if top { upper } else { lower },
        width,
        height,
    };
    if let Some(input) = input.and_then(|input| screen.intersection(input)) {
        result.x = (input.x + (input.width - width) / 2).clamp(left, right.max(left));
        result.y = (input.y - height - 12).clamp(upper, lower.max(upper));
        if input.y - height - 12 >= upper {
            result.y = input.y - height - 12;
        } else if input.y + input.height + 12 <= lower {
            result.y = input.y + input.height + 12;
        } else if input.x + input.width + 12 <= right {
            result.x = input.x + input.width + 12;
        } else if input.x - width - 12 >= left {
            result.x = input.x - width - 12;
        } else {
            // A full-screen editor may leave no non-overlapping placement.
            // Choose the edge with the smaller overlap, keeping all keys reachable.
            result.y = if input.y - upper > screen.y + screen.height - input.y - input.height {
                upper
            } else {
                lower
            };
        }
    }
    result.x = result.x.clamp(left, right.max(left));
    result.y = result.y.clamp(upper, lower.max(upper));
    result
}

#[cfg(test)]
mod tests {
    use super::*;
    const SCREEN: Rect = Rect {
        x: 0,
        y: 0,
        width: 1920,
        height: 1080,
    };

    #[test]
    fn follows_input_above_with_gap_instead_of_screen_center() {
        let input = Rect {
            x: 100,
            y: 600,
            width: 300,
            height: 40,
        };
        let result = keyboard_rect(SCREEN, Some(input), (640, 229), false);
        assert_eq!(result.x, 20);
        assert_eq!(result.y + result.height + 12, input.y);
    }

    #[test]
    fn uses_below_when_there_is_no_room_above() {
        let input = Rect {
            x: 1100,
            y: 80,
            width: 400,
            height: 40,
        };
        let result = keyboard_rect(SCREEN, Some(input), (640, 229), false);
        assert_eq!(result.y, 132);
        assert_eq!(result.x, 980);
    }

    #[test]
    fn clamps_to_negative_origin_monitor() {
        let screen = Rect { x: -1920, ..SCREEN };
        let input = Rect {
            x: -200,
            y: 800,
            width: 100,
            height: 40,
        };
        let result = keyboard_rect(screen, Some(input), (640, 229), false);
        assert_eq!(result.x, -660);
        assert_eq!(result.y, 559);
    }

    #[test]
    fn moves_beside_tall_input_instead_of_covering_it() {
        let input = Rect {
            x: 50,
            y: 50,
            width: 500,
            height: 950,
        };
        let result = keyboard_rect(SCREEN, Some(input), (640, 229), false);
        assert!(result.x >= input.x + input.width + 12);
    }

    #[test]
    fn missing_or_invalid_bounds_use_safe_fallback() {
        for input in [
            None,
            Some(Rect {
                x: i32::MIN,
                y: 0,
                width: -1,
                height: 0,
            }),
        ] {
            assert_eq!(keyboard_rect(SCREEN, input, (640, 229), false).y, 831);
        }
    }

    #[test]
    fn small_screen_constrains_keyboard_size() {
        let screen = Rect {
            width: 500,
            height: 200,
            ..SCREEN
        };
        let result = keyboard_rect(screen, None, (640, 229), false);
        assert!(result.x >= 0 && result.y >= 0);
        assert!(result.width <= 500 && result.height <= 200);
    }

    #[test]
    fn follows_the_visible_part_of_an_input_whose_center_is_off_screen() {
        let input = Rect {
            x: -500,
            y: 200,
            width: 600,
            height: 40,
        };
        let result = keyboard_rect(SCREEN, Some(input), (640, 229), false);
        assert_eq!(result.x, 20);
        assert_eq!(result.y, input.y + input.height + 12);
    }

    #[test]
    fn keeps_the_keyboard_and_its_margin_inside_each_screen_edge() {
        for screen in [
            SCREEN,
            Rect {
                x: -1920,
                y: -1080,
                ..SCREEN
            },
            Rect {
                width: 500,
                height: 200,
                ..SCREEN
            },
        ] {
            for x in [
                screen.x,
                screen.x + screen.width / 2,
                screen.x + screen.width - 1,
            ] {
                for y in [
                    screen.y,
                    screen.y + screen.height / 2,
                    screen.y + screen.height - 1,
                ] {
                    let input = Rect {
                        x,
                        y,
                        width: 1,
                        height: 1,
                    };
                    let result = keyboard_rect(screen, Some(input), (640, 229), false);
                    assert!(
                        result.x >= screen.x + 20 && result.y >= screen.y + 20,
                        "{screen:?}: {result:?}"
                    );
                    assert!(
                        result.x + result.width <= screen.x + screen.width - 20,
                        "{screen:?}: {result:?}"
                    );
                    assert!(
                        result.y + result.height <= screen.y + screen.height - 20,
                        "{screen:?}: {result:?}"
                    );
                }
            }
        }
    }
}
