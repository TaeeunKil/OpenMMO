//! Per-tile baked river field: pixel-aligned surface elevation + flow
//! direction. The runtime renders one quad per river-bearing tile and
//! derives every visual effect from this field plus the heightmap.
//!
//! Format `RFD1`:
//!
//! ```text
//! header (16 bytes):
//!   bytes  0..4   magic    b"RFD1"
//!   bytes  4..6   u16      version (currently 1)
//!   bytes  6..8   u16      grid_x  (== VERTS_PER_SIDE = 65)
//!   bytes  8..10  u16      grid_z  (== VERTS_PER_SIDE = 65)
//!   bytes 10..16  u8[6]    reserved (zero)
//!
//! per-pixel (4 bytes, row-major over 65×65, X then Z):
//!   bytes  0..2   u16      surfaceY — encoded same as heightmap
//!                          (HEIGHT_BIAS / HEIGHT_STEP). Holds the river
//!                          *water surface* at this world XZ within the
//!                          field's reach, otherwise the natural ground
//!                          (so depth = surfaceY − bedY collapses to 0).
//!   byte   2      i8       flowX — unit downstream direction × 127
//!   byte   3      i8       flowZ — unit downstream direction × 127
//! ```
//!
//! Cross-tile consistency: both tiles touching a seam see the same
//! segment list (filtered with the global `river_margin`), so identical
//! world-XZ pixels produce identical surfaceY/flow values regardless of
//! which tile owns the segment.

use super::super::global_map::GlobalMap;
use super::super::vector_features::{nearest_river_segment, RiverSegment};
use super::constants::{
    HEIGHT_BIAS, HEIGHT_STEP, RIVER_CARVE_DEPTH_EXTRA_M, RIVER_CARVE_DEPTH_MIN_M,
    RIVER_CARVE_MIN_BED_Y_M, RIVER_CARVE_TAPER_EXTRA_M, RIVER_CARVE_TAPER_MIN_M,
    RIVER_DEPTH_OFFSET_M, RIVER_MAX_WIDTH_M, VERTS_PER_SIDE,
};
use super::context::BakeContext;
use super::heightmap::{lerp, sample_natural_height_single};

pub const RIVER_FIELD_BIN_MAGIC: &[u8; 4] = b"RFD1";
pub const RIVER_FIELD_BIN_VERSION: u16 = 1;
const RIVER_FIELD_HEADER_SIZE: usize = 16;
const RIVER_FIELD_PIXEL_SIZE: usize = 4;
const RIVER_FIELD_PAYLOAD_SIZE: usize = VERTS_PER_SIDE * VERTS_PER_SIDE * RIVER_FIELD_PIXEL_SIZE;
const RIVER_FIELD_TOTAL_SIZE: usize = RIVER_FIELD_HEADER_SIZE + RIVER_FIELD_PAYLOAD_SIZE;

/// Pixels farther than this distance from any river segment carry
/// `surfaceY = natural ground` so the runtime depth-fade reads `depth =
/// 0` and skips the river there. Sized to comfortably exceed
/// `max_half_width + max_taper` (≈15 m) plus a safety margin so a
/// fragment at the very edge of the carve still sees a coherent
/// surface — without the margin a single-pixel band at the carve outer
/// edge could land just past the cutoff and drop alpha mid-bank.
const INFLUENCE_RADIUS_M: f32 =
    RIVER_MAX_WIDTH_M * 0.5 + RIVER_CARVE_TAPER_MIN_M + RIVER_CARVE_TAPER_EXTRA_M + 2.0;

/// Bake the per-tile river field. Returns `None` when the tile carries
/// no river segments — caller skips writing a file.
pub fn bake_river_field(
    map: &GlobalMap,
    ctx: &BakeContext,
    heights: &[f32],
    tile_origin_x: f32,
    tile_origin_z: f32,
    river_segs: &[RiverSegment],
) -> Option<Vec<u8>> {
    if river_segs.is_empty() {
        return None;
    }
    let mut out = Vec::with_capacity(RIVER_FIELD_TOTAL_SIZE);
    out.extend_from_slice(RIVER_FIELD_BIN_MAGIC);
    out.extend_from_slice(&RIVER_FIELD_BIN_VERSION.to_le_bytes());
    out.extend_from_slice(&(VERTS_PER_SIDE as u16).to_le_bytes());
    out.extend_from_slice(&(VERTS_PER_SIDE as u16).to_le_bytes());
    out.extend_from_slice(&[0u8; 6]);

    for j in 0..VERTS_PER_SIDE {
        for i in 0..VERTS_PER_SIDE {
            let wx = tile_origin_x + i as f32;
            let wz = tile_origin_z + j as f32;
            let bed_y = heights[j * VERTS_PER_SIDE + i];

            let (surface_y, flow_x, flow_z) = compute_pixel(
                wx,
                wz,
                bed_y,
                map,
                ctx,
                heights,
                tile_origin_x,
                tile_origin_z,
                river_segs,
            );

            let v = ((surface_y + HEIGHT_BIAS) / HEIGHT_STEP)
                .round()
                .clamp(0.0, 65535.0) as u16;
            out.extend_from_slice(&v.to_le_bytes());

            let fx = encode_unit(flow_x);
            let fz = encode_unit(flow_z);
            out.push(fx as u8);
            out.push(fz as u8);
        }
    }
    Some(out)
}

