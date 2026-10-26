//! A full-screen, cross-media-bar style background drawn on the GPU.
//!
//! The CPU works out where the waves run across each column of the screen (a few thousand sines
//! a frame) and uploads them as a small data texture; the fragment shader then lights every pixel
//! from those curves and fifteen colours. `shade` is the same maths on the CPU, used by tests and
//! anything that needs a picture without a GPU.

use crate::draw::TexId;
use crate::quad::Quad;
use crate::shaders::RECT_VERT;
use crate::surface::GfxError;
use crate::Canvas;

/// Columns and rows of the curve texture. Each texel holds two 16-bit values.
pub const BACKDROP_COLS: usize = 720;
pub const BACKDROP_ROWS: usize = 8;

/// Original: two sheets, a translucent veil and an opaque sheet, splitting the screen in two
/// colours. Classic: one colour crossed by four waves, each a sharp bright edge with light
/// trailing off one side and a soft shadow on the other.
pub const STYLE_ORIGINAL: u8 = 0;
pub const STYLE_CLASSIC: u8 = 1;

/// Rows of the curve texture: row 0 holds Original's sheet edge and veil edge; rows 1 and 2 hold
/// Classic's four waves, two a row.
pub const ROW_EDGES: usize = 0;
pub const ROW_LINES: usize = 1;
pub const LINES: usize = 4;

/// The colours a backdrop is lit with, in five parts of three: the left, middle and right of
/// the top, the ground just above the sheet, the sheet just below its edge, the sheet at the
/// bottom, and Classic's ground at the bottom.
pub type BackdropColours = [[u8; 3]; 15];

/// Where across the screen (0..1) the left, middle and right colours were measured.
const AT_LEFT: f32 = 0.234;
const AT_MIDDLE: f32 = 0.583;
const AT_RIGHT: f32 = 0.979;

#[derive(Copy, Clone, PartialEq, Debug)]
pub struct Backdrop {
    pub curves: TexId,
    pub style: u8,
    pub colours: BackdropColours,
    pub edges: BackdropEdges,
    /// For the start-up and switch-off: how far either side of the middle line the picture
    /// shows (0.5 and more is all of it), its brightness, how far its waves are smoothed away
    /// (0 none, 1 a plain field of the colour), and how soft the band's edges are.
    pub shape: [f32; 4],
    pub alpha: f32,
    /// The handheld's own background picture for the colour, stretched over the screen, when it
    /// is on the card: both styles draw their ground from it instead of from `colours`.
    pub ground: Option<TexId>,
}

/// Original's edges, per colour: the glow trailing below the main wave's edge (rgb, then its
/// length in pixels of a 480-line screen), the same below the veil's, then how much the veil
/// behind the main wave shows (its opacity) and how soft both edges are (half
/// the width of their step, in the same pixels; 3 when zero), then a saturation for the whole
/// picture (1 when zero) and, at 1, the older layering by height instead (the third value is then
/// how much of the sheet the upper edge brings).
pub type BackdropEdges = [[f32; 4]; 3];

/// The picture as it is: not squeezed, brightened or smoothed.
pub const BACKDROP_PLAIN: [f32; 4] = [1.0, 1.0, 0.0, 0.03];

/// Packs a curve height (screen fractions, -0.5..1.5) into two bytes.
pub fn encode(v: f32) -> [u8; 2] {
    let q = (((v + 0.5) / 2.0).clamp(0.0, 1.0) * 65535.0).round() as u32;
    [(q >> 8) as u8, (q & 0xff) as u8]
}

pub fn decode(hi: u8, lo: u8) -> f32 {
    (hi as f32 * 256.0 + lo as f32) / 65535.0 * 2.0 - 0.5
}

/// Writes one value into a curve buffer of `BACKDROP_COLS` by `BACKDROP_ROWS` RGBA texels.
pub fn put(buf: &mut [u8], col: usize, row: usize, second: bool, v: f32) {
    let i = (row * BACKDROP_COLS + col) * 4 + if second { 2 } else { 0 };
    buf[i..i + 2].copy_from_slice(&encode(v));
}

