use crate::ballistics::Coord;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Axis {
    X,
    Y,
}

fn axis_of(c: char) -> Option<Axis> {
    match c {
        // OCR sometimes renders the lowercase x as × or *.
        'x' | 'X' | '×' | '*' => Some(Axis::X),
        'y' | 'Y' => Some(Axis::Y),
        _ => None,
    }
}

fn digit_of(c: char) -> Option<char> {
    Some(match c {
        '0'..='9' => c,
        'l' | 'I' | 'i' | '|' | '!' => '1',
        'O' | 'o' | 'D' | 'Q' => '0',
        'S' | 's' => '5',
        'B' => '8',
        'Z' | 'z' => '2',
        'g' | 'q' => '9',
        'G' => '6',
        _ => return None,
    })
}

const MAX_COORD: f64 = 200.0;

fn is_sep(c: char) -> bool {
    c == '.' || c == ','
}

pub fn readings(text: &str) -> Vec<(Axis, f64)> {
    text.lines().flat_map(line_readings).collect()
}

fn line_readings(line: &str) -> Vec<(Axis, f64)> {
    let tokens: Vec<&str> = line.split_whitespace().collect();
    let mut out = Vec::new();
    let mut i = 0;
    while i < tokens.len() {
        let mut chars = tokens[i].chars();
        let Some(axis) = chars.next().and_then(axis_of) else {
            i += 1;
            continue;
        };

        let mut value = String::new();
        let mut parts = vec![chars.as_str().trim_start_matches([':', '='])];
        parts.extend(tokens[i + 1..].iter().copied());

        let mut used = 0;
        for part in parts {
            if !part.is_empty() && !part.chars().all(|c| digit_of(c).is_some() || is_sep(c)) {
                break;
            }
            value.push_str(part);
            used += 1;
            if complete(&value) {
                break;
            }
        }

        if let Some(v) = to_number(&value) {
            out.push((axis, v));
            i += used.max(1);
        } else {
            i += 1;
        }
    }
    out
}

fn complete(raw: &str) -> bool {
    raw.find(is_sep).is_some_and(|p| raw[p + 1..].chars().filter(|c| digit_of(*c).is_some()).count() >= 2)
}

fn to_number(raw: &str) -> Option<f64> {
    let digits = |s: &str| s.chars().filter_map(digit_of).collect::<String>();

    let (int, frac) = match raw.find(is_sep) {
        Some(p) => (digits(&raw[..p]), digits(&raw[p + 1..])),
        None => {
            // Readout always has two decimals.
            let all = digits(raw);
            if !(3..=5).contains(&all.len()) {
                return None;
            }
            let (int, frac) = all.split_at(all.len() - 2);
            (int.to_string(), frac.to_string())
        }
    };

    if !(1..=3).contains(&int.len()) || frac.is_empty() {
        return None;
    }
    let frac = &frac[..frac.len().min(2)];
    let v: f64 = format!("{int}.{frac}").parse().ok()?;
    (v < MAX_COORD).then_some(v)
}

pub fn parse_readout(text: &str) -> Option<Coord> {
    merge(readings(text))
}

pub fn merge(readings: impl IntoIterator<Item = (Axis, f64)>) -> Option<Coord> {
    let (mut x, mut y) = (None, None);
    for (axis, v) in readings {
        match axis {
            Axis::X => x = x.or(Some(v)),
            Axis::Y => y = y.or(Some(v)),
        }
    }
    Some(Coord { x: x?, y: y? })
}

#[derive(Default)]
pub struct Votes {
    x: Vec<(f64, u32)>,
    y: Vec<(f64, u32)>,
}

impl Votes {
    pub fn add(&mut self, readings: &[(Axis, f64)]) {
        for axis in [Axis::X, Axis::Y] {
            if let Some(&(_, v)) = readings.iter().find(|(a, _)| *a == axis) {
                let tally = match axis {
                    Axis::X => &mut self.x,
                    Axis::Y => &mut self.y,
                };
                match tally.iter_mut().find(|(t, _)| (*t - v).abs() < 1e-9) {
                    Some(entry) => entry.1 += 1,
                    None => tally.push((v, 1)),
                }
            }
        }
    }

    fn best(tally: &[(f64, u32)], quorum: u32) -> Option<f64> {
        // max_by_key keeps the last max; reverse so ties go to the earliest value.
        let &(v, n) = tally.iter().rev().max_by_key(|(_, n)| *n)?;
        (n >= quorum).then_some(v)
    }

    pub fn decided(&self, quorum: u32) -> Option<Coord> {
        Some(Coord {
            x: Self::best(&self.x, quorum)?,
            y: Self::best(&self.y, quorum)?,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn c(x: f64, y: f64) -> Option<Coord> {
        Some(Coord { x, y })
    }

    #[test]
    fn parses_in_game_readout() {
        assert_eq!(parse_readout("y111.29\nx92.92"), c(92.92, 111.29));
    }

    #[test]
    fn tolerates_ocr_spacing_and_case() {
        assert_eq!(parse_readout("Y 111 . 29  X92,92"), c(92.92, 111.29));
        assert_eq!(parse_readout("×92.92 y111.29"), c(92.92, 111.29));
        assert_eq!(parse_readout("YI 11.29\n*92.92"), c(92.92, 111.29));
    }

    #[test]
    fn fixes_letter_digit_confusion() {
        assert_eq!(parse_readout("ylll.29\nx92.9Z"), c(92.92, 111.29));
        assert_eq!(parse_readout("y lll.29 xS0.O1"), c(50.01, 111.29));
    }

    #[test]
    fn restores_lost_decimal_point() {
        assert_eq!(parse_readout("y111Z9\nx9292"), c(92.92, 111.29));
    }

    #[test]
    fn ignores_integer_grid_labels() {
        assert_eq!(parse_readout("10 9 1KM x92.92"), None);
        assert_eq!(parse_readout("9 10 11 y111.29 x92.92 1KM"), c(92.92, 111.29));
        assert_eq!(parse_readout("x92.92 10\ny111.29"), c(92.92, 111.29));
    }

    #[test]
    fn merges_axes_from_separate_passes() {
        let a = readings("x92.92");
        let b = readings("y111.29");
        assert_eq!(merge(a.into_iter().chain(b)), c(92.92, 111.29));
    }

    #[test]
    fn rejects_values_past_map_bounds() {
        assert_eq!(parse_readout("x991.30 y110.92"), None);
    }

    #[test]
    fn votes_outvote_a_confident_misread() {
        let mut v = Votes::default();
        v.add(&readings("Y711.77\nx98.53"));
        assert_eq!(v.decided(2), None);
        v.add(&readings("y111.17\nx98.53"));
        assert_eq!(v.decided(2), None);
        v.add(&readings("ylll. 17"));
        assert_eq!(v.decided(2), c(98.53, 111.17));
        assert_eq!(v.decided(1), c(98.53, 111.17));
    }

    #[test]
    fn missing_axis_fails() {
        assert_eq!(parse_readout("x92.92"), None);
        assert_eq!(parse_readout(""), None);
        assert_eq!(parse_readout("Toggle Legend"), None);
    }
}