#[inline]
fn encode_unit(v: f32) -> i8 {
    (v.clamp(-1.0, 1.0) * 127.0).round().clamp(-127.0, 127.0) as i8
}

/// Compute the field record for one pixel: river surface elevation +
/// downstream-unit flow direction. Falls back to `(bed_y_pixel, 0, 0)`
/// when the nearest segment is past `INFLUENCE_RADIUS_M` so the runtime
/// depth-fade collapses to zero outside the carve reach.
#[allow(clippy::too_many_arguments)]
fn compute_pixel(
    wx: f32,
    wz: f32,
    bed_y_pixel: f32,
    map: &GlobalMap,
    ctx: &BakeContext,
    heights: &[f32],
    tile_origin_x: f32,
    tile_origin_z: f32,
    river_segs: &[RiverSegment],
) -> (f32, f32, f32) {
    let Some((d, idx, t)) = nearest_river_segment(wx, wz, river_segs) else {
        return (bed_y_pixel, 0.0, 0.0);
    };
    if d > INFLUENCE_RADIUS_M {
        return (bed_y_pixel, 0.0, 0.0);
    }
    let seg = &river_segs[idx];

    let dx = seg.bx - seg.ax;
    let dz = seg.bz - seg.az;
    let len = (dx * dx + dz * dz).sqrt().max(1e-3);
    let flow_x = dx / len;
    let flow_z = dz / len;

    // Surface = carved bed at the centerline projection + runtime offset.
    // For in-tile projections, bilinear-sample the already-baked
    // (post-carve) `heights` directly — at the centerline the carve has
    // applied the full depth, so the post-carve sample IS the bed.
    // For projections that escape the tile, fall back to natural sampling
    // + the carve formula (rebuilds bed = natural − capped_depth).
    let proj_x = lerp(seg.ax, seg.bx, t);
    let proj_z = lerp(seg.az, seg.bz, t);
    let bed_at_proj = sample_in_tile(heights, tile_origin_x, tile_origin_z, proj_x, proj_z)
        .unwrap_or_else(|| {
            let natural = sample_natural_height_single(map, ctx, proj_x, proj_z);
            let flow_norm = lerp(seg.flow_norm_a, seg.flow_norm_b, t);
            let depth = RIVER_CARVE_DEPTH_MIN_M + RIVER_CARVE_DEPTH_EXTRA_M * flow_norm;
            let capped = depth.min((natural - RIVER_CARVE_MIN_BED_Y_M).max(0.0));
            natural - capped
        });
    let surface_y = bed_at_proj + RIVER_DEPTH_OFFSET_M;

    (surface_y, flow_x, flow_z)
}

/// Bilinear-sample the local `heights` array at world `(wx, wz)`. Returns
/// `None` when the position lies outside the tile — caller falls back
/// to a global sampler.
#[inline]
fn sample_in_tile(
    heights: &[f32],
    tile_origin_x: f32,
    tile_origin_z: f32,
    wx: f32,
    wz: f32,
) -> Option<f32> {
    let lx = wx - tile_origin_x;
    let lz = wz - tile_origin_z;
    let max = (VERTS_PER_SIDE - 1) as f32;
    if lx < 0.0 || lz < 0.0 || lx > max || lz > max {
        return None;
    }
    let i0 = lx.floor() as usize;
    let j0 = lz.floor() as usize;
    let fx = lx - i0 as f32;
    let fz = lz - j0 as f32;
    let i1 = (i0 + 1).min(VERTS_PER_SIDE - 1);
    let j1 = (j0 + 1).min(VERTS_PER_SIDE - 1);
    let h00 = heights[j0 * VERTS_PER_SIDE + i0];
    let h10 = heights[j0 * VERTS_PER_SIDE + i1];
    let h01 = heights[j1 * VERTS_PER_SIDE + i0];
    let h11 = heights[j1 * VERTS_PER_SIDE + i1];
    let h0 = h00 * (1.0 - fx) + h10 * fx;
    let h1 = h01 * (1.0 - fx) + h11 * fx;
    Some(h0 * (1.0 - fz) + h1 * fz)
}

#[cfg(test)]
mod tests {
    use super::super::super::vector_features::RiverSegment;
    use super::*;

    fn fake_segments() -> Vec<RiverSegment> {
        vec![RiverSegment {
            ax: -10.0,
            az: 0.0,
            bx: 10.0,
            bz: 0.0,
            flow_norm_a: 0.5,
            flow_norm_b: 0.5,
            width_a: 4.0,
            width_b: 4.0,
        }]
    }

