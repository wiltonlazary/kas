// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License in the LICENSE-APACHE file or at:
//     https://www.apache.org/licenses/LICENSE-2.0

//! Geometry data types

#[cfg(feature = "winit")]
use winit::dpi::{PhysicalPosition, PhysicalSize};

/// An `(x, y)` coordinate.
#[derive(Copy, Clone, PartialEq, Eq, Debug, Default)]
pub struct Coord(pub i32, pub i32);

impl Coord {
    /// A coord of `(0, 0)`
    pub const ZERO: Coord = Coord(0, 0);
}

impl From<(i32, i32)> for Coord {
    fn from(coord: (i32, i32)) -> Coord {
        Coord(coord.0, coord.1)
    }
}

impl std::ops::Add for Coord {
    type Output = Self;

    fn add(self, other: Self) -> Self {
        Coord(self.0 + other.0, self.1 + other.1)
    }
}

#[cfg(feature = "winit")]
impl From<PhysicalPosition> for Coord {
    fn from(pos: PhysicalPosition) -> Coord {
        let pos: (i32, i32) = pos.into();
        Coord(pos.0, pos.1)
    }
}

#[cfg(feature = "winit")]
impl From<Coord> for PhysicalPosition {
    fn from(coord: Coord) -> PhysicalPosition {
        (coord.0, coord.1).into()
    }
}

impl std::ops::AddAssign<Size> for Coord {
    fn add_assign(&mut self, rhs: Size) {
        self.0 += rhs.0 as i32;
        self.1 += rhs.1 as i32;
    }
}

/// A `(w, h)` size.
#[derive(Copy, Clone, PartialEq, Eq, Debug, Default)]
pub struct Size(pub u32, pub u32);

impl Size {
    /// A size of `(0, 0)`
    pub const ZERO: Size = Size(0, 0);

    /// Maximum possible size
    // TODO: what value do we want to use here? Note that current layout logic
    // can add together multiple copies of this value.
    pub const MAX: Size = Size(0xFFFF, 0xFFFF);

    /// Uniform size in each dimension
    #[inline]
    pub const fn uniform(v: u32) -> Self {
        Size(v, v)
    }
}

impl From<(u32, u32)> for Size {
    fn from(size: (u32, u32)) -> Size {
        Size(size.0, size.1)
    }
}

#[cfg(feature = "winit")]
impl From<PhysicalSize> for Size {
    fn from(size: PhysicalSize) -> Size {
        let size: (u32, u32) = size.into();
        Size(size.0, size.1)
    }
}

#[cfg(feature = "winit")]
impl From<Size> for PhysicalSize {
    fn from(size: Size) -> PhysicalSize {
        (size.0, size.1).into()
    }
}

impl std::ops::Add for Size {
    type Output = Self;

    fn add(self, other: Self) -> Self {
        Size(self.0 + other.0, self.1 + other.1)
    }
}

impl std::ops::Sub for Size {
    type Output = Self;

    fn sub(self, other: Self) -> Self {
        Size(self.0 - other.0, self.1 - other.1)
    }
}

impl std::ops::AddAssign for Size {
    fn add_assign(&mut self, rhs: Self) {
        self.0 += rhs.0;
        self.1 += rhs.1;
    }
}

impl std::ops::SubAssign for Size {
    fn sub_assign(&mut self, rhs: Self) {
        self.0 -= rhs.0;
        self.1 -= rhs.1;
    }
}

/// A rectangular region.
#[derive(Clone, Copy, Default, Debug)]
pub struct Rect {
    pub pos: Coord,
    pub size: Size, // TODO: more efficient to store pos+size ?
}

impl Rect {
    /// Check whether the given coordinate is contained within this rect
    pub fn contains(&self, c: Coord) -> bool {
        c.0 >= self.pos.0
            && c.0 < self.pos.0 + (self.size.0 as i32)
            && c.1 >= self.pos.1
            && c.1 < self.pos.1 + (self.size.1 as i32)
    }

    /// Get pos as `f32` tuple
    pub fn pos_f32(&self) -> (f32, f32) {
        (self.pos.0 as f32, self.pos.1 as f32)
    }

    /// Get size as `f32` tuple
    pub fn size_f32(&self) -> (f32, f32) {
        (self.size.0 as f32, self.size.1 as f32)
    }
}