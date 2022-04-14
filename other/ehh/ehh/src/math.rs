use std::ops::{Add, Div, Mul, Sub};

#[derive(Default, Copy, Clone, Debug, PartialEq)]
pub struct Vector2 {
    pub x: f32,
    pub y: f32,
}
impl Vector2 {
    pub fn new(x: f32, y: f32) -> Self {
        Vector2 { x, y }
    }
    pub fn length_squared(&self) -> f32 {
        self.x * self.x + self.y * self.y
    }
    pub fn distance(&self, point: Vector2) -> f32 {
        //((self.x - point.x).powf(2.0) + (self.y - point.y).powf(2.0)).sqrt()
        let x = point.x - self.x;
        let y = point.y - self.y;
        (x * x + y * y).sqrt()
    }
    pub fn length(&self) -> f32 {
        (self.x * self.x + self.y * self.y).sqrt()
    }

    #[must_use]
    pub fn normalize(&self) -> Self {
        let inv_length = 1.0 / self.length();
        *self * inv_length
    }
}
impl Add for Vector2 {
    type Output = Self;

    fn add(self, other: Self) -> Self {
        Self {
            x: self.x + other.x,
            y: self.y + other.y,
        }
    }
}
impl Sub for Vector2 {
    type Output = Self;

    fn sub(self, other: Self) -> Self {
        Self {
            x: self.x - other.x,
            y: self.y - other.y,
        }
    }
}
impl Mul<f32> for Vector2 {
    type Output = Self;

    fn mul(self, scale: f32) -> Self {
        Self {
            x: self.x * scale,
            y: self.y * scale,
        }
    }
}
impl Div<f32> for Vector2 {
    type Output = Self;

    fn div(self, scale: f32) -> Self {
        Self {
            x: self.x / scale,
            y: self.y / scale,
        }
    }
}

impl From<Vector2> for (f32, f32) {
    fn from(x: Vector2) -> (f32, f32) {
        (x.x, x.y)
    }
}

#[derive(Default, Copy, Clone, Debug)]
pub struct Line {
    pub p1: Vector2,
    pub p2: Vector2,
}
impl Line {
    pub fn new(p1: Vector2, p2: Vector2) -> Self {
        Line { p1, p2 }
    }
    pub fn length(&self) -> f32 {
        self.p1.distance(self.p2)
    }
    pub fn point_at(&self, amount: f32) -> Vector2 {
        lerp(self.p1, self.p2, amount)
    }
}

pub fn lerp(p1: Vector2, p2: Vector2, amount: f32) -> Vector2 {
    Vector2 {
        x: p1.x + (p2.x - p1.x) * amount,
        y: p1.y + (p2.y - p1.y) * amount,
    }
}

pub enum Easing {
    Linear,
    OutQuad,
}

// TODO: lol no generics
pub fn interp_time(v1: f32, v2: f32, t1: f32, t2: f32, t: f32, easing: Easing) -> f32 {
    //let t = t.clamp(t1, t2);
    let p = (t - t1) / (t2 - t1);
    let p = match easing {
        Easing::Linear => p,
        Easing::OutQuad => 1.0 - (1.0 - p) * (1.0 - p),
    };
    (v2 - v1) * p + v1
}

pub fn catmull_rom(p1: Vector2, p2: Vector2, p3: Vector2, p4: Vector2, amount: f32) -> Vector2 {
    let squared = amount * amount;
    let cubed = squared * amount;

    let x = 0.5f32
        * (2f32 * p2.x
            + (0f32 - p1.x + p3.x) * amount
            + (2f32 * p1.x - 5f32 * p2.x + 4f32 * p3.x - p4.x) * squared
            + (0f32 - p1.x + 3f32 * p2.x - 3f32 * p3.x + p4.x) * cubed);
    let y = 0.5f32
        * (2f32 * p2.y
            + (0f32 - p1.y + p3.y) * amount
            + (2f32 * p1.y - 5f32 * p2.y + 4f32 * p3.y - p4.y) * squared
            + (0f32 - p1.y + 3f32 * p2.y - 3f32 * p3.y + p4.y) * cubed);
    Vector2 { x, y }
}

pub fn straight_line(p1: Vector2, p2: Vector2, p3: Vector2) -> bool {
    // yes, osu!stable will compare the floats directly like this, basically being ineffective
    (p2.x - p1.x) * (p3.y - p1.y) - (p3.x - p1.x) * (p2.y - p1.y) == 0.0
}

fn circle_t_at(pt: Vector2, center: Vector2) -> f64 {
    (pt.y - center.y).atan2(pt.x - center.x) as f64
}

pub fn circle_point(center: Vector2, radius: f32, t: f64) -> Vector2 {
    Vector2::new(
        (t.cos() * radius as f64) as f32,
        (t.sin() * radius as f64) as f32,
    ) + center
}

pub fn circle_through_points(a: Vector2, b: Vector2, c: Vector2) -> (Vector2, f32, f64, f64) {
    let diameter = 2.0 * (a.x * (b.y - c.y) + b.x * (c.y - a.y) + c.x * (a.y - b.y));
    let a_len = a.length_squared();
    let b_len = b.length_squared();
    let c_len = c.length_squared();

    let center = Vector2::new(
        (a_len * (b.y - c.y) + b_len * (c.y - a.y) + c_len * (a.y - b.y)) / diameter,
        (a_len * (c.x - b.x) + b_len * (a.x - c.x) + c_len * (b.x - a.x)) / diameter,
    );
    let radius = center.distance(a);
    let t0 = circle_t_at(a, center);
    let mut t1 = circle_t_at(b, center);
    let mut t2 = circle_t_at(c, center);

    while t1 < t0 {
        t1 += std::f64::consts::TAU;
    }
    while t2 < t0 {
        t2 += std::f64::consts::TAU;
    }
    if t1 > t2 {
        t2 -= std::f64::consts::TAU;
    }

    (center, radius, t0, t2)
}

