use std::collections::BTreeSet;

use crate::domain::{FeatureKind, FeatureSegmentDescriptor, WorldCellCoordinate};

pub trait FeatureGraph {
    fn kind(&self) -> FeatureKind;
    fn nodes(&self) -> &[WorldCellCoordinate];
    fn segments(&self) -> &[FeatureSegmentDescriptor];
    fn rasterize(&self) -> Vec<WorldCellCoordinate>;
}

#[derive(Debug, Clone)]
pub struct LinearFeatureGraph {
    pub kind: FeatureKind,
    pub nodes: Vec<WorldCellCoordinate>,
    pub segments: Vec<FeatureSegmentDescriptor>,
}

impl FeatureGraph for LinearFeatureGraph {
    fn kind(&self) -> FeatureKind {
        self.kind
    }

    fn nodes(&self) -> &[WorldCellCoordinate] {
        &self.nodes
    }

    fn segments(&self) -> &[FeatureSegmentDescriptor] {
        &self.segments
    }

    fn rasterize(&self) -> Vec<WorldCellCoordinate> {
        let mut result = BTreeSet::new();
        for segment in &self.segments {
            for point in Self::rasterize_segment(segment) {
                result.insert(point);
            }
        }
        result.into_iter().collect()
    }
}

impl LinearFeatureGraph {
    pub fn create_line(
        kind: FeatureKind,
        start: WorldCellCoordinate,
        end: WorldCellCoordinate,
        elevated: bool,
    ) -> Self {
        Self {
            kind,
            nodes: vec![start, end],
            segments: vec![FeatureSegmentDescriptor {
                kind,
                start,
                end,
                elevated,
            }],
        }
    }

    pub fn rasterize_segment(segment: &FeatureSegmentDescriptor) -> Vec<WorldCellCoordinate> {
        let mut points = Vec::new();
        let (mut x0, mut y0) = (segment.start.x, segment.start.y);
        let (x1, y1) = (segment.end.x, segment.end.y);
        let dx = (x1 - x0).abs();
        let sx = if x0 < x1 { 1 } else { -1 };
        let dy = -(y1 - y0).abs();
        let sy = if y0 < y1 { 1 } else { -1 };
        let mut error = dx + dy;
        let z = segment.start.z;

        loop {
            points.push(WorldCellCoordinate { x: x0, y: y0, z });
            if x0 == x1 && y0 == y1 {
                break;
            }

            let double_error = 2 * error;
            if double_error >= dy {
                error += dy;
                x0 += sx;
            }
            if double_error <= dx {
                error += dx;
                y0 += sy;
            }
        }

        points
    }
}
