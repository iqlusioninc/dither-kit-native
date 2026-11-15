//! Reference interpreter for display-list programs.
//!
//! This is the executable form of `spec/SPEC.md`. It is written as plain
//! per-pixel scalar functions so that the GLSL, Metal and HLSL shaders can
//! mirror it line by line, and it is tested pixel-for-pixel against
//! `dither-core`.

use crate::color::{Rgba, shade_hc};
use crate::program::{NONE, op};

const BAYER2: [u32; 4] = [0, 2, 3, 1];
const BAYER4: [u32; 16] = [0, 8, 2, 10, 12, 4, 14, 6, 3, 11, 1, 9, 15, 7, 13, 5];
const BAYER8: [u32; 64] = [
    0, 32, 8, 40, 2, 34, 10, 42, 48, 16, 56, 24, 50, 18, 58, 26, 12, 44, 4, 36, 14, 46, 6, 38, 60, 28, 52, 20,
    62, 30, 54, 22, 3, 35, 11, 43, 1, 33, 9, 41, 51, 19, 59, 27, 49, 17, 57, 25, 15, 47, 7, 39, 13, 45, 5, 37,
    63, 31, 55, 23, 61, 29, 53, 21,
];
const HALFTONE4: [u32; 16] = [12, 5, 6, 13, 4, 0, 1, 7, 11, 3, 2, 8, 15, 10, 9, 14];

const TAU: f32 = std::f32::consts::TAU;

/// Matrix ids, as encoded in programs.
pub const M_BAYER2: i32 = 0;
pub const M_BAYER4: i32 = 1;
pub const M_BAYER8: i32 = 2;
pub const M_HALFTONE: i32 = 3;
pub const M_NOISE: i32 = 4;

pub fn hash2(x: i32, y: i32) -> u32 {
    let mut h = (x as u32).wrapping_mul(0x9E37_79B1) ^ (y as u32).wrapping_mul(0x85EB_CA77);
    h ^= h >> 15;
    h = h.wrapping_mul(0x2C1B_3C6D);
    h ^= h >> 12;
    h = h.wrapping_mul(0x297A_2D39);
    h ^ (h >> 15)
}

pub fn hash01(x: i32, y: i32) -> f32 {
    hash2(x, y) as f32 / u32::MAX as f32
}

pub fn threshold(m: i32, x: i32, y: i32) -> f32 {
    let (xu, yu) = (x.rem_euclid(8) as usize, y.rem_euclid(8) as usize);
    match m {
        M_BAYER2 => (BAYER2[(yu % 2) * 2 + xu % 2] as f32 + 0.5) / 4.0,
        M_BAYER4 => (BAYER4[(yu % 4) * 4 + xu % 4] as f32 + 0.5) / 16.0,
        M_HALFTONE => (HALFTONE4[(yu % 4) * 4 + xu % 4] as f32 + 0.5) / 16.0,
        M_NOISE => (hash2(x, y) as f32 + 0.5) / (u32::MAX as f32 + 1.0),
        _ => (BAYER8[yu * 8 + xu] as f32 + 0.5) / 64.0,
    }
}

pub fn on(m: i32, x: i32, y: i32, i: f32) -> bool {
    i > threshold(m, x, y)
}

/// Fill ids: 0 gradient, 1 dotted, 2 hatched, 3 solid.
pub fn covers(fill: i32, x: i32, y: i32, i: f32) -> bool {
    match fill {
        1 => {
            x.rem_euclid(2) == 0
                && (y + (x / 2).rem_euclid(2)).rem_euclid(2) == 0
                && on(M_BAYER4, x / 2, y / 2, i * 1.4)
        }
        2 => (x + y).rem_euclid(3) == 0 && on(M_BAYER4, x, y, 0.25 + i),
        3 => i > 0.02,
        _ => on(M_BAYER8, x, y, i),
    }
}

/// Rust's `f32::round`: half away from zero.
pub fn rnd(v: f32) -> f32 {
    v.signum() * (v.abs() + 0.5).floor()
}

