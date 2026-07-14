import * as THREE from 'three'
import { MeshBasicNodeMaterial } from 'three/webgpu'
import { getHousingMaterial } from './housing-textures'
import { WOOD_TEXTURE_IDX } from './house-geo-utils'

/** Metres of board covered by one repeat of the wood texture (tunable). */
const BOARD_TEX_TILE = 0.6

/**
 * Procedural arched shop sign — a wooden board shaped like a rainbow arch (∩)
 * with the shop name running along the arch (e.g. "Rica's General Store").
 *
 * Generated in code rather than authored as a GLB so the shop name stays a
 * plain data string (editable in the map editor) instead of a baked-per-name
 * asset, and so the dimensions/curvature are tunable parameters. The board
 * template is shared and cloned by the object overlay; the text mesh is built
 * per placement from that placement's `text` field.
 *
 * Geometry: an annular sector (ring segment) whose flat face lies in the XY
 * plane and faces +Z (the readable front). The board arcs *upward* — the centre
 * is higher than the ends — and the `thickness` is the front-to-back depth.
 *
 * The text is drawn as ordinary straight text on a canvas and mapped onto a
 * matching annular ribbon in front of the face; the annular UV mapping is what
 * bends the letters into the arch (each glyph tilts to the local tangent), so
 * the text curvature always matches the board.
 *
 * Local space: origin at the bounding-box centre. Placement rotation (editor
 * `R`) turns the +Z face toward the street.
 */

export interface ShopSignParams {
  /** Horizontal chord of the arch centreline (m). */
  width: number
  /** Radial band width — how "tall" the arch strip itself is (m). */
  height: number
  /** Front-to-back plank depth (m). */
  thickness: number
  /** How much the arch centre rises above its ends (sagitta of the centreline,
   *  m). Larger = a taller, rounder arch; smaller = a flatter arch. */
  rise: number
  /** Angular tessellation of the arch. */
  segments: number
}

export const SHOP_SIGN_DEFAULTS: ShopSignParams = {
  width: 3,
  height: 0.5,
  thickness: 0.1,
  rise: 0.4,
  segments: 32,
}

export interface ShopSignTextParams {
  /** Fraction of the radial band height the text occupies (0..1). */
  heightFrac: number
  /** Fraction of the arch's angular span the text spans (inset from the ends). */
  widthFrac: number
  fillColor: string
  outlineColor: string
  /** Outline width in canvas pixels. */
  outlineWidth: number
}

export const SHOP_SIGN_TEXT_DEFAULTS: ShopSignTextParams = {
  heightFrac: 0.6,
  widthFrac: 0.98,
  fillColor: '#f6e7c1',
  outlineColor: '#2c1a0c',
  outlineWidth: 12,
}

interface Arch {
  /** Inner / centreline / outer radii. */
  ri: number
  rc: number
  ro: number
  /** Half of the angular span (radians), measured from the top (vertical). */
  phi: number
  /** Y offset that recentres the bounding box on the local origin. */
  yOff: number
}

/** Derive the arch radii and angular span from the tunable parameters. */
function computeArch(p: ShopSignParams): Arch {
  // Circle through the two ends and the raised centre of the centreline.
  const rc = p.rise / 2 + (p.width * p.width) / (8 * p.rise)
  const phi = Math.asin(Math.min(1, p.width / 2 / rc)) // half angular span
  const ro = rc + p.height / 2
  const ri = rc - p.height / 2
  // Curvature centre is at the local origin before recentring: the highest
  // point is the outer edge at the top, the lowest are the inner edge at the
  // ends. Shift so that midpoint sits on y=0.
  const maxY = ro
  const minY = ri * Math.cos(phi)
  const yOff = -(maxY + minY) / 2
  return { ri, rc, ro, phi, yOff }
}

/**
 * Extruded annular-sector geometry with explicit per-face outward normals.
 * Rendered DoubleSide so triangle winding never affects visibility, and normals
 * are supplied directly so lighting is correct regardless of winding.
 */
