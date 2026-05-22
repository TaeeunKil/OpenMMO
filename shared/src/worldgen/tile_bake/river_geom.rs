//! Shared river-width derivation: flow → baked width, plus the mouth-fan
//! curve used by the renderer/per-pixel width math. Note that bake-time
//! polylines are NOT widened by the fan factor — the tile baker splits each
//! mouth into distributary branches instead
//! (`context::apply_mouth_distributaries`). The road-A* / settlement
//! "wide-river" gate therefore flags the delta region (within
//! `RIVER_MOUTH_FAN_ARC_CELLS` of a sea-bound mouth) directly as impassable,
//! and elsewhere predicts visible width from the natural polyline width
//! without folding the fan back in.

use super::super::global_map::GlobalMap;
use super::super::grid::fold_x_delta_f32;
use super::super::rivers::RiverMap;
use super::constants::{
    RIVER_CARVE_TAPER_EXTRA_M, RIVER_CARVE_TAPER_MIN_M, RIVER_MAX_WIDTH_M, RIVER_MIN_WIDTH_M,
    RIVER_MOUTH_FAN_EXTRA, RIVER_MOUTH_FAN_SHARPNESS,
};

pub use super::constants::RIVER_MOUTH_FAN_ARC_CELLS;

/// Hard cap (visible water meters — `baked_width + 2 × carve_taper`) above
/// which no bridge is placed and road A* refuses to cross. Matches the wide
/// bridge model's deck length so the deck ends always land on solid bank
/// past the river's depth-fade contour.
pub const BRIDGE_MAX_VISIBLE_WIDTH_M: f32 = 28.0;

/// Predicted visible water width (m) at a river vertex: baked channel width
/// plus 2× the natural-reach carve taper that grows with `flow_norm`. Tracks
/// per-vertex flow so a medium river at `flow_norm ≈ 0.5` reads as ~18 m
/// visible instead of the worst-case ~28 m the old constant-threshold gate
/// assumed — that conservatism over-detoured every fork above ~78 % flow,
/// including ordinary inland rivers a wide bridge can clearly span.
///
/// Note this **ignores the mouth-fan multiplier**: bake-time replaces the
/// post-apex tail of every sea-bound river with several narrower
/// distributary branches (see `context::apply_mouth_distributaries`), so
/// the wide single-channel envelope `mouth_fan_factor` describes never
/// actually gets carved. Folding the fan into this gate over-detours roads
/// around every delta mouth even though each distributary stays narrower
/// than the bridge model can span.
///
/// Distributary branches use the steeper but shorter
/// [`RIVER_MOUTH_BRANCH_TAPER_M`] (5 m fixed) than this natural formula
/// gives, so for branches this remains an upper bound; bake-time
/// `segment_carve_taper_at` resolves the true width when the bridge is
/// placed.
#[inline]
pub fn predicted_visible_width(raw_flow: f32, inv_log_max: f32) -> f32 {
    let norm = if inv_log_max <= 0.0 {
        0.0
    } else {
        (raw_flow.max(1.0).log2() * inv_log_max).clamp(0.0, 1.0)
    };
    let baked = RIVER_MIN_WIDTH_M + (RIVER_MAX_WIDTH_M - RIVER_MIN_WIDTH_M) * norm;
    let taper = RIVER_CARVE_TAPER_MIN_M + RIVER_CARVE_TAPER_EXTRA_M * norm;
    baked + 2.0 * taper
}

// Pre-folded mouth-fan curve coefficients. `s(t) = (1/(k·t+1) - 1/(1+k)) · (1+k)/k`
// peaks at 1.0 at the mouth (t=0) and decays to 0 at t=1.
const MOUTH_FAN_K: f32 = RIVER_MOUTH_FAN_SHARPNESS;
const MOUTH_FAN_S_NORM: f32 = (1.0 + MOUTH_FAN_K) / MOUTH_FAN_K;
const MOUTH_FAN_INV_ONE_PLUS_K: f32 = 1.0 / (1.0 + MOUTH_FAN_K);

/// Pre-compute `1 / log2(max_flow)` once per polyline batch; pass into
/// [`flow_to_width`]. Returns 0 when `max_flow ≤ 1` so the width
/// degrades to `RIVER_MIN_WIDTH_M`.
#[inline]
pub fn flow_log_inv(max_flow: f32) -> f32 {
    if max_flow > 1.0 {
        1.0 / max_flow.log2()
    } else {
        0.0
    }
}