fn fade(c: Rgba, f: f32) -> Rgba {
    c.fade(f)
}

fn shade(h: f32, c: f32, level: f32) -> Rgba {
    shade_hc(h, c, level)
}

fn color_at(p: &[f32], i: usize) -> Rgba {
    Rgba::new(p[i] as u8, p[i + 1] as u8, p[i + 2] as u8, p[i + 3] as u8)
}

/// Step index of `(px, py)` on the Bresenham line `a -> b`, if it lies on it.
pub fn line_step(x0: i32, y0: i32, x1: i32, y1: i32, px: i32, py: i32) -> Option<i32> {
    let (adx, ady) = ((x1 - x0).abs(), (y1 - y0).abs());
    let (sx, sy) = (if x0 < x1 { 1 } else { -1 }, if y0 < y1 { 1 } else { -1 });
    if adx >= ady {
        let k = (px - x0) * sx;
        if k < 0 || k > adx {
            return None;
        }
        let m = if adx == 0 { 0 } else { (2 * k * ady + adx) / (2 * adx) };
        (py == y0 + sy * m).then_some(k)
    } else {
        let k = (py - y0) * sy;
        if k < 0 || k > ady {
            return None;
        }
        let m = (2 * k * adx + ady) / (2 * ady);
        (px == x0 + sx * m).then_some(k)
    }
}

fn smoothstep(a: f32, b: f32, x: f32) -> f32 {
    let t = ((x - a) / (b - a)).clamp(0.0, 1.0);
    t * t * (3.0 - 2.0 * t)
}

fn value_noise(x: f32, y: f32) -> f32 {
    let (xi, yi) = (x.floor() as i32, y.floor() as i32);
    let (fx, fy) = (x - x.floor(), y - y.floor());
    let sx = fx * fx * (3.0 - 2.0 * fx);
    let sy = fy * fy * (3.0 - 2.0 * fy);
    let n00 = hash01(xi, yi);
    let n10 = hash01(xi + 1, yi);
    let n01 = hash01(xi, yi + 1);
    let n11 = hash01(xi + 1, yi + 1);
    let top = n00 + (n10 - n00) * sx;
    let bot = n01 + (n11 - n01) * sx;
    top + (bot - top) * sy
}

pub fn field(f: i32, u: f32, v: f32, t: f32) -> f32 {
    let s = match f {
        1 => {
            let crest = 0.5 + 0.18 * (u * 6.0 + t * 1.1).sin() + 0.08 * (u * 15.0 - t * 1.7).sin();
            (1.0 - (v - crest).abs() * 2.6).clamp(0.0, 1.0) * 0.9 + v * 0.15
        }
        2 => {
            let d = ((u - 0.5) * (u - 0.5) + (v - 0.5) * (v - 0.5)).sqrt();
            0.5 + 0.5 * (d * 30.0 - t * 3.0).sin() * (1.0 - d * 1.2).max(0.0)
        }
        3 => value_noise(u * 6.0 + t * 0.35, v * 4.0 - t * 0.2) * 0.7 + value_noise(u * 13.0 - t * 0.5, v * 9.0) * 0.3,
        4 => {
            let curtain = value_noise(u * 5.0 + t * 0.25, t * 0.15) * 0.6 + 0.4 * (u * 9.0 + t * 0.6).sin().abs();
            let fall = (1.0 - (v - 0.35 * curtain).max(0.0) * 1.6).clamp(0.0, 1.0);
            curtain * fall
        }
        _ => {
            let a = (u * 7.0 + t * 0.9).sin();
            let b = (v * 6.0 - t * 0.7).sin();
            let c = ((u + v) * 5.0 + t * 0.5).sin();
            let d = (((u - 0.5) * (u - 0.5) + (v - 0.5) * (v - 0.5)).sqrt() * 12.0 - t * 1.2).sin();
            (a + b + c + d) / 8.0 + 0.5
        }
    };
    s.clamp(0.0, 1.0)
}