function buildBoardGeometry(p: ShopSignParams): THREE.BufferGeometry {
  const { ri, rc, ro, phi, yOff } = computeArch(p)
  const hz = p.thickness / 2

  const pos: number[] = []
  const nor: number[] = []
  const uvs: number[] = []

  // World-scale UVs so the wood texture tiles at a natural size (RepeatWrapping).
  // u runs along the arch (centreline arc length), v across the band / depth.
  const uArc = (a: number): number => (rc * (a + phi)) / BOARD_TEX_TILE

  const quad = (
    a: number[],
    b: number[],
    c: number[],
    d: number[],
    n: number[],
    uvA: number[],
    uvB: number[],
    uvC: number[],
    uvD: number[]
  ) => {
    // Orient the winding so the triangle's geometric front matches the outward
    // normal n. MeshStandardMaterial flips the shading normal by gl_FrontFacing,
    // so a face wound backwards relative to n renders as if lit from behind
    // (dark under a light on its front). Auto-correcting keeps every face lit
    // from its outward side regardless of how the corners were listed.
    const abx = b[0] - a[0]
    const aby = b[1] - a[1]
    const abz = b[2] - a[2]
    const acx = c[0] - a[0]
    const acy = c[1] - a[1]
    const acz = c[2] - a[2]
    const gx = aby * acz - abz * acy
    const gy = abz * acx - abx * acz
    const gz = abx * acy - aby * acx
    const outward = gx * n[0] + gy * n[1] + gz * n[2] >= 0
    const emit = (v: number[], t: number[]) => {
      pos.push(v[0], v[1], v[2])
      nor.push(n[0], n[1], n[2])
      uvs.push(t[0], t[1])
    }
    const order = outward
      ? [
          [a, uvA],
          [b, uvB],
          [c, uvC],
          [a, uvA],
          [c, uvC],
          [d, uvD],
        ]
      : [
          [a, uvA],
          [c, uvC],
          [b, uvB],
          [a, uvA],
          [d, uvD],
          [c, uvC],
        ]
    for (const [v, t] of order) emit(v, t)
  }

  // Points on the front (z=+hz) / back (z=-hz) faces at radius r, angle a.
  const pf = (r: number, a: number): number[] => [
    r * Math.sin(a),
    r * Math.cos(a) + yOff,
    hz,
  ]
  const pb = (r: number, a: number): number[] => [
    r * Math.sin(a),
    r * Math.cos(a) + yOff,
    -hz,
  ]
  // Face UV at radius r, angle a: u along arch, v across the radial band.
  const uvFace = (r: number, a: number): number[] => [
    uArc(a),
    (r - ri) / BOARD_TEX_TILE,
  ]
  const depthV = p.thickness / BOARD_TEX_TILE

  const N = p.segments
  for (let i = 0; i < N; i++) {
    const a0 = -phi + (2 * phi * i) / N
    const a1 = -phi + (2 * phi * (i + 1)) / N
    const am = (a0 + a1) / 2
    const radial = [Math.sin(am), Math.cos(am), 0] // outward radial

    // Front face (+Z)
    quad(
      pf(ri, a0),
      pf(ro, a0),
      pf(ro, a1),
      pf(ri, a1),
      [0, 0, 1],
      uvFace(ri, a0),
      uvFace(ro, a0),
      uvFace(ro, a1),
      uvFace(ri, a1)
    )
    // Back face (−Z)
    quad(
      pb(ri, a1),
      pb(ro, a1),
      pb(ro, a0),
      pb(ri, a0),
      [0, 0, -1],
      uvFace(ri, a1),
      uvFace(ro, a1),
      uvFace(ro, a0),
      uvFace(ri, a0)
    )
    // Outer edge (v = depth across the plank thickness)
    quad(
      pf(ro, a0),
      pb(ro, a0),
      pb(ro, a1),
      pf(ro, a1),
      radial,
      [uArc(a0), 0],
      [uArc(a0), depthV],
      [uArc(a1), depthV],
      [uArc(a1), 0]
    )
    // Inner edge
    quad(
      pf(ri, a1),
      pb(ri, a1),
      pb(ri, a0),
      pf(ri, a0),
      [-radial[0], -radial[1], 0],
      [uArc(a1), 0],
      [uArc(a1), depthV],
      [uArc(a0), depthV],
      [uArc(a0), 0]
    )
  }

  // End caps (tangent-facing).
  const capL = [-Math.cos(-phi), Math.sin(-phi), 0]
  quad(
    pf(ri, -phi),
    pf(ro, -phi),
    pb(ro, -phi),
    pb(ri, -phi),
    capL,
    [0, 0],
    [(ro - ri) / BOARD_TEX_TILE, 0],
    [(ro - ri) / BOARD_TEX_TILE, depthV],
    [0, depthV]
  )
  const capR = [Math.cos(phi), -Math.sin(phi), 0]
  quad(
    pf(ro, phi),
    pf(ri, phi),
    pb(ri, phi),
    pb(ro, phi),
    capR,
    [0, 0],
    [(ro - ri) / BOARD_TEX_TILE, 0],
    [(ro - ri) / BOARD_TEX_TILE, depthV],
    [0, depthV]
  )

  const geo = new THREE.BufferGeometry()
  geo.setAttribute('position', new THREE.Float32BufferAttribute(pos, 3))
  geo.setAttribute('normal', new THREE.Float32BufferAttribute(nor, 3))
  geo.setAttribute('uv', new THREE.Float32BufferAttribute(uvs, 2))
  geo.computeBoundingBox()
  geo.computeBoundingSphere()
  return geo
}

