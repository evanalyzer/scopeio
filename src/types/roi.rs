//! # roi
//!
//! **Author:** Joachim Danmayr
//! **Date:** 2026-10-24
//!
//! ## License
//! Copyright 2026 Joachim Danmayr.
//! Licensed under the **AGPL-3.0-only** with the EVAnalyzer exception.
//! See LICENSE and LICENSE-EXCEPTION.

/// Region of interest consisting of one or more shapes.
#[derive(Debug, Clone, PartialEq, Default)]
#[non_exhaustive]
pub struct Roi {
    pub name: Option<String>,
    pub shapes: Vec<Shape>,
}

/// One shape of a [`Roi`].
///
/// Coordinates are in pixels of level 0. A plane coordinate of `None`
/// means the shape applies to all values of that dimension.
#[derive(Debug, Clone, PartialEq)]
#[non_exhaustive]
pub struct Shape {
    pub geometry: Geometry,
    pub label: Option<String>,
    pub c: Option<u32>,
    pub z: Option<u32>,
    pub t: Option<u32>,
}

/// Geometry of a [`Shape`]. Points are `[x, y]`.
#[derive(Debug, Clone, PartialEq)]
#[non_exhaustive]
pub enum Geometry {
    Point([f64; 2]),
    Line([f64; 2], [f64; 2]),
    Rectangle { x: f64, y: f64, width: f64, height: f64 },
    Ellipse { center: [f64; 2], radius_x: f64, radius_y: f64 },
    Polygon(Vec<[f64; 2]>),
    Polyline(Vec<[f64; 2]>),
}
