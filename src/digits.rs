use egui::{Color32, Painter, Rect, Shape, Stroke, Vec2};

// A B C D E F G: polygonal seven-segment digits, used by the live timer only.
const MASKS: [u8; 10] = [0x3f, 0x06, 0x5b, 0x4f, 0x66, 0x6d, 0x7d, 0x07, 0x7f, 0x6f];

pub fn paint(p: &Painter, rect: Rect, value: &str, color: Color32) {
    let units: f32 = value
        .chars()
        .map(|c| if c == '.' { 20.0 } else { 66.0 })
        .sum::<f32>()
        - 8.0;
    let scale = (rect.height() / 100.0).min(rect.width() / units);
    let origin = rect.center() - Vec2::new(units * scale, 100.0 * scale) * 0.5;
    let mut offset = 0.0;
    let polygon = |vertices: &[(f32, f32)], x: f32| {
        let points = vertices
            .iter()
            .map(|(a, b)| origin + Vec2::new(a + x, *b) * scale)
            .collect();
        p.add(Shape::convex_polygon(points, color, Stroke::NONE));
    };
    for ch in value.chars() {
        if ch == '.' {
            polygon(
                &[(0.0, 94.0), (6.0, 88.0), (12.0, 94.0), (6.0, 100.0)],
                offset,
            );
            offset += 20.0;
            continue;
        }
        let Some(digit) = ch.to_digit(10) else {
            continue;
        };
        let mask = MASKS[digit as usize];
        for (bit, y) in [(0, 0.0), (6, 45.0), (3, 90.0)] {
            if mask & (1 << bit) != 0 {
                polygon(
                    &[
                        (10.0, y),
                        (48.0, y),
                        (54.0, y + 5.0),
                        (48.0, y + 10.0),
                        (10.0, y + 10.0),
                        (4.0, y + 5.0),
                    ],
                    offset,
                );
            }
        }
        for (bit, x, y) in [
            (5, 0.0, 5.0),
            (1, 48.0, 5.0),
            (4, 0.0, 50.0),
            (2, 48.0, 50.0),
        ] {
            if mask & (1 << bit) != 0 {
                polygon(
                    &[
                        (x + 5.0, y),
                        (x + 10.0, y + 5.0),
                        (x + 10.0, y + 40.0),
                        (x + 5.0, y + 45.0),
                        (x, y + 40.0),
                        (x, y + 5.0),
                    ],
                    offset,
                );
            }
        }
        offset += 66.0;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn all_digits_have_distinct_segment_patterns() {
        let unique: std::collections::HashSet<_> = MASKS.iter().collect();
        assert_eq!(unique.len(), 10);
        assert_eq!(MASKS[8].count_ones(), 7);
        assert_eq!(MASKS[1].count_ones(), 2);
    }
}