/** Build the shared, text-less board template (a group with one mesh). */
export function buildShopSignBoard(
  params: Partial<ShopSignParams> = {}
): THREE.Group {
  const p = { ...SHOP_SIGN_DEFAULTS, ...params }
  const geo = buildBoardGeometry(p)
  // Same shared material the house doors use (wood_shutter). It updates in place
  // once housing textures finish loading, so the sign always matches the doors.
  const mat = getHousingMaterial(WOOD_TEXTURE_IDX)
  const mesh = new THREE.Mesh(geo, mat)
  mesh.castShadow = true
  mesh.receiveShadow = true
  // Shared housing material — the object overlay's disposer must skip it, else
  // disposing a removed sign would break every door using this material.
  mesh.userData.isSignBoard = true
  const group = new THREE.Group()
  group.name = 'shop-sign-board'
  group.add(mesh)
  return group
}

const CANVAS_PX_PER_M = 256

/** Render the shop name as straight, centred, auto-fitted text on a canvas. */
function renderNameCanvas(
  text: string,
  arcLen: number,
  bandH: number,
  tp: ShopSignTextParams
): HTMLCanvasElement {
  const cw = Math.max(2, Math.round(arcLen * CANVAS_PX_PER_M))
  const chpx = Math.max(2, Math.round(bandH * CANVAS_PX_PER_M))
  const canvas = document.createElement('canvas')
  canvas.width = cw
  canvas.height = chpx
  const ctx = canvas.getContext('2d')!

  // Fit the text tightly into the canvas box: grow it to fill whichever axis
  // is the binding constraint, leaving only enough margin for the outline.
  // (The old logic capped by height and merely shrank on overflow, so short
  // names left large horizontal margins unused.)
  // Outline extends ~outlineWidth/2 past the glyphs on every side, so reserve
  // outlineWidth total on each axis.
  const pad = tp.outlineWidth
  const maxW = Math.max(1, cw - pad)
  const maxH = Math.max(8, chpx - pad)
  let fontPx = maxH
  const setFont = () =>
    (ctx.font = `bold ${fontPx}px Georgia, "Times New Roman", serif`)
  setFont()
  const measured = ctx.measureText(text).width
  if (measured > 0) {
    // Font size at which the text width would exactly equal maxW.
    const widthFit = (fontPx * maxW) / measured
    fontPx = Math.max(8, Math.floor(Math.min(maxH, widthFit)))
    setFont()
  }

  ctx.textAlign = 'center'
  ctx.textBaseline = 'middle'
  ctx.lineJoin = 'round'
  ctx.lineCap = 'round'
  if (tp.outlineWidth > 0) {
    ctx.strokeStyle = tp.outlineColor
    ctx.lineWidth = tp.outlineWidth
    ctx.strokeText(text, cw / 2, chpx / 2)
  }
  ctx.fillStyle = tp.fillColor
  ctx.fillText(text, cw / 2, chpx / 2)
  return canvas
}

