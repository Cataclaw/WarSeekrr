use serde::{Deserialize, Serialize};
use std::sync::OnceLock;

const FIRING_TABLES: &str = include_str!("../data/firing_tables.json");

/// Distances arrive from hypot(), so a row hit can land a float step off.
const RANGE_EPSILON: f64 = 1e-6;

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct Coord {
    pub x: f64,
    pub y: f64,
}

#[derive(Debug, Deserialize)]
pub struct WeaponData {
    pub meters_per_unit: f64,
    #[serde(rename = "default")]
    pub default_weapon: String,
    pub weapons: Vec<Weapon>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all(serialize = "camelCase"))]
pub struct Weapon {
    pub id: String,
    pub name: String,
    pub range_m: (f64, f64),
    pub arcs: Vec<Arc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Arc {
    pub id: String,
    pub name: String,
    pub table: Vec<(f64, f64)>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ArcSolution {
    pub id: String,
    pub name: String,
    pub min_mil: f64,
    pub max_mil: f64,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Solution {
    pub distance_m: f64,
    pub azimuth_deg: f64,
    pub in_range: bool,
    pub arcs: Vec<ArcSolution>,
}

pub fn data() -> &'static WeaponData {
    static DATA: OnceLock<WeaponData> = OnceLock::new();
    DATA.get_or_init(|| serde_json::from_str(FIRING_TABLES).expect("bundled firing_tables.json is valid"))
}

pub fn weapon(id: &str) -> Option<&'static Weapon> {
    data().weapons.iter().find(|w| w.id == id)
}

pub fn distance_meters(from: Coord, to: Coord) -> f64 {
    (to.x - from.x).hypot(to.y - from.y) * data().meters_per_unit
}

/// 0 = north (+Y), 90 = east (+X).
pub fn azimuth_degrees(from: Coord, to: Coord) -> f64 {
    let deg = (to.x - from.x).atan2(to.y - from.y).to_degrees();
    if deg < 0.0 {
        deg + 360.0
    } else {
        deg
    }
}

pub fn elevation_mil(table: &[(f64, f64)], range_m: f64) -> Option<(f64, f64)> {
    if !range_m.is_finite() {
        return None;
    }

    let hits: Vec<f64> = table
        .iter()
        .filter(|(r, _)| (r - range_m).abs() <= RANGE_EPSILON)
        .map(|&(_, mil)| mil)
        .collect();
    if !hits.is_empty() {
        let min = hits.iter().copied().fold(f64::INFINITY, f64::min);
        let max = hits.iter().copied().fold(f64::NEG_INFINITY, f64::max);
        return Some((min, max));
    }

    table.windows(2).find_map(|pair| {
        let (lr, lm) = pair[0];
        let (rr, rm) = pair[1];
        // Strict comparisons, so rows sharing a range never bracket each other.
        (range_m > lr && range_m < rr).then(|| {
            let mil = lm + (range_m - lr) / (rr - lr) * (rm - lm);
            (mil, mil)
        })
    })
}

pub fn solve(weapon: &Weapon, artillery: Coord, target: Coord) -> Solution {
    let distance_m = distance_meters(artillery, target);
    let in_range = distance_m >= weapon.range_m.0 && distance_m <= weapon.range_m.1;

    let arcs = if in_range {
        weapon
            .arcs
            .iter()
            .filter_map(|arc| {
                elevation_mil(&arc.table, distance_m).map(|(min_mil, max_mil)| ArcSolution {
                    id: arc.id.clone(),
                    name: arc.name.clone(),
                    min_mil,
                    max_mil,
                })
            })
            .collect()
    } else {
        Vec::new()
    };

    Solution {
        distance_m,
        azimuth_deg: azimuth_degrees(artillery, target),
        in_range,
        arcs,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn c(x: f64, y: f64) -> Coord {
        Coord { x, y }
    }

    #[test]
    fn mortar_reference_vector() {
        let s = solve(weapon("mortar").unwrap(), c(50.0, 50.0), c(53.0, 54.0));
        assert!((s.distance_m - 500.0).abs() < 1e-9);
        assert!((s.azimuth_deg - 36.87).abs() < 0.01);
        assert!(s.in_range);
        assert_eq!(s.arcs.len(), 1);
        assert_eq!(s.arcs[0].min_mil.round(), 461.0);
    }

    #[test]
    fn sph2_reference_vector() {
        let s = solve(weapon("sph2").unwrap(), c(50.0, 50.0), c(65.0, 50.0));
        assert!((s.distance_m - 1500.0).abs() < 1e-9);
        assert!((s.azimuth_deg - 90.0).abs() < 1e-9);
        let mils: Vec<f64> = s.arcs.iter().map(|a| a.min_mil.round()).collect();
        assert_eq!(mils, vec![84.0, 1213.0]);
    }

    #[test]
    fn sph2_duplicate_range_is_span() {
        let high = &weapon("sph2").unwrap().arcs.iter().find(|a| a.id == "high").unwrap().table;
        assert_eq!(elevation_mil(high, 2629.0), Some((610.0, 620.0)));
    }

    #[test]
    fn out_of_range_has_no_arcs() {
        let s = solve(weapon("mortar").unwrap(), c(50.0, 50.0), c(50.0, 60.0));
        assert!(!s.in_range);
        assert!(s.arcs.is_empty());
    }

    #[test]
    fn azimuth_quadrants() {
        let o = c(0.0, 0.0);
        assert_eq!(azimuth_degrees(o, c(0.0, 1.0)), 0.0);
        assert_eq!(azimuth_degrees(o, c(1.0, 0.0)), 90.0);
        assert_eq!(azimuth_degrees(o, c(0.0, -1.0)), 180.0);
        assert_eq!(azimuth_degrees(o, c(-1.0, 0.0)), 270.0);
    }
}
