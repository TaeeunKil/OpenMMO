//! Shared river-width derivation: flow → baked width and the mouth-fan
//! widening factor. The bake applies these to the world-space polylines
//! at heightmap/splatmap time; road A* uses the same formulas to predict
//! where bridges would land too wide and detour around them.

use super::constants::{
    RIVER_MAX_WIDTH_M, RIVER_MIN_WIDTH_M, RIVER_MOUTH_FAN_EXTRA, RIVER_MOUTH_FAN_SHARPNESS,
};

pub use super::constants::RIVER_MOUTH_FAN_ARC_CELLS;

/// Render-side ribbon adjustments — keep in sync with `river-geometry.ts`'s
/// `RIVER_WIDTH_SCALE` and `RIVER_WIDTH_PAD_M`.
pub const RIVER_RIBBON_WIDTH_SCALE: f32 = 1.5;
pub const RIVER_RIBBON_WIDTH_PAD_M: f32 = 1.0;

/// Hard cap (rendered ribbon meters) above which no bridge is placed and
/// road A* refuses to cross. Wider crossings are visually implausible for
/// the catalog's stone bridge models and almost always sit in the
/// mouth-fan / delta zone.
pub const BRIDGE_MAX_VISIBLE_WIDTH_M: f32 = 29.0;

/// Baked-width equivalent of [`BRIDGE_MAX_VISIBLE_WIDTH_M`], used by
/// callers that only see baked widths (road A* prediction, before the
/// ribbon expansion).
pub const BRIDGE_MAX_BAKED_WIDTH_M: f32 =
    (BRIDGE_MAX_VISIBLE_WIDTH_M - RIVER_RIBBON_WIDTH_PAD_M * 2.0) / RIVER_RIBBON_WIDTH_SCALE;

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

/// Convert a baked river width (m) to its rendered ribbon width.
#[inline]
pub fn baked_to_visible_width(baked: f32) -> f32 {
    baked * RIVER_RIBBON_WIDTH_SCALE + RIVER_RIBBON_WIDTH_PAD_M * 2.0
}