fn get(buf: &[u8], col: usize, row: usize, second: bool) -> f32 {
    let i = (row * BACKDROP_COLS + col) * 4 + if second { 2 } else { 0 };
    decode(buf[i], buf[i + 1])
}

const FRAG: &str = r#"
#ifdef GL_FRAGMENT_PRECISION_HIGH
precision highp float;
#else
precision mediump float;
#endif
varying vec2 v_uv;
uniform sampler2D u_curves;
uniform vec2 u_size;
uniform float u_style;
uniform float u_alpha;
uniform vec3 u_c[15];
uniform vec4 u_e[3];
uniform vec4 u_shape;
uniform sampler2D u_ground;
uniform float u_has_ground;

vec4 texel(float row) {
    return texture2D(u_curves, vec2(v_uv.x, (row + 0.5) / 8.0));
}

float dec(vec2 hl) {
    return (hl.x * 65280.0 + hl.y * 255.0) / 65535.0 * 2.0 - 0.5;
}

// Left to middle and middle to right are straight runs, eased into each other across the
// middle (and into the held colour at the far left) so no seam shows where they meet.
float floor_soft(float x, float a) {
    float d = x - a;
    return a + 0.5 * (d + sqrt(d * d + 0.01));
}

vec3 side(vec3 l, vec3 m, vec3 r, float u) {
    vec3 a = mix(l, m, floor_soft((u - 0.234) / 0.349, -0.3));
    vec3 b = mix(m, r, clamp((u - 0.583) / 0.396, -1.0, 1.06));
    return clamp(mix(a, b, smoothstep(0.433, 0.733, u)), 0.0, 1.0);
}

// Lengths below are in pixels of a 480-line screen; `k` scales them to this one.
float lit_side(float d, float k) {
    // Light trailing off the bright side of a Classic wave.
    return pow(max(1.0 - d / (46.0 * k), 0.0), 1.6) * smoothstep(-1.5 * k, 0.5 * k, d);
}

