//! Native message draw requests. Font rasterization remains a backend concern.
use crate::mission_messages::Queue;
use giants_formats::gtext::Catalog;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Palette(pub [u32; 7]);
impl Palette {
    pub fn load(catalog: &Catalog) -> Result<Self, &'static str> {
        let mut colors = [
            0, 0x1fffff, 0xf0f01f, 0xd0d0d0, 0x1f1fff, 0xff1fff, 0x1fffff,
        ];
        for (slot, key) in [
            "ChatColorSystem",
            "ChatColorPlayerAll",
            "ChatColorPlayerTeam",
            "ChatColorSmartie",
            "ChatColorDisciple",
            "ChatColorMission",
        ]
        .into_iter()
        .enumerate()
        {
            let Some(value) = catalog.get(key.as_bytes()) else {
                continue;
            };
            if value.first() == Some(&b'?') {
                continue;
            }
            colors[slot + 1] = hexadecimal(value)?;
        }
        Ok(Self(colors))
    }
}
fn hexadecimal(text: &[u8]) -> Result<u32, &'static str> {
    let text = if text.starts_with(b"0x") || text.starts_with(b"0X") {
        &text[2..]
    } else {
        text
    };
    let mut value = 0u32;
    for (i, &byte) in text.iter().enumerate() {
        let digit = match byte {
            b'0'..=b'9' => byte - b'0',
            b'a'..=b'f' => byte - b'a' + 10,
            b'A'..=b'F' => byte - b'A' + 10,
            _ => break,
        };
        if i >= 8 {
            return Err("message color outside eight-digit profile");
        }
        value = value.wrapping_mul(16).wrapping_add(u32::from(digit));
    }
    Ok(value)
}
#[derive(Debug, Clone, PartialEq)]
pub struct Draw {
    pub text: Vec<u8>,
    pub color: u32,
    pub size: f32,
    pub x: f32,
    pub y: f32,
    pub centered: bool,
}
/// Coordinates, size and row advance acquired from the ordinary message path.
/// Prompt/cursor, history heading and decorative background are separate.
pub fn commands(
    queue: &Queue,
    palette: &Palette,
    now: f32,
    history: bool,
    alternate: bool,
) -> Result<Vec<Draw>, &'static str> {
    let selection = queue.select(now, history, alternate);
    let Some(last) = selection.last else {
        return Ok(vec![]);
    };
    let x = if alternate { 0.1 } else { 0.18 };
    let mut y = if alternate {
        0.85f32
    } else if history {
        0.07f32
    } else {
        0.05f32
    };
    let mut draws = vec![];
    for i in selection.first..=last {
        let Some((text, route)) = queue.line(i) else {
            continue;
        };
        let color = *palette
            .0
            .get(route as usize)
            .ok_or("message route outside acquired palette")?;
        let prefix = text[0];
        let special = matches!(prefix, 11 | 12);
        draws.push(Draw {
            text: if special {
                text[1..].to_vec()
            } else {
                text.to_vec()
            },
            color,
            size: if special { 0.03125 * 1.5 } else { 0.03125 },
            x: if prefix == 12 { 0.5 } else { x },
            y,
            centered: prefix == 12,
        });
        y += 0.03125;
    }
    Ok(draws)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn color_profile_stops_like_native_and_preserves_defaults() {
        assert_eq!(hexadecimal(b"0XaBc123 remainder"), Ok(0xabc123));
        assert_eq!(hexadecimal(b" ffffff"), Ok(0));
        assert_eq!(hexadecimal(b""), Ok(0));
        assert_eq!(hexadecimal(b"ffffffff"), Ok(u32::MAX));
        assert!(hexadecimal(b"100000000").is_err());
        let mut table = giants_formats::gtext::TextTable::new();
        table.insert(b"chatcolormission".to_vec(), b"0X12aBcD".to_vec());
        let palette = Palette::load(&Catalog::from_table(table).unwrap()).unwrap();
        assert_eq!(palette.0[1], 0x1fffff);
        assert_eq!(palette.0[6], 0x12abcd);
    }
    #[test]
    fn draw_controls_are_leading_only_and_invalid_routes_are_refused() {
        let palette = Palette([0, 1, 2, 3, 4, 5, 6]);
        let mut queue = Queue::default();
        queue.post(Some(b"\x0blarge"), 6, 1.).unwrap();
        queue.post(Some(b"\x0ccenter"), 6, 1.).unwrap();
        queue.post(Some(b"inside\x0b"), 6, 1.).unwrap();
        let rows = commands(&queue, &palette, 1., false, false).unwrap();
        assert_eq!(rows.len(), 3);
        assert_eq!(rows[0].text, b"large");
        assert_eq!(rows[0].size, 0.046875);
        assert!(!rows[0].centered);
        assert!(rows[1].centered);
        assert_eq!(rows[1].x, 0.5);
        assert_eq!(rows[2].text, b"inside\x0b");
        queue.post(Some(b"unsupported"), 7, 1.).unwrap();
        assert!(commands(&queue, &palette, 1., false, false).is_err());
    }
}
