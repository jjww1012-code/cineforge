use serde::{Deserialize, Deserializer, Serialize, Serializer};

/// sRGB-Farbe. In JSON als `"#ff8800"` oder `[1.0, 0.5, 0.0]` (0..1).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Rgb {
    pub r: f32,
    pub g: f32,
    pub b: f32,
}

impl Rgb {
    pub const WHITE: Rgb = Rgb::new(1.0, 1.0, 1.0);
    pub const BLACK: Rgb = Rgb::new(0.0, 0.0, 0.0);

    pub const fn new(r: f32, g: f32, b: f32) -> Self {
        Self { r, g, b }
    }

    pub fn parse_hex(s: &str) -> Option<Rgb> {
        let h = s.strip_prefix('#')?;
        let h: String = match h.len() {
            3 => h.chars().flat_map(|c| [c, c]).collect(),
            6 => h.to_string(),
            _ => return None,
        };
        let v = u32::from_str_radix(&h, 16).ok()?;
        let c = |shift: u32| ((v >> shift) & 0xff) as f32 / 255.0;
        Some(Rgb::new(c(16), c(8), c(0)))
    }

    pub fn to_hex(self) -> String {
        let b = |v: f32| (v.clamp(0.0, 1.0) * 255.0).round() as u8;
        format!("#{:02x}{:02x}{:02x}", b(self.r), b(self.g), b(self.b))
    }

    pub fn is_black(self) -> bool {
        self.r <= 0.0 && self.g <= 0.0 && self.b <= 0.0
    }
}

impl Serialize for Rgb {
    fn serialize<S: Serializer>(&self, s: S) -> Result<S::Ok, S::Error> {
        s.serialize_str(&self.to_hex())
    }
}

impl<'de> Deserialize<'de> for Rgb {
    fn deserialize<D: Deserializer<'de>>(d: D) -> Result<Self, D::Error> {
        #[derive(Deserialize)]
        #[serde(untagged)]
        enum Repr {
            Hex(String),
            Arr([f32; 3]),
        }
        match Repr::deserialize(d)? {
            Repr::Hex(s) => Rgb::parse_hex(&s).ok_or_else(|| {
                serde::de::Error::custom(format!("ungültige Farbe \"{s}\" (erwartet z. B. \"#ff8800\")"))
            }),
            Repr::Arr([r, g, b]) => Ok(Rgb::new(r, g, b)),
        }
    }
}
