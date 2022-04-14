use std::cmp::Ordering;

use log::warn;

use crate::math;
use crate::math::{Line, Vector2};

#[derive(PartialEq)]
pub enum CurveType {
    Catmull,
    Bezier,
    Linear,
    PerfectCircle,
}

#[derive(Default)]
pub struct Curve {
    pub lines: Vec<Line>,
    pub line_lengths: Vec<f32>,
    pub length: f32,
}

impl Curve {
    pub fn new(
        curve_type: CurveType,
        control_points: Vec<Vector2>,
        _beatmap_version: i32,
        pixel_length: f64,
    ) -> Self {
        let mut lines = match curve_type {
            CurveType::Catmull => Self::handle_catmull(control_points),
            CurveType::Bezier => Self::handle_bezier(control_points),
            CurveType::Linear => Self::handle_linear(control_points),
            CurveType::PerfectCircle => Self::handle_perfect_circle(control_points),
        };
        // apparently, this was too inaccurate
        /*
        let mut new_length = 0.0;
        let mut too_long_idx = None;
        let mut too_long_p2 = Vector2::default();
        for i in 0..lines.len() {
            let len = lines[i].length();
            if pixel_length != 0.0 && len + new_length > pixel_length {
                too_long_idx = Some(i);
                too_long_p2 = lines[i].point_at((pixel_length - new_length) / pixel_length);
                new_length = pixel_length;
                break;
            }
            line_lengths[i] = new_length;
            new_length += len;
        }
        if let Some(too_long_idx) = too_long_idx {
            lines.truncate(too_long_idx + 1);
            line_lengths.truncate(too_long_idx + 1);
            let len = lines.len();
            if len > 0 {
                lines[len - 1] = Line::new(lines[len - 1].p1, too_long_p2);
                line_lengths[len - 1] = new_length;
            }
        }
        */

        let mut curve_len = 0.0;
        for x in &lines {
            curve_len += x.length();
        }

        if pixel_length > 0.0 {
            let mut diff = curve_len as f64 - pixel_length;

            while let Some(x) = lines.pop() {
                if x.length() as f32 > diff as f32 + 0.0001 {
                    if x.p1 != x.p2 {
                        //let pt = x.point_at((x.length() - diff as f32) / x.length());
                        let pt = x.p1 + (x.p2 - x.p1).normalize() * (x.length() - diff as f32);
                        lines.push(Line::new(x.p1, pt));
                    } else {
                        lines.push(x);
                    }
                    break;
                }
                diff -= x.length() as f64;
            }
        }

        let mut line_lengths = vec![0.0; lines.len()];

        curve_len = 0.0;
        for i in 0..lines.len() {
            line_lengths[i] = curve_len;
            curve_len += lines[i].length();
        }

        Curve {
            lines,
            line_lengths,
            length: curve_len,
        }
    }

    fn line_at(&self, amount: f32) -> (usize, f32) {
        let target_length = amount * self.length as f32;
        let target_idx = self
            .line_lengths
            .partition_point(|&x| x < target_length)
            .saturating_sub(1);
        (target_idx, target_length)
    }

    pub fn point_at(&self, amount: f32) -> Vector2 {
        // TODO: i think this just returns a NaN position
        if self.length == 0.0 || self.lines.is_empty() {
            return Vector2::new(0.0, 0.0);
        }
        let (target_idx, target_length) = self.line_at(amount);
        let line = &self.lines[target_idx];
        line.point_at((target_length - self.line_lengths[target_idx]) / line.length())
    }

    pub fn angle_at(&self, amount: f32) -> f32 {
        if self.length == 0.0 || self.lines.is_empty() {
            return 0.0;
        }
        let (target_idx, _target_length) = self.line_at(amount);
        let line = &self.lines[target_idx];
        (line.p2.y - line.p1.y).atan2(line.p2.x - line.p1.x)
    }

    fn handle_catmull(control_points: Vec<Vector2>) -> Vec<Line> {
        let mut lines = Vec::with_capacity(control_points.len() * 50);
        for i in 0..control_points.len() {
            let p1 = control_points[i.saturating_sub(1)];
            let p2 = control_points[i as usize];
            let p3 = if i + 1 < control_points.len() {
                control_points[(i + 1) as usize]
            } else {
                p2 + (p2 - p1)
            };
            let p4 = if i + 2 < control_points.len() {
                control_points[(i + 2) as usize]
            } else {
                p3 + (p3 - p2)
            };
            for i in 0..50 {
                lines.push(Line::new(
                    math::catmull_rom(p1, p2, p3, p4, i as f32 / 50.0),
                    math::catmull_rom(p1, p2, p3, p4, (i + 1) as f32 / 50.0),
                ));
            }
        }
        lines
    }