void main() {
    float u = v_uv.x;
    float v = v_uv.y;
    float k = u_size.y / 480.0;
    // Only a band about the middle shows, `u_shape.x` either side of it, its edges soft.
    float inside = 1.0 - smoothstep(u_shape.x - u_shape.w, u_shape.x + u_shape.w, abs(v - 0.5));
    float py = v * u_size.y;
    vec3 top = side(u_c[0], u_c[1], u_c[2], u);
    // The handheld's own picture for the colour, when it is on the card, a touch brighter on its
    // screen than in the file: the ground under both styles' waves.
    vec3 picture = clamp(texture2D(u_ground, v_uv).rgb * 1.046 + 0.04, 0.0, 1.0);
    vec3 col;
    vec3 plain;
    if (u_style < 0.5) {
        vec4 e = texel(0.0);
        float d1 = py - dec(e.rg) * u_size.y;
        float d2 = py - dec(e.ba) * u_size.y;
        vec3 ground = mix(top, side(u_c[3], u_c[4], u_c[5], u), clamp((v - 0.11) / 0.44, 0.0, 1.0));
        if (u_has_ground > 0.5) {
            ground = picture;
        }
        plain = ground;
        float em = min(dec(e.rg), dec(e.ba));
        vec3 sheet = mix(side(u_c[6], u_c[7], u_c[8], u), side(u_c[9], u_c[10], u_c[11], u),
            clamp((v - em - 0.055) / max(0.92 - em, 0.05), 0.0, 1.0));
        // Two sheets: the main wave in front, solid, and the veil behind it, faint, whichever
        // of the two is higher at this point. `u_e[2].x` is how much the veil shows.
        // Each edge is a soft step, three pixels either side unless the colour asks for more.
        float soft = max(u_e[2].y, 3.0) * k;
        float cm = smoothstep(-soft, soft, d1);
        float cv = smoothstep(-soft, soft, d2);
        float veil = u_e[2].x;
        if (u_e[2].w > 0.5) {
            // The older layering, kept for a colour tuned with it: the upper edge brings part of
            // the sheet's colour, the lower the rest, each trailing its glow.
            float du = max(d1, d2);
            float dl = min(d1, d2);
            float cu = smoothstep(-soft, soft, du);
            float cl = smoothstep(-soft, soft, dl);
            col = mix(ground, sheet, cl + veil * (cu - cl));
            col += u_e[0].rgb * exp(-max(du, 0.0) / (u_e[0].w * k)) * cu;
            col += u_e[1].rgb * exp(-max(dl, 0.0) / (u_e[1].w * k)) * cl;
        } else {
            col = mix(ground, sheet, 1.0 - (1.0 - cm) * (1.0 - veil * cv));
            // Below each edge some colours trail a glow of their own; the veil's is as faint as it.
            col += u_e[0].rgb * exp(-max(d1, 0.0) / (u_e[0].w * k)) * cm;
            col += u_e[1].rgb * exp(-max(d2, 0.0) / (u_e[1].w * k)) * cv * veil * (1.0 - cm);
        }
        col = clamp(col, 0.0, 1.0);
        // Some colours run a little less saturated than measured, as this screen is vivid.
        if (u_e[2].z > 0.0) {
            col = mix(vec3(dot(col, vec3(0.299, 0.587, 0.114))), col, u_e[2].z);
        }
    } else {
        col = mix(top, side(u_c[12], u_c[13], u_c[14], u), smoothstep(0.11, 0.97, v));
        if (u_has_ground > 0.5) {
            col = picture;
        }
        plain = col;
        for (int i = 0; i < 2; i++) {
            vec4 l = texel(1.0 + float(i));
            // The upper pair shine on their top edge, the lower pair on their bottom.
            float dir = i == 0 ? 1.0 : -1.0;
            float strength = i == 0 ? 0.24 : 0.22;
            float d0 = (py - dec(l.rg) * u_size.y) * dir;
            float d1 = (py - dec(l.ba) * u_size.y) * dir;
            // White light added, the same to every channel, as the handheld's: where waves
            // cross it builds up, and on a pale colour it runs to white.
            col += vec3(strength * lit_side(d0, k) + (strength - 0.04) * lit_side(d1, k));
        }
        col = min(col, vec3(1.0));
    }
    col = mix(col, plain, u_shape.z);
    col = clamp(col * u_shape.y, 0.0, 1.0) * inside;
    FRAG_COLOR = vec4(col, u_alpha);
}
"#;

/// The shader source, exposed so tests can compile and run it off-device.
pub fn fragment_source() -> &'static str {
    FRAG
}

fn smoothstep(e0: f32, e1: f32, x: f32) -> f32 {
    let t = ((x - e0) / (e1 - e0)).clamp(0.0, 1.0);
    t * t * (3.0 - 2.0 * t)
}

fn mix3(a: [f32; 3], b: [f32; 3], t: f32) -> [f32; 3] {
    [0, 1, 2].map(|i| a[i] + (b[i] - a[i]) * t)
}

fn side(c: &[[f32; 3]; 15], part: usize, u: f32) -> [f32; 3] {
    let (l, m, r) = (c[part * 3], c[part * 3 + 1], c[part * 3 + 2]);
    // As the shader: two straight runs eased into each other across the middle.
    let floor_soft = |x: f32, a: f32| {
        let d = x - a;
        a + 0.5 * (d + (d * d + 0.01).sqrt())
    };
    let a = mix3(l, m, floor_soft((u - AT_LEFT) / (AT_MIDDLE - AT_LEFT), -0.3));
    let b = mix3(m, r, ((u - AT_MIDDLE) / (AT_RIGHT - AT_MIDDLE)).clamp(-1.0, 1.06));
    let v = mix3(a, b, smoothstep(AT_MIDDLE - 0.15, AT_MIDDLE + 0.15, u));
    v.map(|x| x.clamp(0.0, 1.0))
}

fn lit_side(d: f32, k: f32) -> f32 {
    (1.0 - d / (46.0 * k)).max(0.0).powf(1.6) * smoothstep(-1.5 * k, 0.5 * k, d)
}

