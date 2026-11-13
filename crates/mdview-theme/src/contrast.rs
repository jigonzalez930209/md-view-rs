//! WCAG 2.2 contrast helpers.

use egui::Color32;

/// Relative luminance of an sRGB color.
pub fn relative_luminance(color: Color32) -> f32 {
    let channel = |value: u8| {
        let value = value as f32 / 255.0;
        if value <= 0.040_45 {
            value / 12.92
        } else {
            ((value + 0.055) / 1.055).powf(2.4)
        }
    };
    0.2126 * channel(color.r()) + 0.7152 * channel(color.g()) + 0.0722 * channel(color.b())
}

/// Contrast ratio between two opaque colors, from 1.0 to 21.0.
pub fn contrast_ratio(a: Color32, b: Color32) -> f32 {
    let first = relative_luminance(a);
    let second = relative_luminance(b);
    let (light, dark) = if first > second {
        (first, second)
    } else {
        (second, first)
    };
    (light + 0.05) / (dark + 0.05)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn black_and_white_are_twenty_one_to_one() {
        let ratio = contrast_ratio(Color32::BLACK, Color32::WHITE);
        assert!((ratio - 21.0).abs() < 0.01);
    }

    #[test]
    fn identical_colors_have_no_contrast() {
        assert!((contrast_ratio(Color32::from_rgb(20, 30, 40), Color32::from_rgb(20, 30, 40)) - 1.0).abs() < 0.001);
    }
}