    fn handle_bezier(control_points: Vec<Vector2>) -> Vec<Line> {
        let mut lines = Vec::default();
        let mut start_idx = 0;
        let mut it = 0..control_points.len();

        if control_points.len() > 1 {
            // make sure that none of the control points are crazy
            // "crazy" as in over 1 billion osu pixels away from the previous point
            let mut last_point = &control_points[0];
            let mut crazy = false;
            for x in control_points.iter().skip(1) {
                if last_point.distance(*x) > 1_000_000_000.0 {
                    warn!("Skipping a bezier curve because a control point is {} pixels away from another", last_point.distance(*x));
                    crazy = true;
                    break;
                }
                last_point = x;
            }
            if crazy {
                return lines;
            }

            while let Some(i) = it.next() {
                // osu's bezier curve handling changed a lot over time
                // this behavior isn't mimiced in anything else i can find for some reason
                // TODO: implement bezier curve behavior for old versions
                // this is currently the behavior for versions 10+
                let skip =
                    i < control_points.len() - 2 && control_points[i] == control_points[i + 1];
                if !skip && i != control_points.len() - 1 {
                    continue;
                }

                let range = &control_points[start_idx..(i + 1)];
                if range.len() == 2 {
                    // the original code seems to subdivide a line for whatever reason
                    // except it subdivides with a resolution of 1, so it's useless
                    lines.push(Line::new(range[0], range[1]));
                } else {
                    let points = math::bezier(range.to_vec());
                    for x in 1..points.len() {
                        lines.push(Line::new(points[x - 1], points[x]));
                    }
                }
                if skip {
                    it.next();
                }
                start_idx = i + 1;
            }
        }

        lines
    }

    fn handle_linear(control_points: Vec<Vector2>) -> Vec<Line> {
        let mut lines = Vec::default();
        for i in 1..control_points.len() {
            lines.push(Line::new(control_points[i - 1], control_points[i]));
        }
        lines
    }

    fn handle_perfect_circle(control_points: Vec<Vector2>) -> Vec<Line> {
        match control_points.len().cmp(&3) {
            Ordering::Less => Self::handle_linear(control_points),
            Ordering::Greater => Self::handle_bezier(control_points),
            Ordering::Equal => {
                if math::straight_line(control_points[0], control_points[1], control_points[2]) {
                    return Self::handle_linear(control_points);
                }

                let (center, radius, t_initial, t_final) = math::circle_through_points(
                    control_points[0],
                    control_points[1],
                    control_points[2],
                );
                let curve_length = ((t_final - t_initial) * radius as f64).abs(); // this is an object field in the game, but i don't think i'd ever need to use it
                let point_count = (curve_length * 0.125) as i32;

                let mut lines = Vec::with_capacity((point_count * 50) as usize);
                let mut last_point = control_points[0];
                for i in 1..point_count {
                    let progress = i as f64 / point_count as f64;
                    let t = t_final * progress + t_initial * (1.0 - progress);
                    let point = math::circle_point(center, radius, t);
                    lines.push(Line::new(last_point, point));
                    last_point = point;
                }
                lines.push(Line::new(last_point, control_points[2]));

                lines
            }
        }
    }
}

#[cfg(test)]
mod tests {
    fn test_curve(
        curve_type: crate::curve::CurveType,
        pixel_length: f64,
        input_path: &str,
        output_path: &str,
    ) {
        use crate::curve::*;
        use crate::math::*;
        use crate::num_util::*;
        // generated from decompiled osu-stable code
        // one of the stupid sliders in unshakable aspire
        let path = {
            let mut out = Vec::<Vector2>::new();
            let path = std::fs::read_to_string(input_path).unwrap();
            let slider_split = path.split('|');
            for entry in slider_split {
                let mut point_split = entry.split(':');
                // TODO: ewwww
                let x = f64_to_wrapping_i32(point_split.next().unwrap().parse::<f64>().unwrap());
                let y = f64_to_wrapping_i32(point_split.next().unwrap().parse::<f64>().unwrap());
                out.push(Vector2::new(x as f32, y as f32));
            }
            out
        };
        let expected = {
            let mut out = Vec::<Line>::new();
            let expected = std::fs::read_to_string(output_path).unwrap();
            let entry_split = expected.split('|');
            for entry in entry_split {
                let mut line_split = entry.split(':');
                // TODO: ewwwwwwwwwww
                let x1 = line_split.next().unwrap().parse::<f64>().unwrap();
                let y1 = line_split.next().unwrap().parse::<f64>().unwrap();
                let x2 = line_split.next().unwrap().parse::<f64>().unwrap();
                let y2 = line_split.next().unwrap().parse::<f64>().unwrap();

                out.push(Line::new(
                    Vector2::new(x1 as f32, y1 as f32),
                    Vector2::new(x2 as f32, y2 as f32),
                ));
            }
            out
        };
        let curve = Curve::new(curve_type, path, 14, pixel_length);
        //println!("{:?}", curve.lines);
        assert_eq!(curve.lines.len(), expected.len());
        for (a, b) in curve.lines.iter().zip(expected.iter()) {
            /*
            assert_eq!((a.p1.x - b.p1.x).abs() < 0.01, true);
            assert_eq!((a.p1.y - b.p1.y).abs() < 0.01, true);
            assert_eq!((a.p2.x - b.p2.x).abs() < 0.01, true);
            assert_eq!((a.p2.y - b.p2.y).abs() < 0.01, true);
            */
            assert_eq!(a.p1.x, b.p1.x);
            assert_eq!(a.p1.y, b.p1.y);
            assert_eq!(a.p2.x, b.p2.x);
            assert_eq!(a.p2.y, b.p2.y);
        }
    }

    #[test]
    fn test_bezier() {
        // TODO: generate test case that uses specified pixel length
        test_curve(
            crate::curve::CurveType::Bezier,
            0.0,
            "test/unshakable_path.in",
            "test/unshakable_path_interp.in",
        );
        test_curve(
            crate::curve::CurveType::Bezier,
            0.0,
            "test/simple_slider.in",
            "test/simple_slider_interp.in",
        );
    }
}
