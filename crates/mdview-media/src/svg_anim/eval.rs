//! Timeline evaluation and value interpolation.

use super::*;

impl Clip {
    pub(super) fn value_at(&self, time: f32) -> Option<String> {
        let local = time - self.begin;
        if local < 0.0 {
            return if self.set_like {
                None
            } else {
                self.values.first().cloned()
            };
        }
        if self.set_like {
            return self.values.first().cloned();
        }
        let cycle = match &self.repeat {
            Repeat::Once => {
                if local > self.duration {
                    return self.values.last().cloned();
                }
                local
            }
            Repeat::Count(count) => {
                let total = self.duration * *count as f32;
                if local >= total {
                    return self.values.last().cloned();
                }
                local % self.duration
            }
            Repeat::Indefinite => local % self.duration.max(0.001),
        };
        if self.duration <= 0.0 || self.values.len() == 1 {
            return self.values.last().cloned();
        }
        let progress = (cycle / self.duration).clamp(0.0, 1.0);
        let times = if self.key_times.len() == self.values.len() {
            self.key_times.clone()
        } else {
            (0..self.values.len())
                .map(|index| index as f32 / (self.values.len() - 1) as f32)
                .collect()
        };
        let mut segment = 0;
        for index in 0..times.len().saturating_sub(1) {
            if progress >= times[index] && progress <= times[index + 1] {
                segment = index;
                break;
            }
        }
        if matches!(self.calc, CalcMode::Discrete) {
            let index = ((progress * self.values.len() as f32).floor() as usize).min(self.values.len() - 1);
            return Some(self.values[index].clone());
        }
        let start = times[segment];
        let end = times[segment + 1];
        let fraction = if end > start {
            ((progress - start) / (end - start)).clamp(0.0, 1.0)
        } else {
            0.0
        };
        Some(interpolate(&self.values[segment], &self.values[segment + 1], fraction))
    }
}

fn interpolate(from: &str, to: &str, fraction: f32) -> String {
    if let (Some(from), Some(to)) = (parse_hex_color(from), parse_hex_color(to)) {
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

fn parse_hex_color(value: &str) -> Option<[u8; 3]> {
    let value = value.trim();
    let hex = value.strip_prefix('#')?;
    if hex.len() != 6 {
        return None;
    }
    Some([
        u8::from_str_radix(&hex[0..2], 16).ok()?,
        u8::from_str_radix(&hex[2..4], 16).ok()?,
        u8::from_str_radix(&hex[4..6], 16).ok()?,
    ])
}