fn direction(d: i32, x: i32, y: i32, w: i32, h: i32) -> f32 {
    let fx = if w > 1 { x as f32 / (w - 1) as f32 } else { 0.5 };
    let fy = if h > 1 { y as f32 / (h - 1) as f32 } else { 0.5 };
    match d {
        1 => fy,
        2 => 1.0 - fx,
        3 => fx,
        4 => {
            let (dx, dy) = (fx - 0.5, fy - 0.5);
            ((dx * dx + dy * dy).sqrt() / std::f32::consts::FRAC_1_SQRT_2).min(1.0)
        }
        _ => 1.0 - fy,
    }
}

/// Evaluates one op at pixel `(x, y)`. Returns the color it paints, if any.
/// `cur` is the color painted so far (needed by the button's sparkles).
fn eval_op(p: &[f32], i: usize, tb: usize, x: i32, y: i32, cur: Option<Rgba>) -> Option<Rgba> {
    let a = |k: usize| p[i + 2 + k];
    let ai = |k: usize| p[i + 2 + k] as i32;
    match p[i] as u32 {
        op::LINE => {
            let k = line_step(ai(0), ai(1), ai(2), ai(3), x, y)?;
            let dash = ai(4);
            (dash <= 1 || k % dash == 0).then(|| color_at(p, i + 7))
        }
        op::DOT => {
            let (dx, dy) = (x - ai(0), y - ai(1));
            if dx.abs() > 1 || dy.abs() > 1 {
                return None;
            }
            Some(if dx == 0 && dy == 0 { color_at(p, i + 8) } else { color_at(p, i + 4) })
        }
        op::AREA_BODY => {
            let (off, w, h) = (ai(0) as usize, ai(1), ai(2));
            if x >= w {
                return None;
            }
            let top = p[tb + off + x as usize * 2];
            let bottom = p[tb + off + x as usize * 2 + 1];
            if top == NONE {
                return None;
            }
            if y < top.ceil() as i32 + 1 || y > bottom as i32 {
                return None;
            }
            let span = (h - 1) as f32 - top;
            let t = 1.0 - (y as f32 - top) / span.max(1.0);
            covers(ai(3), x, y, (0.1 + 0.9 * t) * a(6)).then(|| fade(shade(a(4), a(5), 0.32 + 0.4 * t), a(7)))
        }
        op::AREA_STROKE => {
            let (off, w) = (ai(0) as usize, ai(1));
            if x >= w {
                return None;
            }
            let top = p[tb + off + x as usize * 2];
            if top == NONE {
                return None;
            }
            let ty = rnd(top) as i32;
            let mut from = ty;
            if x > 0 {
                let prev = p[tb + off + (x as usize - 1) * 2];
                if prev != NONE {
                    from = rnd(prev) as i32;
                }
            }
            (y >= from.min(ty) && y <= from.max(ty)).then(|| color_at(p, i + 4))
        }
        op::BAR => {
            let (bx0, bx1, top, bottom, h) = (ai(0), ai(1), a(2), ai(3), ai(4));
            let ty = rnd(top) as i32;
            if x < bx0 || x > bx1.max(bx0) || y < ty || y > bottom {
                return None;
            }
            let t = 1.0 - (y as f32 - top) / ((h - 1) as f32 - top).max(1.0);
            let lit = y == ty || covers(ai(5), x, y, 0.3 + 0.7 * t);
            let level = if y == ty { 0.82 } else { 0.32 + 0.38 * t };
            lit.then(|| fade(shade(a(6), a(7), level), a(8)))
        }
        op::PIE => {
            let (cx, cy, radius, inner) = (a(0), a(1), a(2), a(3));
            let (multi, hover, n, off) = (ai(4) != 0, ai(5), ai(6), ai(7) as usize);
            let (dx, dy) = (x as f32 + 0.5 - cx, y as f32 + 0.5 - cy);
            let r = (dx * dx + dy * dy).sqrt();
            let ang = dx.atan2(-dy).rem_euclid(TAU);
            let mut idx = -1;
            for s in 0..n {
                let (a0, a1) = (p[tb + off + s as usize * 5], p[tb + off + s as usize * 5 + 1]);
                if ang >= a0 && ang < a1 {
                    idx = s;
                    break;
                }
            }
            if idx < 0 {
                return None;
            }
            let row = tb + off + idx as usize * 5;
            let lift = if hover == idx { 2.0 } else { 0.0 };
            let outer = radius - 2.0 + lift;
            if r < inner || r > outer {
                return None;
            }
            let (a0, a1) = (p[row], p[row + 1]);
            if multi && (ang - a0).min(a1 - ang) * r < 0.9 {
                return None;
            }
            let dim = if hover >= 0 && hover != idx { 0.5 } else { 1.0 };
            let t = ((r - inner) / (outer - inner).max(1.0)).clamp(0.0, 1.0);
            let rim = outer - r < 1.2;
            if rim || covers(p[row + 4] as i32, x, y, 0.3 + 0.7 * t) {
                let level = if rim { 0.8 } else { 0.3 + 0.4 * t };
                Some(fade(shade(p[row + 2], p[row + 3], level), dim))
            } else {
                None
            }
        }
        op::POLY => {
            let (cx, cy, radius, fill) = (a(0), a(1), a(2), ai(3));
            let (n, off) = (ai(7) as usize, ai(8) as usize);
            let (px, py) = (x as f32 + 0.5, y as f32 + 0.5);
            let mut inside = false;
            let mut j = n - 1;
            for k in 0..n {
                let (xi, yi) = (p[tb + off + k * 2], p[tb + off + k * 2 + 1]);
                let (xj, yj) = (p[tb + off + j * 2], p[tb + off + j * 2 + 1]);
                if (yi > py) != (yj > py) && px < (xj - xi) * (py - yi) / (yj - yi) + xi {
                    inside = !inside;
                }
                j = k;
            }
            if !inside {
                return None;
            }
            let d = ((px - cx) * (px - cx) + (py - cy) * (py - cy)).sqrt() / radius;
            covers(fill, x, y, 0.2 + 0.55 * d).then(|| fade(shade(a(4), a(5), 0.32 + 0.35 * d), a(6)))
        }
        op::GRADIENT => {
            let (w, h, dir, m, density) = (ai(0), ai(1), ai(2), ai(3), a(4));
            if x >= w || y >= h {
                return None;
            }
            let pos = direction(dir, x, y, w, h);
            if ai(7) != 0 {
                let (th, tc) = if on(m, x, y, pos) { (a(8), a(9)) } else { (a(5), a(6)) };
                on(m, x + 3, y + 5, density * (0.55 + 0.45 * pos)).then(|| shade(th, tc, 0.35 + 0.35 * pos))
            } else {
                on(m, x, y, (1.0 - pos) * density).then(|| shade(a(5), a(6), 0.3 + 0.4 * (1.0 - pos)))
            }
        }
        op::BACKGROUND => {
            let (w, h, f, m, density, time) = (ai(0), ai(1), ai(2), ai(3), a(4), a(5));
            if x >= w || y >= h {
                return None;
            }
            let (u, v) = (x as f32 / w.max(1) as f32, y as f32 / h.max(1) as f32);
            let it = smoothstep(0.18, 0.82, field(f, u, v, time));
            if !on(m, x, y, it * density) {
                return None;
            }
            let accent = ai(8) != 0 && on(M_BAYER4, x + 1, y + 2, (it - 0.62) * 2.2);
            let (th, tc) = if accent { (a(9), a(10)) } else { (a(6), a(7)) };
            Some(shade(th, tc, 0.3 + 0.5 * it))
        }
        op::BUTTON => {
            let (w, h, style, density, time, th, tc) = (ai(0), ai(1), ai(2), a(3), a(4), a(5), a(6));
            if x >= w || y >= h {
                return None;
            }
            let edge = x == 0 || y == 0 || x + 1 == w || y + 1 == h;
            let mut c = None;
            if edge && style != 2 {
                if (x + y) % 2 == 0 || style == 0 {
                    c = Some(shade(th, tc, 0.6));
                }
            } else {
                let fy = if h > 1 { y as f32 / (h - 1) as f32 } else { 0.5 };
                let it = density * (1.05 - 0.45 * fy);
                if covers(0, x, y, it) {
                    c = Some(shade(th, tc, 0.45 + 0.25 * (1.0 - fy)));
                }
            }
            // Sparkles land on pixels lit so far (this op, or ones below it).
            if density > 0.4 && hash01(x, y ^ 0x5bd1) < 0.04 * density && c.or(cur).is_some() {
                let phase = hash01(y, x ^ 0x1b56) * TAU;
                let speed = 1.5 + 2.5 * hash01(x ^ 0x77, y);
                let wink = (time * speed + phase).sin().max(0.0).powi(12);
                if wink > 0.35 {
                    c = Some(fade(shade(th, tc, 0.97), wink));
                }
            }
            c
        }
        op::METER => {
            let (w, h, value, fill, th, tc) = (ai(0), ai(1), a(2), ai(3), a(4), a(5));
            if x >= w || y >= h {
                return None;
            }
            let filled = rnd(value.clamp(0.0, 1.0) * w as f32) as i32;
            if x < filled {
                let it = 0.35 + 0.65 * (x as f32 + 1.0) / filled.max(1) as f32;
                (covers(fill, x, y, it) || x + 1 == filled).then(|| shade(th, tc, 0.4 + 0.35 * it))
            } else if (x + y) % 4 == 0 {
                Some(fade(shade(260.0, 0.012, 0.35), 0.6))
            } else {
                None
            }
        }
        op::AVATAR => {
            if !(0..8).contains(&x) || !(0..8).contains(&y) {
                return None;
            }
            let bits = (ai(0) as u32) | ((ai(1) as u32) << 16);
            let mirror_x = ai(2) != 0;
            let hue = a(3);
            let acc = (ai(4) as u32) | ((ai(5) as u32) << 16);
            let i = if mirror_x {
                (y * 4 + if x < 4 { x } else { 7 - x }) as u32
            } else {
                ((if y < 4 { y } else { 7 - y }) * 8 + x) as u32
            };
            let lit = (bits >> i) & 1 == 1;
            let accent = lit && (acc >> (i % 23)) & 1 == 1 && (acc >> ((i + 7) % 23)) & 1 == 1;
            let level = if accent {
                0.92
            } else if lit {
                0.62
            } else {
                0.16
            };
            Some(shade(hue, 0.19, level))
        }
        op::SWATCH => Some(shade(a(0), a(1), if (x + y) % 2 == 0 { 0.72 } else { 0.48 })),
        _ => None,
    }
}

