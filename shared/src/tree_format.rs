//! Tree placement and wire-format constants shared by Rust producers/consumers.
//!
//! V1 stores tile-local X/Z, rotation, and scale only. Altitude/Y is
//! intentionally not part of the payload; clients reconstruct it by sampling
//! the tile heightmap at decode time. Keep the TypeScript decoder in
//! `client/src/lib/utils/tree-data.ts` aligned with these constants.

pub const TREE_V1_MAGIC: u32 = 0x5452_3031; // "TR01"
pub const TREE_V1_HEADER_BYTES: usize = 12;
pub const TREE_V1_BYTES_PER_INSTANCE: usize = 6;

/// `[(scale_min, scale_range)]` per type: slot 0 is `tree.glb`, slot 1 is
/// `tree2.glb`. These values define how the V1 scale byte is interpreted, so
/// changing them changes the apparent size of existing baked tree data.
pub const TREE_V1_SCALE: [(f32, f32); 2] = [(0.7, 2.3), (0.6, 0.8)];

/// Base exclusion radius at scale 1.0, per tree type (slot 0 is `tree.glb`,
/// slot 1 is `tree2.glb`). Multiplied by the instance scale at placement time
/// to keep trees clear of housing footprints. Must match
/// `TREE_EXCLUSION_RADIUS` in `tree-data.ts`.
pub const TREE_EXCLUSION_RADIUS: [f32; 2] = [2.0, 1.5];