/**
 * Build a per-instance curved text mesh: an annular ribbon sitting just in
 * front of the board's arched face. Straight canvas text is bent into the arch
 * purely by the ribbon's UV mapping, so it always tracks the board curvature.
 *
 * NOTE: the returned mesh is flagged `userData.isSignText` so the overlay's
 * material disposer skips it — disposing a MeshBasicNodeMaterial+CanvasTexture
 * pair crashes the WebGPU sampler bindings (see TextLabel.svelte); we let GC
 * reclaim it instead.
 */
export function buildShopSignText(
  text: string,
  boardParams: Partial<ShopSignParams> = {},
  textParams: Partial<ShopSignTextParams> = {}
): THREE.Mesh {
  const bp = { ...SHOP_SIGN_DEFAULTS, ...boardParams }
  const tp = { ...SHOP_SIGN_TEXT_DEFAULTS, ...textParams }
  const { rc, phi, yOff } = computeArch(bp)

  const phiT = phi * tp.widthFrac
  const bandH = bp.height * tp.heightFrac
  const rti = rc - bandH / 2 // inner (bottom of text)
  const rto = rc + bandH / 2 // outer (top of text)
  const z = bp.thickness / 2 + 0.012 // 12mm proud of the wood to avoid z-fight

  // --- Canvas texture (straight text; the ribbon supplies the arch) ---
  const arcLen = rc * 2 * phiT
  const canvas = renderNameCanvas(text, arcLen, bandH, tp)
  const texture = new THREE.CanvasTexture(canvas)
  texture.minFilter = THREE.LinearFilter
  texture.magFilter = THREE.LinearFilter
  texture.anisotropy = 4

  const mat = new MeshBasicNodeMaterial()
  mat.map = texture
  mat.transparent = true
  mat.depthWrite = false
  mat.side = THREE.DoubleSide

  // --- Annular ribbon geometry ---
  // u runs left→right of the arch (a: -phiT→+phiT; −X is viewer-left when the
  // +Z face is viewed head-on). v runs inner→outer, i.e. bottom→top of the
  // letters (outer radius = top of the arch).
  const N = Math.max(8, Math.round(bp.segments * tp.widthFrac))
  const pos: number[] = []
  const uv: number[] = []
  const nor: number[] = []
  const idx: number[] = []
  for (let i = 0; i <= N; i++) {
    const u = i / N
    const a = -phiT + 2 * phiT * u
    const s = Math.sin(a)
    const c = Math.cos(a)
    pos.push(rti * s, rti * c + yOff, z)
    uv.push(u, 0)
    nor.push(0, 0, 1)
    pos.push(rto * s, rto * c + yOff, z)
    uv.push(u, 1)
    nor.push(0, 0, 1)
  }
  for (let i = 0; i < N; i++) {
    const a = i * 2
    const b = i * 2 + 1
    const cc = i * 2 + 2
    const d = i * 2 + 3
    idx.push(a, cc, d, a, d, b)
  }
  const geo = new THREE.BufferGeometry()
  geo.setAttribute('position', new THREE.Float32BufferAttribute(pos, 3))
  geo.setAttribute('uv', new THREE.Float32BufferAttribute(uv, 2))
  geo.setAttribute('normal', new THREE.Float32BufferAttribute(nor, 3))
  geo.setIndex(idx)
  geo.computeBoundingBox()
  geo.computeBoundingSphere()

  const mesh = new THREE.Mesh(geo, mat)
  mesh.renderOrder = 1
  mesh.userData.isSignText = true
  return mesh
}