/// Paints ops in `[start, end)` over `cur`.
fn eval_list(p: &[f32], start: usize, end: usize, x: i32, y: i32, mut cur: Option<Rgba>) -> Option<Rgba> {
    let tb = 1 + p[0] as usize;
    let mut i = start;
    while i < end {
        if p[i] as u32 == op::BLOOM {
            break;
        }
        if let Some(c) = eval_op(p, i, tb, x, y, cur) {
            cur = Some(c);
        }
        i += p[i + 1] as usize;
    }
    cur
}

/// Index of the BLOOM op, or the end of the op list.
pub fn bloom_index(p: &[f32]) -> usize {
    let end = 1 + p[0] as usize;
    let mut i = 1;
    while i < end {
        if p[i] as u32 == op::BLOOM {
            return i;
        }
        i += p[i + 1] as usize;
    }
    end
}

/// Base pass: every op before BLOOM.
pub fn base(p: &[f32], x: i32, y: i32) -> Option<Rgba> {
    eval_list(p, 1, bloom_index(p), x, y, None)
}

/// Post pass: bloom over `lookup` (the base pass), then the overlay ops.
pub fn post(p: &[f32], x: i32, y: i32, lookup: &dyn Fn(i32, i32) -> Option<Rgba>) -> Option<Rgba> {
    let end = 1 + p[0] as usize;
    let b = bloom_index(p);
    let mut cur = lookup(x, y);
    if b < end {
        let (radius, strength) = (p[b + 2] as i32, p[b + 3]);
        if cur.is_none() && radius > 0 {
            'search: for d in 1..=radius {
                for oy in -d..=d {
                    for ox in -d..=d {
                        if ox.abs() != d && oy.abs() != d {
                            continue;
                        }
                        if let Some(c) = lookup(x + ox, y + oy)
                            && c.a > 160
                        {
                            let glow = strength * (1.0 - (d as f32 - 1.0) / radius as f32);
                            if on(M_BAYER4, x, y, glow) {
                                cur = Some(c.fade(0.35 + 0.25 * glow));
                            }
                            break 'search;
                        }
                    }
                }
            }
        }
        cur = eval_list(p, b + p[b + 1] as usize, end, x, y, cur);
    }
    cur
}