    fn small_test_ctx() -> (
        crate::worldgen::global_map::GlobalMap,
        crate::worldgen::tile_bake::BakeContext,
    ) {
        // Tiny world keeps the test fast.
        let cfg = crate::worldgen::config::WorldGenConfig {
            seed: 7,
            world_size_m: 256,
            global_res: 32,
            ..Default::default()
        };
        let mut map = crate::worldgen::continent::generate_continent_mask(&cfg);
        crate::worldgen::elevation::generate_elevation(&mut map);
        let rm = crate::worldgen::rivers::compute_flow(&map);
        let net = crate::worldgen::roads::compute_roads(&map, &[], &rm);
        let coast =
            crate::worldgen::coasts::extract_coasts(&map.land_mask, map.config.global_res as usize);
        let ctx = BakeContext::new(&map, &rm, &net, &coast);
        (map, ctx)
    }

    #[test]
    fn empty_segments_returns_none() {
        let (map, ctx) = small_test_ctx();
        let heights = vec![0.0f32; VERTS_PER_SIDE * VERTS_PER_SIDE];
        let bin = bake_river_field(&map, &ctx, &heights, 0.0, 0.0, &[]);
        assert!(bin.is_none());
    }

    #[test]
    fn encode_unit_round_trip() {
        // Linear quantization to i8 must clip cleanly at ±1, preserve sign,
        // and round to the nearest integer step.
        assert_eq!(encode_unit(0.0), 0);
        assert_eq!(encode_unit(1.0), 127);
        assert_eq!(encode_unit(-1.0), -127);
        assert_eq!(encode_unit(2.0), 127);
        assert_eq!(encode_unit(-2.0), -127);
        // 0.5 × 127 = 63.5 → rounds to 64.
        assert_eq!(encode_unit(0.5), 64);
    }

    #[test]
    fn binary_size_matches_layout() {
        // Pin the on-disk layout — runtime decoders hard-code these offsets
        // and any drift would silently corrupt every loader.
        let (map, ctx) = small_test_ctx();
        let heights = vec![5.0f32; VERTS_PER_SIDE * VERTS_PER_SIDE];
        let segs = fake_segments();
        let bin = bake_river_field(&map, &ctx, &heights, -32.0, -32.0, &segs)
            .expect("non-empty segments produce a file");
        assert_eq!(bin.len(), RIVER_FIELD_TOTAL_SIZE);
        assert_eq!(&bin[0..4], RIVER_FIELD_BIN_MAGIC);
        assert_eq!(
            u16::from_le_bytes([bin[4], bin[5]]),
            RIVER_FIELD_BIN_VERSION
        );
        assert_eq!(u16::from_le_bytes([bin[6], bin[7]]), VERTS_PER_SIDE as u16);
        assert_eq!(u16::from_le_bytes([bin[8], bin[9]]), VERTS_PER_SIDE as u16);
    }

    #[test]
    fn pixel_far_from_river_falls_back_to_bed() {
        // A pixel at influence_radius + 50 m carries surfaceY = bed_y so
        // the runtime depth-fade reads depth = 0 there.
        let cfg = crate::worldgen::config::WorldGenConfig {
            seed: 7,
            world_size_m: 1024,
            global_res: 64,
            ..Default::default()
        };
        let mut map = crate::worldgen::continent::generate_continent_mask(&cfg);
        crate::worldgen::elevation::generate_elevation(&mut map);
        let rm = crate::worldgen::rivers::compute_flow(&map);
        let net = crate::worldgen::roads::compute_roads(&map, &[], &rm);
        let coast =
            crate::worldgen::coasts::extract_coasts(&map.land_mask, map.config.global_res as usize);
        let ctx = BakeContext::new(&map, &rm, &net, &coast);
        let heights = vec![5.0f32; VERTS_PER_SIDE * VERTS_PER_SIDE];
        let segs = vec![RiverSegment {
            ax: 300.0,
            az: 300.0,
            bx: 320.0,
            bz: 300.0,
            flow_norm_a: 0.5,
            flow_norm_b: 0.5,
            width_a: 4.0,
            width_b: 4.0,
        }];
        let bin = bake_river_field(&map, &ctx, &heights, -32.0, -32.0, &segs)
            .expect("segment present, file is written");

        let off = RIVER_FIELD_HEADER_SIZE;
        let surface = u16::from_le_bytes([bin[off], bin[off + 1]]);
        let surface_m = surface as f32 * HEIGHT_STEP - HEIGHT_BIAS;
        assert!(
            (surface_m - 5.0).abs() < 0.05,
            "far pixel surfaceY should match bed (5.0 m), got {surface_m}"
        );
        assert_eq!(bin[off + 2] as i8, 0, "far pixel flowX should be 0");
        assert_eq!(bin[off + 3] as i8, 0, "far pixel flowZ should be 0");
    }
}