/// The colour the shader gives the pixel at `u`, `v` (0..1 across and down) of a canvas `h`
/// pixels tall, before `alpha`. Mirrors the fragment shader line for line.
#[allow(clippy::too_many_arguments)]
pub fn shade(
    curves: &[u8],
    style: u8,
    c: &BackdropColours,
    e: &BackdropEdges,
    shape: [f32; 4],
    u: f32,
    v: f32,
    h: f32,
) -> [f32; 3] {
    let inside = 1.0 - smoothstep(shape[0] - shape[3], shape[0] + shape[3], (v - 0.5).abs());
    let (col, plain) = shade_plain(curves, style, c, e, u, v, h);
    let col = mix3(col, plain, shape[2]);
    col.map(|x| (x * shape[1]).clamp(0.0, 1.0) * inside)
}

fn shade_plain(
    curves: &[u8],
    style: u8,
    c: &BackdropColours,
    e: &BackdropEdges,
    u: f32,
    v: f32,
    h: f32,
) -> ([f32; 3], [f32; 3]) {
    let c = &c.map(|rgb| rgb.map(|v| v as f32 / 255.0));
    let col_i = ((u * BACKDROP_COLS as f32) as usize).min(BACKDROP_COLS - 1);
    let k = h / 480.0;
    let py = v * h;
    let top = side(c, 0, u);
    if style == STYLE_ORIGINAL {
        let (e1, e2) = (
            get(curves, col_i, ROW_EDGES, false),
            get(curves, col_i, ROW_EDGES, true),
        );
        let (d1, d2) = (py - e1 * h, py - e2 * h);
        let ground = mix3(top, side(c, 1, u), ((v - 0.11) / 0.44).clamp(0.0, 1.0));
        let em = e1.min(e2);
        let sheet = mix3(
            side(c, 2, u),
            side(c, 3, u),
            ((v - em - 0.055) / (0.92 - em).max(0.05)).clamp(0.0, 1.0),
        );
        let soft = e[2][1].max(3.0) * k;
        let cm = smoothstep(-soft, soft, d1);
        let cv = smoothstep(-soft, soft, d2);
        let veil = e[2][0];
        let (col, gu, gl) = if e[2][3] > 0.5 {
            // As the shader: the older layering, by height.
            let (du, dl) = (d1.max(d2), d1.min(d2));
            let (cu, cl) = (smoothstep(-soft, soft, du), smoothstep(-soft, soft, dl));
            (
                mix3(ground, sheet, cl + veil * (cu - cl)),
                (-du.max(0.0) / (e[0][3] * k)).exp() * cu,
                (-dl.max(0.0) / (e[1][3] * k)).exp() * cl,
            )
        } else {
            (
                mix3(ground, sheet, 1.0 - (1.0 - cm) * (1.0 - veil * cv)),
                (-d1.max(0.0) / (e[0][3] * k)).exp() * cm,
                (-d2.max(0.0) / (e[1][3] * k)).exp() * cv * veil * (1.0 - cm),
            )
        };
        let col = [0, 1, 2].map(|i| (col[i] + e[0][i] * gu + e[1][i] * gl).clamp(0.0, 1.0));
        // As the shader: some colours a little less saturated.
        let col = match e[2][2] > 0.0 {
            true => {
                let y = 0.299 * col[0] + 0.587 * col[1] + 0.114 * col[2];
                col.map(|c| y + (c - y) * e[2][2])
            }
            false => col,
        };
        (col, ground)
    } else {
        let mut col = mix3(top, side(c, 4, u), smoothstep(0.11, 0.97, v));
        let plain = col;
        for i in 0..2 {
            let dir = if i == 0 { 1.0 } else { -1.0 };
            let strength = if i == 0 { 0.24 } else { 0.22 };
            let d0 = (py - get(curves, col_i, ROW_LINES + i, false) * h) * dir;
            let d1 = (py - get(curves, col_i, ROW_LINES + i, true) * h) * dir;
            let add = strength * lit_side(d0, k) + (strength - 0.04) * lit_side(d1, k);
            col = col.map(|x| x + add);
        }
        (col.map(|x| x.min(1.0)), plain)
    }
}