// required for backwards compatibility to maps before version 10
// strangely, it looks like lazer doesn't have this?
pub fn broken_bezier(points: &[Vector2]) -> Vec<Vector2> {
    let interp_len = points.len() * 50;
    let mut scratch = vec![Vector2::default(); points.len()];
    let mut output = vec![Vector2::default(); interp_len];

    #[allow(clippy::needless_range_loop)]
    for i in 0..interp_len {
        scratch[..].clone_from_slice(points);
        for k in 0..points.len() {
            for l in 0..(points.len() - k - 1) {
                scratch[l] = lerp(scratch[l], scratch[l + 1], i as f32 / interp_len as f32);
            }
        }
        output[i] = scratch[0];
    }

    output
}

// bezier curve stuff
// direct port from osu-stable
pub fn bezier(points: Vec<Vector2>) -> Vec<Vector2> {
    let mut output = Vec::<Vector2>::new();
    if points.is_empty() {
        return output;
    }

    // TODO: allocation optimization
    let last_point = points[points.len() - 1];
    let mut stack = Vec::<Vec<Vector2>>::new();
    let mut stack2 = Vec::<Vec<Vector2>>::new();
    let points_len = points.len();
    let mut subdiv_buf1 = vec![Vector2::default(); points_len];
    let mut subdiv_buf2 = vec![Vector2::default(); points_len * 2 - 1];
    stack.push(points);
    while !stack.is_empty() {
        let mut pop = stack.pop().unwrap();
        if flat_enough(&pop) {
            approximate(&pop, &mut output, &mut subdiv_buf2, &mut subdiv_buf1);
            stack2.push(pop);
            continue;
        }
        let mut pop2 = stack2
            .pop()
            .unwrap_or_else(|| vec![Vector2::default(); points_len]);
        subdivide(&pop, &mut subdiv_buf2, Some(&mut pop2), &mut subdiv_buf1);
        pop[..].clone_from_slice(&subdiv_buf2[..points_len]);
        stack.push(pop2);
        stack.push(pop);
    }
    output.push(last_point);

    output
}
fn flat_enough(points: &[Vector2]) -> bool {
    for i in 1..(points.len() - 1) {
        if (points[i - 1] - points[i] * 2f32 + points[i + 1]).length_squared() > 0.25f32 {
            return false;
        }
    }
    true
}
fn subdivide(
    points: &[Vector2],
    l: &mut [Vector2],
    mut r: Option<&mut [Vector2]>,
    scratch: &mut [Vector2],
) {
    scratch[..].clone_from_slice(points);
    for j in 0..points.len() {
        l[j] = scratch[0];
        if let Some(ref mut r) = r {
            r[points.len() - j - 1] = scratch[points.len() - j - 1];
        }
        for k in 0..(points.len() - j - 1) {
            scratch[k] = (scratch[k] + scratch[k + 1]) / 2f32;
        }
    }
}
fn approximate(
    points: &[Vector2],
    output: &mut Vec<Vector2>,
    scratch1: &mut [Vector2],
    scratch2: &mut [Vector2],
) {
    subdivide(points, scratch1, None, scratch2);
    // TODO: turn this into a proper copy operation
    for i in 0..(points.len() - 1) {
        scratch1[points.len() + i] = scratch2[i + 1];
    }
    output.push(points[0]);
    for j in 1..(points.len() - 1) {
        let num = j * 2;
        output.push((scratch1[num - 1] + scratch1[num] * 2.0 + scratch1[num + 1]) * 0.25);
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn test_bezier() {
        use crate::math::*;
        // generated from decompiled osu-stable code
        let expected = [
            (0.0, 0.0),
            (0.078125, 1.19140625),
            (0.3125, 2.265625),
            (0.703125, 3.22265625),
            (1.25, 4.0625),
            (1.953125, 4.78515625),
            (2.8125, 5.390625),
            (3.828125, 5.87890625),
            (5.0, 6.25),
            (6.328125, 6.50390625),
            (7.8125, 6.640625),
            (9.453125, 6.66015625),
            (11.25, 6.5625),
            (13.203125, 6.34765625),
            (15.3125, 6.015625),
            (17.578125, 5.56640625),
            (20.0, 5.0),
        ];
        // same test points as libosu, but their expected result has many more points
        let control_points = vec![
            Vector2::new(0.0, 0.0),
            Vector2::new(0.0, 10.0),
            Vector2::new(20.0, 5.0),
        ];
        let bezier = bezier(control_points);
        assert_eq!(bezier.len(), expected.len());
        for (a, b) in bezier.iter().zip(expected.iter()) {
            /*
            assert_eq!((a.x - b.0).abs() < 0.01, true);
            assert_eq!((a.y - b.1).abs() < 0.01, true);
            */
            assert_eq!(a.x, b.0);
            assert_eq!(a.y, b.1);
        }
    }
}

#[derive(Clone, Copy)]
pub struct Rect {
    pub left: u32,
    pub top: u32,
    pub right: u32,
    pub bottom: u32,
}

impl Rect {
    pub fn new(left: u32, top: u32, right: u32, bottom: u32) -> Rect {
        Rect {
            left,
            top,
            right,
            bottom,
        }
    }

    pub fn width(&self) -> u32 {
        self.right.saturating_sub(self.left)
    }

    pub fn height(&self) -> u32 {
        self.bottom.saturating_sub(self.top)
    }

    pub fn area(&self) -> u32 {
        self.width() * self.height()
    }

    pub fn is_empty(&self) -> bool {
        self.area() == 0
    }
}