/// Renders a whole program into a `w x h` grid, row-major.
pub fn render(p: &[f32], w: usize, h: usize) -> Vec<Option<Rgba>> {
    let mut basebuf = vec![None; w * h];
    for y in 0..h {
        for x in 0..w {
            basebuf[y * w + x] = base(p, x as i32, y as i32);
        }
    }
    let lookup = |x: i32, y: i32| {
        if x < 0 || y < 0 || x >= w as i32 || y >= h as i32 {
            None
        } else {
            basebuf[y as usize * w + x as usize]
        }
    };
    let mut out = vec![None; w * h];
    for y in 0..h {
        for x in 0..w {
            out[y * w + x] = post(p, x as i32, y as i32, &lookup);
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn bayer_tables_match_recursive_construction() {
        for y in 0..4 {
            for x in 0..4 {
                let inner = BAYER2[(y % 2) * 2 + (x % 2)];
                let outer = BAYER2[(y / 2) * 2 + (x / 2)];
                assert_eq!(BAYER4[y * 4 + x], 4 * inner + outer);
            }
        }
        for y in 0..8 {
            for x in 0..8 {
                let inner = BAYER4[(y % 4) * 4 + (x % 4)];
                let outer = BAYER2[((y / 4) % 2) * 2 + ((x / 4) % 2)];
                assert_eq!(BAYER8[y * 8 + x], 4 * inner + outer);
            }
        }
    }

    fn bresenham(x0: i32, y0: i32, x1: i32, y1: i32) -> Vec<(i32, i32)> {
        let (dx, dy) = ((x1 - x0).abs(), -(y1 - y0).abs());
        let (sx, sy) = (if x0 < x1 { 1 } else { -1 }, if y0 < y1 { 1 } else { -1 });
        let (mut x, mut y, mut err) = (x0, y0, dx + dy);
        let mut v = vec![];
        loop {
            v.push((x, y));
            if x == x1 && y == y1 {
                break;
            }
            let e2 = 2 * err;
            if e2 >= dy {
                err += dy;
                x += sx;
            }
            if e2 <= dx {
                err += dx;
                y += sy;
            }
        }
        v
    }

    #[test]
    fn closed_form_line_matches_bresenham() {
        let r = 9;
        for x0 in -r..=r {
            for y0 in -r..=r {
                for (x1, y1) in [(r, 3), (-4, r), (2, -r), (-r, -r), (0, 5), (7, 0), (x0, y0)] {
                    let pts = bresenham(x0, y0, x1, y1);
                    for (k, &(px, py)) in pts.iter().enumerate() {
                        assert_eq!(line_step(x0, y0, x1, y1, px, py), Some(k as i32));
                    }
                    let n = (-r - 1..=r + 1)
                        .flat_map(|px| (-r - 1..=r + 1).map(move |py| (px, py)))
                        .filter(|&(px, py)| line_step(x0, y0, x1, y1, px, py).is_some())
                        .count();
                    assert_eq!(n, pts.len());
                }
            }
        }
    }
}