pub(crate) struct BackdropPass {
    prog: gl::types::GLuint,
    u_rect: gl::types::GLint,
    u_target: gl::types::GLint,
    u_size: gl::types::GLint,
    u_style: gl::types::GLint,
    u_alpha: gl::types::GLint,
    u_c: gl::types::GLint,
    u_e: gl::types::GLint,
    u_shape: gl::types::GLint,
    u_has_ground: gl::types::GLint,
}

impl BackdropPass {
    pub fn new() -> Result<Self, GfxError> {
        let prog = crate::shaders::program(RECT_VERT, FRAG)?;
        unsafe {
            gl::UseProgram(prog);
            gl::Uniform1i(crate::gl::uniform_location(prog, "u_curves"), 0);
            gl::Uniform1i(crate::gl::uniform_location(prog, "u_ground"), 1);
        }
        Ok(BackdropPass {
            prog,
            u_rect: crate::gl::uniform_location(prog, "u_rect"),
            u_target: crate::gl::uniform_location(prog, "u_target"),
            u_size: crate::gl::uniform_location(prog, "u_size"),
            u_style: crate::gl::uniform_location(prog, "u_style"),
            u_alpha: crate::gl::uniform_location(prog, "u_alpha"),
            u_c: crate::gl::uniform_location(prog, "u_c"),
            u_e: crate::gl::uniform_location(prog, "u_e"),
            u_shape: crate::gl::uniform_location(prog, "u_shape"),
            u_has_ground: crate::gl::uniform_location(prog, "u_has_ground"),
        })
    }

    pub fn draw(
        &self,
        b: &Backdrop,
        curves: gl::types::GLuint,
        ground: Option<gl::types::GLuint>,
        canvas: Canvas,
        quad: &Quad,
    ) {
        let flat: Vec<f32> = b.colours.iter().flatten().map(|v| *v as f32 / 255.0).collect();
        unsafe {
            if b.alpha < 1.0 {
                gl::Enable(gl::BLEND);
                gl::BlendFunc(gl::SRC_ALPHA, gl::ONE_MINUS_SRC_ALPHA);
            }
            gl::UseProgram(self.prog);
            if let Some(ground) = ground {
                gl::ActiveTexture(gl::TEXTURE1);
                gl::BindTexture(gl::TEXTURE_2D, ground);
            }
            gl::Uniform1f(self.u_has_ground, if ground.is_some() { 1.0 } else { 0.0 });
            gl::ActiveTexture(gl::TEXTURE0);
            gl::BindTexture(gl::TEXTURE_2D, curves);
            gl::Uniform4f(self.u_rect, 0.0, 0.0, canvas.w, canvas.h);
            gl::Uniform2f(self.u_target, canvas.w, canvas.h);
            gl::Uniform2f(self.u_size, canvas.w, canvas.h);
            gl::Uniform1f(self.u_style, b.style as f32);
            gl::Uniform1f(self.u_alpha, b.alpha);
            gl::Uniform3fv(self.u_c, 15, flat.as_ptr());
            let edges: Vec<f32> = b.edges.iter().flatten().copied().collect();
            gl::Uniform4fv(self.u_e, 3, edges.as_ptr());
            gl::Uniform4f(self.u_shape, b.shape[0], b.shape[1], b.shape[2], b.shape[3]);
        }
        quad.draw();
        unsafe { gl::Disable(gl::BLEND) };
    }
}

impl Drop for BackdropPass {
    fn drop(&mut self) {
        unsafe { gl::DeleteProgram(self.prog) };
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn heights_survive_the_texture() {
        for v in [-0.4f32, 0.0, 0.123, 0.5, 0.999, 1.3] {
            let [h, l] = encode(v);
            assert!((decode(h, l) - v).abs() < 1e-4, "{v}");
        }
    }
}
