#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Color {
    pub r: u8,
    pub g: u8,
    pub b: u8,
    pub a: u8,
}

impl Color {
    pub const fn rgb(r: u8, g: u8, b: u8) -> Self {
        Color { r, g, b, a: 255 }
    }

    pub const fn rgba(r: u8, g: u8, b: u8, a: u8) -> Self {
        Color { r, g, b, a }
    }

    pub fn from_hex(s: &str) -> Option<Self> {
        let s = s.strip_prefix('#').unwrap_or(s);
        if !s.is_ascii() {
            return None;
        }
        let byte = |i: usize| u8::from_str_radix(&s[i..i + 2], 16).ok();
        match s.len() {
            3 => {
                let nib = |i: usize| u8::from_str_radix(&s[i..i + 1], 16).ok().map(|v| v * 17);
                Some(Color::rgb(nib(0)?, nib(1)?, nib(2)?))
            }
            6 => Some(Color::rgb(byte(0)?, byte(2)?, byte(4)?)),
            8 => Some(Color::rgba(byte(0)?, byte(2)?, byte(4)?, byte(6)?)),
            _ => None,
        }
    }

    pub fn to_hex(self) -> String {
        if self.a == 255 {
            format!("#{:02x}{:02x}{:02x}", self.r, self.g, self.b)
        } else {
            format!("#{:02x}{:02x}{:02x}{:02x}", self.r, self.g, self.b, self.a)
        }
    }
}

impl std::fmt::Display for Color {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.to_hex())
    }
}

pub mod hex_opt {
    use super::Color;
    use serde::{Deserialize, Deserializer, Serializer};

    pub fn serialize<S: Serializer>(value: &Option<Color>, s: S) -> Result<S::Ok, S::Error> {
        s.serialize_str(&value.map(Color::to_hex).unwrap_or_default())
    }

    pub fn deserialize<'de, D: Deserializer<'de>>(d: D) -> Result<Option<Color>, D::Error> {
        let s = String::deserialize(d)?;
        Ok(if s.is_empty() {
            None
        } else {
            Color::from_hex(&s)
        })
    }
}

#[cfg(test)]
mod tests {
    use super::Color;

    #[test]
    fn hex_round_trips_for_opaque_and_alpha() {
        let opaque = Color::from_hex("#22aa22").unwrap();
        assert_eq!(opaque, Color::rgb(0x22, 0xaa, 0x22));
        assert_eq!(opaque.to_hex(), "#22aa22");

        let alpha = Color::from_hex("#22aa2280").unwrap();
        assert_eq!(alpha, Color::rgba(0x22, 0xaa, 0x22, 0x80));
        assert_eq!(alpha.to_hex(), "#22aa2280");
    }

    #[test]
    fn hex_accepts_short_form_and_rejects_garbage() {
        assert_eq!(Color::from_hex("#0af"), Some(Color::rgb(0x00, 0xaa, 0xff)));
        assert_eq!(Color::from_hex("zzzzzz"), None);
        assert_eq!(Color::from_hex(""), None);
    }
}
