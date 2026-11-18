//! Timeline evaluation and value interpolation.

use super::*;

impl Clip {
    /// The value at `time`, or `None` when the clip does not apply and the element keeps its
    /// own value: before `begin`, and after the active duration unless the clip freezes.
    pub(super) fn value_at(&self, time: f32) -> Option<String> {
        let local = time - self.begin;
        if local < 0.0 {
            return None;
        }
        if self.set_like {
            return self.values.first().cloned();
        }
        let duration = self.duration.max(0.001);
        let iterations = match &self.repeat {
            Repeat::Once => Some(1.0),
            Repeat::Count(count) => Some(*count as f32),
            Repeat::Indefinite => None,
        };
        let (iteration, cycle) = match iterations {
            Some(count) if local > duration * count => {
                if !self.freeze {
                    return None;
                }
                (count.ceil() as u32 - 1, 1.0)
            }
            // The end of an iteration shows its last value, not the next iteration's first.
            Some(count) if local >= duration * count => (count.ceil() as u32 - 1, 1.0),
            _ => ((local / duration).floor() as u32, (local % duration) / duration),
        };
        let reversed = match self.direction {
            Direction::Normal => false,
            Direction::Reverse => true,
            Direction::Alternate => iteration % 2 == 1,
            Direction::AlternateReverse => iteration % 2 == 0,
        };
        let progress = if reversed { 1.0 - cycle } else { cycle }.clamp(0.0, 1.0);
        if let Property::Motion(path, rotate) = &self.property {
            let eased = self.easing.first().map_or(progress, |easing| easing.apply(progress));
            return Some(path.transform(eased, *rotate));
        }
        if self.values.len() == 1 {
            return self.values.first().cloned();
        }
        let times = if self.key_times.len() == self.values.len() {
            self.key_times.clone()
        } else {
            (0..self.values.len())
                .map(|index| index as f32 / (self.values.len() - 1) as f32)
                .collect()
        };
        if matches!(self.calc, CalcMode::Discrete) {
            let index = ((progress * self.values.len() as f32).floor() as usize).min(self.values.len() - 1);
            return Some(self.values[index].clone());
        }
        let mut segment = times.len().saturating_sub(2);
        for index in 0..times.len().saturating_sub(1) {
            if progress >= times[index] && progress <= times[index + 1] {
                segment = index;
                break;
            }
        }
        let start = times[segment];
        let end = times[segment + 1];
        let fraction = if end > start {
            ((progress - start) / (end - start)).clamp(0.0, 1.0)
        } else {
            0.0
        };
        // SMIL gives one spline per segment; CSS gives one timing function for all of them.
        let easing = self
            .easing
            .get(segment)
            .or_else(|| (self.easing.len() == 1).then(|| &self.easing[0]));
        let fraction = easing.map_or(fraction, |easing| easing.apply(fraction));
        Some(interpolate(&self.values[segment], &self.values[segment + 1], fraction))
    }
}

fn interpolate(from: &str, to: &str, fraction: f32) -> String {
    if let (Some(from), Some(to)) = (super::easing::parse_color(from), super::easing::parse_color(to)) {
        let mix = |a: u8, b: u8| (a as f32 + (b as f32 - a as f32) * fraction).round() as u8;
        return format!(
            "#{:02x}{:02x}{:02x}",
            mix(from[0], to[0]),
            mix(from[1], to[1]),
            mix(from[2], to[2])
        );
    }
    if let (Some(from_parts), Some(to_parts)) = (parts(from), parts(to))
        && from_parts.len() == to_parts.len()
    {
        let mut output = String::new();
        for (left, right) in from_parts.iter().zip(&to_parts) {
            match (left, right) {
                (Part::Number(left), Part::Number(right)) => {
                    output.push_str(&format!("{:.3}", left + (right - left) * fraction));
                }
                (Part::Text(left), Part::Text(right)) if left == right => output.push_str(left),
                _ => return if fraction < 0.5 { from.to_owned() } else { to.to_owned() },
            }
        }
        return output;
    }
    if fraction < 0.5 { from.to_owned() } else { to.to_owned() }
}

#[derive(Debug, PartialEq)]
enum Part {
    Number(f32),
    Text(String),
}

fn parts(value: &str) -> Option<Vec<Part>> {
    let mut result = Vec::new();
    let mut text = String::new();
    let mut number = String::new();
    for character in value.chars() {
        let numeric = character.is_ascii_digit() || matches!(character, '.' | '-' | '+');
        if numeric {
            if !text.is_empty() {
                result.push(Part::Text(std::mem::take(&mut text)));
            }
            number.push(character);
        } else {
            if !number.is_empty() {
                result.push(Part::Number(number.parse().ok()?));
                number.clear();
            }
            text.push(character);
        }
    }
    if !number.is_empty() {
        result.push(Part::Number(number.parse().ok()?));
    }
    if !text.is_empty() {
        result.push(Part::Text(text));
    }
    Some(result)
}