/// Map a raw flow accumulation to baked river width in meters using the
/// log-flow normalization. `inv_log_max` should come from
/// [`flow_log_inv`] hoisted out of the per-vertex loop.
#[inline]
pub fn flow_to_width(raw: f32, inv_log_max: f32) -> f32 {
    let norm = if inv_log_max <= 0.0 {
        0.0
    } else {
        (raw.max(1.0).log2() * inv_log_max).clamp(0.0, 1.0)
    };
    RIVER_MIN_WIDTH_M + (RIVER_MAX_WIDTH_M - RIVER_MIN_WIDTH_M) * norm
}

/// Mouth-fan multiplicative width boost at normalised arc-distance
/// `t = arc_remaining_to_mouth / arc_window`. `t = 0` at the mouth (peak
/// boost = `1 + EXTRA`), `t ≥ 1` outside the window (factor = 1).
#[inline]
pub fn mouth_fan_factor(t: f32) -> f32 {
    let t = t.clamp(0.0, 1.0);
    let s = (1.0 / (MOUTH_FAN_K * t + 1.0) - MOUTH_FAN_INV_ONE_PLUS_K) * MOUTH_FAN_S_NORM;
    1.0 + RIVER_MOUTH_FAN_EXTRA * s
}

/// Per-cell boolean mask: `true` where any river polyline vertex would
/// either (a) sit in the post-apex delta region of a sea-bound mouth, where
/// `apply_mouth_distributaries` replaces the natural channel with several
/// branches a single bridge can't span coherently, or (b) carry a predicted
/// visible width above [`BRIDGE_MAX_VISIBLE_WIDTH_M`] on a non-delta reach.
/// Mirrors the gate road A* applies in
/// [`crate::worldgen::roads::astar::RiverField`] — both consumers (road
/// pathing and settlement placement) want the same definition of "can't
/// drop a bridge here, must detour upstream".
pub fn wide_river_cell_mask(map: &GlobalMap, river_map: &RiverMap) -> Vec<bool> {
    let res = map.config.global_res as usize;
    let total = res * res;
    let mut out = vec![false; total];
    let res_f = res as f32;
    let inv_log_max = flow_log_inv(river_map.max_flow());

    for poly in &river_map.rivers {
        let pts = &poly.points;
        let n = pts.len();
        if n < 2 {
            continue;
        }
        let lens = polyline_arc_lengths_cells(pts, res_f);
        let total_arc = *lens.last().unwrap();
        let (end_x, end_y) = pts[n - 1];
        let mouth_in_sea = map.land_mask[(end_y as usize) * res + (end_x as usize)] == 0;
        for i in 0..n {
            let (x, y) = pts[i];
            let idx = (y as usize) * res + (x as usize);
            if is_delta_cell(mouth_in_sea, total_arc - lens[i])
                || predicted_visible_width(poly.flow[i], inv_log_max)
                    > BRIDGE_MAX_VISIBLE_WIDTH_M
            {
                out[idx] = true;
            }
        }
    }
    out
}

/// Cumulative arc length (in global cells) along a polyline, with X-wrap
/// folded so seam-crossing pieces measure their on-grid distance instead of
/// wrapping the long way around. `out[0] = 0`, `out[i]` = sum of segment
/// lengths up to and including the i-th vertex.
pub fn polyline_arc_lengths_cells(pts: &[(u32, u32)], res_f: f32) -> Vec<f32> {
    let n = pts.len();
    let mut lens = Vec::with_capacity(n);
    lens.push(0.0f32);
    let mut cumulative = 0.0f32;
    for i in 1..n {
        let (px, py) = pts[i - 1];
        let (qx, qy) = pts[i];
        let dx = fold_x_delta_f32(qx as f32 - px as f32, res_f);
        let dy = qy as f32 - py as f32;
        cumulative += (dx * dx + dy * dy).sqrt();
        lens.push(cumulative);
    }
    lens
}

/// `true` for vertices in the post-apex delta region of a sea-bound river
/// — within [`RIVER_MOUTH_FAN_ARC_CELLS`] arc-cells of the sea mouth.
/// Marks the same cells `context::apply_mouth_distributaries` would slice
/// off the natural polyline and replace with narrower branches at bake time.
/// Both gates use this so the road network detours upstream of the apex
/// rather than threading the multi-branch delta directly.
#[inline]
pub fn is_delta_cell(mouth_in_sea: bool, arc_to_mouth_cells: f32) -> bool {
    mouth_in_sea && arc_to_mouth_cells < RIVER_MOUTH_FAN_ARC_CELLS
}
