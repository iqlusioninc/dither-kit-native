//! The display-list format every port renders.
//!
//! A program is a flat `f32` array:
//!
//! ```text
//! [P, op, len, args..., op, len, args..., <tables>]
//! ```
//!
//! `P` is the number of floats occupied by ops. Ops start at index 1; each
//! op begins with its opcode and its total length (including the two header
//! floats), so an interpreter can skip ops it does not know. Tables (column
//! geometry, pie slices, polygon points) follow the ops; ops refer to them
//! by an offset relative to the table base `1 + P`.
//!
//! Ops are painted in order and later ops overwrite earlier ones. A `BLOOM`
//! op splits the program into a *base* list (before it) and an *overlay*
//! list (after it): bloom is applied to the base result, then the overlay is
//! painted on top. See `spec/SPEC.md` for the per-op pixel rules.

use crate::color::{Rgba, Tone};

pub mod op {
    pub const LINE: u32 = 1;
    pub const DOT: u32 = 2;
    pub const AREA_BODY: u32 = 3;
    pub const AREA_STROKE: u32 = 4;
    pub const BAR: u32 = 5;
    pub const PIE: u32 = 6;
    pub const POLY: u32 = 7;
    pub const GRADIENT: u32 = 8;
    pub const BACKGROUND: u32 = 9;
    pub const BUTTON: u32 = 10;
    pub const METER: u32 = 11;
    pub const AVATAR: u32 = 12;
    pub const SWATCH: u32 = 13;
    pub const BLOOM: u32 = 14;
}

/// Sentinel stored in column tables for "no geometry in this column".
pub const NONE: f32 = -1.0e9;

/// First bloom operation, or the operation-list end; malformed lists return None.
pub fn bloom_index(data: &[f32]) -> Option<usize> {
    let &count = data.first()?;
    if !count.is_finite() || count < 0.0 || count.fract() != 0.0 || count as usize >= data.len() {
        return None;
    }
    let end = 1 + count as usize;
    let (mut index, mut visited, mut split) = (1, 0, end);
    while index < end {
        if index + 1 >= end {
            return None;
        }
        let length = data[index + 1];
        if !length.is_finite()
            || length < 2.0
            || length.fract() != 0.0
            || length as usize > end - index
        {
            return None;
        }
        // Match shader int conversion and its bounded walk, including unknown ops.
        if split == end && visited < 65_536 && data[index] >= 14.0 && data[index] < 15.0 {
            split = index;
        }
        index += length as usize;
        visited += 1;
    }
    Some(split)
}

#[cfg(test)]
mod metadata_tests {
    use super::bloom_index;

    #[test]
    fn finds_first_bloom_including_radius_zero_and_ignores_tables() {
        assert_eq!(bloom_index(&[0.0]), Some(1));
        assert_eq!(bloom_index(&[2.0, 99.0, 2.0, 14.0, 4.0]), Some(3));
        assert_eq!(bloom_index(&[4.0, 14.0, 4.0, 0.0, 0.0]), Some(1));
        assert_eq!(bloom_index(&[4.0, 14.5, 4.0, 0.0, 0.0]), Some(1));
        assert_eq!(bloom_index(&[4.0, 15.0, 4.0, 0.0, 0.0]), Some(5));
        assert_eq!(
            bloom_index(&[10.0, 99.0, 2.0, 14.0, 4.0, 0.0, 0.0, 14.0, 4.0, 1.0, 0.5]),
            Some(3)
        );
    }

    #[test]
    fn metadata_respects_the_shader_walk_limit() {
        for preceding in [65_535, 65_536] {
            let mut data = vec![(preceding * 2 + 4) as f32];
            for _ in 0..preceding {
                data.extend([99.0, 2.0]);
            }
            let split = data.len();
            data.extend([14.0, 4.0, 0.0, 0.0]);
            assert_eq!(
                bloom_index(&data),
                Some(if preceding < 65_536 {
                    split
                } else {
                    data.len()
                })
            );
        }
    }

    #[test]
    fn malformed_metadata_never_advances_outside_the_operation_list() {
        for data in [
            vec![],
            vec![f32::NAN],
            vec![f32::INFINITY],
            vec![-1.0],
            vec![1.0, 99.0],
            vec![2.0, 99.0, 0.0],
            vec![2.0, 99.0, 2.5],
            vec![6.0, 14.0, 4.0, 0.0, 0.0, 99.0, 0.0],
        ] {
            assert_eq!(bloom_index(&data), None, "{data:?}");
        }
    }
}

/// A display list under construction.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Program {
    ops: Vec<f32>,
    tables: Vec<f32>,
}

impl Program {
    pub fn new() -> Self {
        Self::default()
    }

    /// Appends an op with the given arguments.
    pub fn push(&mut self, opcode: u32, args: &[f32]) {
        self.ops.push(opcode as f32);
        self.ops.push((args.len() + 2) as f32);
        self.ops.extend_from_slice(args);
    }

    /// Appends a table and returns its offset relative to the table base.
    pub fn table(&mut self, values: &[f32]) -> f32 {
        let at = self.tables.len();
        self.tables.extend_from_slice(values);
        at as f32
    }

    pub fn is_empty(&self) -> bool {
        self.ops.is_empty()
    }

    /// The flat array handed to shaders.
    pub fn to_vec(&self) -> Vec<f32> {
        let mut v = Vec::with_capacity(1 + self.ops.len() + self.tables.len());
        v.push(self.ops.len() as f32);
        v.extend_from_slice(&self.ops);
        v.extend_from_slice(&self.tables);
        v
    }

    // Typed helpers used by the encoders.

    pub fn line(&mut self, (x0, y0): (i32, i32), (x1, y1): (i32, i32), color: Rgba, dash: u32) {
        let mut a = vec![x0 as f32, y0 as f32, x1 as f32, y1 as f32, dash as f32];
        a.extend(rgba(color));
        self.push(op::LINE, &a);
    }

    pub fn dot(&mut self, x: i32, y: i32, ring: Rgba, center: Rgba) {
        let mut a = vec![x as f32, y as f32];
        a.extend(rgba(ring));
        a.extend(rgba(center));
        self.push(op::DOT, &a);
    }

    pub fn bloom(&mut self, radius: i32, strength: f32) {
        self.push(op::BLOOM, &[radius as f32, strength]);
    }
}

pub fn rgba(c: Rgba) -> [f32; 4] {
    [c.r as f32, c.g as f32, c.b as f32, c.a as f32]
}

pub fn tone(t: Tone) -> [f32; 2] {
    [t.hue(), t.chroma()]
}
