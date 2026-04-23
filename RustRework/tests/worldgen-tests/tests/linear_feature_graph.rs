mod rustrework_worldgen {
    pub use worldgen_core::*;
}

use rustrework_worldgen::{FeatureKind, LinearFeatureGraph, WorldCellCoordinate};

#[test]
fn rasterize_segment_creates_expected_diagonal_points() {
    let graph = LinearFeatureGraph::create_line(
        FeatureKind::River,
        WorldCellCoordinate { x: 0, y: 0, z: 0 },
        WorldCellCoordinate { x: 3, y: 3, z: 0 },
        false,
    );

    let points = rustrework_worldgen::feature_graph::FeatureGraph::rasterize(&graph);

    assert_eq!(points.len(), 4);
    assert!(points.contains(&WorldCellCoordinate { x: 0, y: 0, z: 0 }));
    assert!(points.contains(&WorldCellCoordinate { x: 1, y: 1, z: 0 }));
    assert!(points.contains(&WorldCellCoordinate { x: 2, y: 2, z: 0 }));
    assert!(points.contains(&WorldCellCoordinate { x: 3, y: 3, z: 0 }));
}
