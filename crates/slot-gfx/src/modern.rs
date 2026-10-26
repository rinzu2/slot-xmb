//! The Modern background: the PS3's wave, its sparkles and its backdrop, drawn the way the
//! console draws them. The programs are faquino's re-authorings of the XMB's own
//! (`ps3xmbwave/spline.js`, `postprocess.js`, `backdrop.js`, `particles.js`, MIT), brought to
//! GLSL ES 1.00 so that the handheld's GPU runs them: the light the wave adds up is written in the
//! console's two 8-bit channels, the glare is drawn in half floats where the GPU can render to
//! them (and, scaled, in bytes where it cannot), and each sparkle is a quad of its own rather
//! than an instance. The scene that feeds them runs on the CPU (`slot_ui::xmb::modern`).

use crate::surface::GfxError;

/// The wave's mesh: 128 lines of 128 vertices, each its position in clip space, then its normal.
pub const MODERN_MESH: usize = 128;
pub const MODERN_VERTICES: usize = MODERN_MESH * MODERN_MESH;
pub const MODERN_RECORD: usize = 8;
/// Floats a sparkle's corner takes: the corner, the position and life's opacity, the turn.
pub const MODERN_SPARKLE_VERTEX: usize = 10;
/// The most sparkles the console's pool holds.
pub const MODERN_SPARKLES: usize = 2049;
/// The backdrop's textures, as the console sizes them.
pub const MODERN_BACK_W: usize = 64;
pub const MODERN_BACK_H: usize = 32;
/// The sparkles' iridescent texture.
pub const MODERN_IRIDESCENT: usize = 128;

const GLARE_W: i32 = 128;
const GLARE_H: i32 = 64;
const PRE_H: i32 = 540;
const LEVELS: usize = 6;
const NOISE_SIZE: usize = 32;
const HALF_FLOAT_OES: gl::types::GLenum = 0x8D61;

/// What the wave's programs read: LINE1.mnu's shading.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct ModernWave {
    pub mipmap_bias: f32,
    pub brightness: f32,
    pub fresnel: f32,
    pub spacing: f32,
    pub thinness: f32,
}

/// What the passes after the wave read: HDR.mnu's and BACKGROUND.mnu's, worked out as the
/// console's are.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct ModernPost {
    pub exposure: f32,
    pub white_sqr_rcp: f32,
    pub threshold: f32,
    pub gamma: f32,
    /// Each tap's weight, red, green and blue, from the centre out.
    pub gauss: [f32; 24],
    pub levels: [f32; LEVELS],
    pub glare_weight: f32,
    /// BACKGROUND.mnu's four colours: the backdrop's at the top and the bottom, the wave's at the
    /// left and the right.
    pub colours: [[f32; 3]; 4],
}

/// What the backdrop's program reads, and its ease from the last moment.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct ModernBackdrop {
    pub day_time: f32,
    pub night_time: f32,
    pub month_time: f32,
    pub night_brightness: f32,
    pub night_day_blend: f32,
    /// COLOUR SHADER: 0 the months', 1 the music's.
    pub shader: u8,
    /// How far into its ease the backdrop is, 0 to 1.
    pub alpha: f32,
    /// A count of the moments handed over: a new one is eased in from a snapshot.
    pub serial: u64,
    /// A count of the textures' changes: a new one is uploaded.
    pub textures: u64,
}

/// What the sparkles' programs read.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct ModernSparkles {
    pub mvp: [f32; 16],
    pub eye: [f32; 3],
    pub spot: [f32; 3],
    pub attn: [f32; 3],
    /// lambert, specular, exposure, specular power
    pub light: [f32; 4],
    pub life_min: [f32; 3],
    pub life_max: [f32; 3],
    /// near focus, its end, far focus, its end
    pub focus: [f32; 4],
    /// near and far blur exponents, vertical field of view
    pub focus_curves: [f32; 3],
    /// in focus, nearest, farthest
    pub size: [f32; 3],
    pub darkness: [f32; 2],
    /// size align, near fuzziness, near align
    pub near: [f32; 3],
    /// fresnel exponent, global alpha
    pub transparency: [f32; 2],
    /// strength, scale, falloff exponent, falloff rate
    pub glare: [f32; 4],
    pub iridescent_exp: f32,
    pub color_control: f32,
    pub colour: f32,
    pub gamma: f32,
}

/// A frame of the Modern background, as the scene hands it over.
#[derive(Clone, Debug, Default)]
pub struct ModernFrame {
    /// `MODERN_VERTICES` records of `MODERN_RECORD` floats.
    pub mesh: Vec<f32>,
    /// Four corners a sparkle, `MODERN_SPARKLE_VERTEX` floats each.
    pub sparkles: Vec<f32>,
    pub sparkle_count: usize,
    pub wave: ModernWave,
    pub post: ModernPost,
    pub backdrop: ModernBackdrop,
    pub particles: ModernSparkles,
    /// The backdrop's four textures, this month's and next month's by day and by night,
    /// `MODERN_BACK_W` by `MODERN_BACK_H` RGBA, first row at the bottom. Read when
    /// `backdrop.textures` moves on.
    pub months: [Vec<u8>; 4],
    /// The sparkles' colours, `MODERN_IRIDESCENT` square RGBA; read once.
    pub iridescent: Vec<u8>,
    /// The scene's brightness, 0 for black.
    pub fade: f32,
}

const FULL_VERT: &str = r#"
attribute vec2 a_pos;
varying vec2 v_uv;
void main() {
    v_uv = a_pos;
    gl_Position = vec4(a_pos * 2.0 - 1.0, 0.0, 1.0);
}
"#;

const HIGHP: &str = "#ifdef GL_FRAGMENT_PRECISION_HIGH\nprecision highp float;\n#else\nprecision mediump float;\n#endif\n";

// lines1.vpo: the position as it comes, already in clip space, and m, the cell's size on screen.
const WAVE_VERT: &str = r#"
attribute vec4 a_pos;
attribute vec4 a_normal;
attribute vec2 a_uv;
uniform float u_mipmap_bias;
uniform float u_brightness;
uniform float u_fresnel;
varying vec2 v_coord;
varying vec4 v_params;
void main() {
    float inv_len = inversesqrt(max(dot(a_pos.xyz, a_pos.xyz), 1e-10));
    vec3 e = a_pos.xyz * inv_len;
    vec3 n = a_normal.xyz * inversesqrt(max(dot(a_normal.xyz, a_normal.xyz), 1e-10));
    float m = abs(dot(e, a_normal.xyz)) * inv_len * u_mipmap_bias;
    v_coord = vec2(a_uv.x, m);
    v_params = vec4(u_fresnel * m, u_brightness * m, abs(dot(e, n)) * m, a_uv.y);
    gl_Position = a_pos;
}
"#;

// lines1.fpo: the stripes and the fresnel table read with m, and the light written as _Encode
// holds it - a fine part and a coarse one, truncated to bytes - its coarse part saturating at a
// light of 0.5. The composite reads it back.
const WAVE_FRAG: &str = r#"
uniform sampler2D u_stripes;
uniform sampler2D u_fres;
uniform float u_spacing;
uniform float u_thinness;
varying vec2 v_coord;
varying vec4 v_params;
void main() {
    float span = 1.0 - u_thinness;
    float x = 0.0;
    if (span > 0.0) {
        x = clamp(fract(v_coord.x * u_spacing) - u_thinness, 0.0, 1.0) / span;
    }
    float stripe = texture2D(u_stripes, vec2(x, v_coord.y)).r;
    vec4 fres = texture2D(u_fres, v_params.zw);
    float light = (fres.r * v_params.x + fres.g * v_params.y * stripe) * v_params.w;
    float n = clamp(floor(light * 4096.0), 0.0, 4095.0);
    float v = n / 4095.0 * 128.0;
    float w = floor(v);
    vec2 code = min(vec2((v - w) * 0.25, w * 0.015625), 1.0);
    FRAG_COLOR = vec4(floor(code * 255.0) / 255.0, 0.0, 0.0);
}
"#;

// back_colours0.fpo, its month fits read from textures through a quintic step across each of
// their 64 x 32 texels.
const BACK_FRAG: &str = r#"
uniform sampler2D u_day1;
uniform sampler2D u_day2;
uniform sampler2D u_night1;
uniform sampler2D u_night2;
uniform float u_day_time;
uniform float u_night_time;
uniform float u_month_time;
uniform float u_night_brightness;
uniform float u_night_day_blend;
uniform float u_alpha;
varying vec2 v_uv;

vec2 smooth_coord(vec2 uv) {
    vec2 size = vec2(64.0, 32.0);
    vec2 t = uv * size + 0.5;
    vec2 i = floor(t);
    vec2 f = t - i;
    return (i + f * f * f * (f * (f * 6.0 - 15.0) + 10.0) - 0.5) / size;
}

vec3 rgb2hsv(vec3 c) {
    float mx = max(c.r, max(c.g, c.b));
    float mn = min(c.r, min(c.g, c.b));
    if (mx == mn) return vec3(0.0, 0.0, mx);
    vec3 d = (mx - c) / (6.0 * (mx - mn)) + 0.5;
    float h;
    if (c.r == mx) h = d.b - d.g;
    else if (c.g == mx) h = 1.0 / 3.0 + d.r - d.b;
    else h = 2.0 / 3.0 + d.g - d.r;
    if (h < 0.0) h += 1.0;
    if (h > 1.0) h -= 1.0;
    return vec3(h, (mx - mn) / mx, mx);
}

vec3 hsv2rgb(vec3 hsv) {
    float v = hsv.z;
    float s = hsv.y;
    if (s == 0.0) return vec3(v);
    float h6 = clamp(hsv.x - 0.0001, 0.0, 1.0) * 6.0;
    float f = fract(h6);
    float p = v * (1.0 - s);
    float q = v * (1.0 - s * f);
    float t = v * (1.0 - s * (1.0 - f));
    if (h6 < 1.0) return vec3(v, t, p);
    if (h6 < 2.0) return vec3(q, v, p);
    if (h6 < 3.0) return vec3(p, v, t);
    if (h6 < 4.0) return vec3(p, q, v);
    if (h6 < 5.0) return vec3(t, p, v);
    return vec3(v, p, q);
}

float rise(float k) {
    float e = exp(2.0 * clamp(k, -10.0, 10.0));
    return 0.5 + 0.5 * (e - 1.0) / (e + 1.0);
}

float window(float clock, float a, float b) {
    return rise(a * (clock - 600.0)) * rise(b * (1800.0 - clock));
}

float glow(float clock, vec2 uv) {
    float a = (clock - 600.0) * 3.14159265 / 1200.0;
    float dx = uv.x - 0.5 - 0.4 * cos(a);
    float broad = 4.0 * (uv.y - 0.05) * (uv.y - 0.05)
        + dx * dx / (1.0 + exp(-0.008 * (clock - 1200.0) * (clock - 1200.0)));
    float ex = uv.x - 0.5 + 0.8 * cos(a);
    float ey = uv.y - 0.99 * sin(a);
    return 0.2 * exp(-4.0 * broad) + 0.19 * exp(-8.0 * (1.25 * ey * ey + ex * ex));
}

void main() {
    vec2 uv = v_uv;
    vec2 st = smooth_coord(uv);
    float day_glow = glow(u_day_time, uv);
    float mid = exp(-0.01 * (u_month_time - 15.0) * (u_month_time - 15.0));
    float value = window(u_day_time, 0.005, 0.01)
        * exp(-6.4 * uv.y * (1.0 - window(u_day_time, 0.005, 0.005))) * (day_glow + 0.6);
    float saturation = (2.0 - exp(-6.4 * uv.y * (1.0 - window(u_day_time, 0.01, 0.01))) - 0.25 * mid)
        * (0.9 - day_glow);
    vec3 hsv1 = rgb2hsv(texture2D(u_day1, st).rgb);
    vec3 hsv2 = rgb2hsv(texture2D(u_day2, st).rgb);
    vec3 day1 = clamp(hsv2rgb(vec3(hsv1.x, hsv1.y * saturation, hsv1.z * value)), 0.0, 1.0);
    vec3 day2 = clamp(hsv2rgb(vec3(hsv2.x, hsv2.y * saturation, hsv2.z * value)), 0.0, 1.0);
    float m = u_month_time;
    float edge = clamp((uv.y + 1.0 - 0.1 * m) * (0.1 + (0.4 / 9.0) * (m - 15.0) * (m - 15.0)), -10.0, 10.0);
    float next = 1.0 - rise(edge);
    vec3 day = mix(day1, day2, next);
    float level = window(u_night_time, 0.005, 0.01)
        * exp(-6.4 * uv.y * (1.0 - window(u_night_time, 0.005, 0.005))) * (glow(u_night_time, uv) + 0.6);
    level = clamp(max(level, u_night_brightness), 0.0, 1.0);
    vec3 night = level * mix(texture2D(u_night1, st).rgb, texture2D(u_night2, st).rgb, next);
    FRAG_COLOR = vec4(mix(night, day, u_night_day_blend), u_alpha);
}
"#;

// back_colours1.fpo: no texture, a light that fades down the screen, two glows and a soft edge.
const MUSIC_FRAG: &str = r#"
uniform float u_day_time;
uniform float u_alpha;
varying vec2 v_uv;
void main() {
    float u = v_uv.x;
    float v = v_uv.y;
    float far = exp(-0.5 * ((u - 1.5) * (u - 1.5) + (v - 1.5) * (v - 1.5)));
    float low = (u - 0.4) * (u - 0.4) / 6.0 + (v - 0.1) * (v - 0.1);
    float pulse = 1.0 + 0.5 * cos(u_day_time);
    vec3 c = vec3(
        0.4 * exp(-0.4 * (1.0 - v)) + 0.6 * exp(-16.0 * pulse * low) + 0.375 * far,
        0.3 * exp(-0.2 * (1.0 - v)) + 0.25 * exp(-12.0 * low) + 0.3 * far,
        0.5 * exp(-0.1 * (1.0 - v)) + 0.45 * exp(-8.0 * low) + 0.6 * far);
    float e = exp(2.0 * clamp(2.5 * (v - 0.2 * u - 0.4), -10.0, 10.0));
    FRAG_COLOR = vec4(c * 0.4 * (1.0 + (e - 1.0) / (e + 1.0)), u_alpha);
}
"#;

const COPY_FRAG: &str = r#"
uniform sampler2D u_src;
uniform float u_scale;
varying vec2 v_uv;
void main() {
    FRAG_COLOR = texture2D(u_src, v_uv) * u_scale;
}
"#;

// LinesController.fpo: the backdrop under BACKGROUND.mnu's vertical gradient, plus the wave's
// light read back out of its two channels under the horizontal one, through the preexpose
// tables' tone curve; alpha is the glare's mask.
const COMPOSITE_FRAG: &str = r#"
uniform sampler2D u_back;
uniform sampler2D u_wave;
uniform sampler2D u_noise;
uniform vec2 u_screen;
uniform vec3 u_colour1;
uniform vec3 u_colour2;
uniform vec3 u_colour3;
uniform vec3 u_colour4;
uniform float u_exposure;
uniform float u_white_sqr_rcp;
uniform float u_threshold;
uniform float u_gamma;
vec3 tone(vec3 x) { return x * (1.0 + x * u_white_sqr_rcp) / (1.0 + x); }
void main() {
    vec2 uv = gl_FragCoord.xy / u_screen;
    vec3 back = texture2D(u_back, uv).rgb * mix(u_colour2, u_colour1, uv.y);
    vec2 code = texture2D(u_wave, (gl_FragCoord.xy + 0.25) / u_screen).rg;
    float light = code.r * 0.03125 + code.g * 0.5;
    vec3 lit = back + light * mix(u_colour3, u_colour4, uv.x);
    vec3 texel = clamp(lit * 8.0 - 0.5, 0.0, 127.0);
    vec3 y = u_gamma * tone(u_exposure * (16.0 / 127.0) * texel);
    float top = tone(vec3(u_exposure * (16.0 / 127.0) * max(texel.r, max(texel.g, texel.b)))).r;
    float mask = 0.0;
    if (top > 0.0) {
        mask = (top - u_threshold) * top / (8.0 * min(top, 1.0));
    }
    vec3 noise = texture2D(u_noise, gl_FragCoord.xy / 32.0).rgb;
    FRAG_COLOR = vec4(y + noise * 0.125, mask);
}
"#;

// GlareSourcePre.fpo: the screen times its mask, times 8 (over `u_scale`'s range in bytes).
const SOURCE_FRAG: &str = r#"
uniform sampler2D u_src;
uniform float u_scale;
varying vec2 v_uv;
void main() {
    vec4 s = texture2D(u_src, v_uv);
    FRAG_COLOR = vec4(s.rgb * clamp(s.a, 0.0, 1.0) * 8.0 * u_scale, 0.0);
}
"#;

// Gaussian.fpo: the texel and up to seven either side along one axis, each channel its weights.
const GAUSSIAN_FRAG: &str = r#"
uniform sampler2D u_src;
uniform vec2 u_step;
uniform vec3 u_weights[8];
varying vec2 v_uv;
void main() {
    vec3 sum = texture2D(u_src, v_uv).rgb * u_weights[0];
    for (int k = 1; k < 8; k++) {
        vec2 d = u_step * float(k);
        sum += (texture2D(u_src, v_uv + d).rgb + texture2D(u_src, v_uv - d).rgb) * u_weights[k];
    }
    FRAG_COLOR = vec4(sum, 0.0);
}
"#;

// AccGlare.fpo: the six blurred levels, each times its weight.
const ACCUMULATE_FRAG: &str = r#"
uniform sampler2D u_level0;
uniform sampler2D u_level1;
uniform sampler2D u_level2;
uniform sampler2D u_level3;
uniform sampler2D u_level4;
uniform sampler2D u_level5;
uniform float u_weight[6];
varying vec2 v_uv;
void main() {
    vec3 sum = texture2D(u_level0, v_uv).rgb * u_weight[0];
    sum += texture2D(u_level1, v_uv).rgb * u_weight[1];
    sum += texture2D(u_level2, v_uv).rgb * u_weight[2];
    sum += texture2D(u_level3, v_uv).rgb * u_weight[3];
    sum += texture2D(u_level4, v_uv).rgb * u_weight[4];
    sum += texture2D(u_level5, v_uv).rgb * u_weight[5];
    FRAG_COLOR = vec4(sum, 0.0);
}
"#;

// ToneApplyDisplay.fpo: the glare, stretched over the screen, less the noise again, added to the
// screen - here into the bar's own frame, at the scene's brightness and `u_alpha` over what is
// under it.
const DISPLAY_FRAG: &str = r#"
uniform sampler2D u_screen_tex;
uniform sampler2D u_glare;
uniform sampler2D u_noise;
uniform vec2 u_screen;
uniform float u_glare_weight;
uniform float u_glare_on;
uniform float u_fade;
uniform float u_alpha;
void main() {
    vec2 uv = gl_FragCoord.xy / u_screen;
    vec3 screen = texture2D(u_screen_tex, uv).rgb;
    vec3 glare = texture2D(u_glare, uv).rgb * u_glare_weight;
    vec3 noise = texture2D(u_noise, gl_FragCoord.xy / 32.0).rgb;
    FRAG_COLOR = vec4((screen + u_glare_on * clamp(glare - noise * 0.125, 0.0, 1.0)) * u_fade, u_alpha);
}
"#;

// particles_quads and particles_second: shared by both vertex programs - the lens terms (focus,
// size, the fade band) and the opacity. The view is never turned, so a normal is the same in
// the view as in the world.
const SPARKLE_VERT_COMMON: &str = r#"
attribute vec2 a_pos;
attribute vec4 a_place;
attribute vec4 a_turn;
uniform mat4 u_mvp;
uniform vec3 u_eye;
uniform vec3 u_spot;
uniform float u_spec_power;
uniform vec3 u_life_min;
uniform vec3 u_life_max;
uniform vec4 u_focus;
uniform vec3 u_focus_curves;
uniform vec3 u_size;
uniform vec2 u_darkness;
uniform vec4 u_near;
uniform vec2 u_transparency;
uniform vec4 u_glare;
varying vec4 v_uv_blur;
varying vec4 v_normal_alpha;
varying vec3 v_world;

vec3 axis_x(vec4 q) {
    return vec3(1.0 - 2.0 * (q.y * q.y + q.z * q.z), 2.0 * (q.x * q.y + q.w * q.z), 2.0 * (q.x * q.z - q.w * q.y));
}
vec3 axis_y(vec4 q) {
    return vec3(2.0 * (q.x * q.y - q.w * q.z), 1.0 - 2.0 * (q.x * q.x + q.z * q.z), 2.0 * (q.y * q.z + q.w * q.x));
}
vec3 axis_z(vec4 q) {
    return vec3(2.0 * (q.x * q.z + q.w * q.y), 2.0 * (q.y * q.z - q.w * q.x), 1.0 - 2.0 * (q.x * q.x + q.y * q.y));
}
float ramp(float x, float a, float b) { return clamp((x - a) / (b - a), 0.0, 1.0); }

vec3 lens_view;
float lens_near_blur;
float lens_far_blur;
float lens_near_curve;
float lens_far_curve;
float lens_size;
float lens_apparent;
float lens_band;
float lens_band_align;

void lens(vec3 p) {
    vec3 to_eye = u_eye - p;
    float dist = length(to_eye);
    lens_view = to_eye / dist;
    lens_near_blur = 1.0 - ramp(dist, u_focus.x, u_focus.y);
    lens_far_blur = ramp(dist, u_focus.z, u_focus.w);
    lens_near_curve = pow(max(lens_near_blur, 1e-10), u_focus_curves.x);
    lens_far_curve = pow(max(lens_far_blur, 1e-10), u_focus_curves.y);
    lens_size = mix(mix(u_size.x, u_size.y, lens_near_curve), u_size.z, lens_far_curve);
    lens_apparent = 2.0 * atan(lens_size / dist) / u_focus_curves.z;
    float ff = u_focus.z;
    lens_band = 1.0 + ramp(dist, ff - 0.2, ff) - ramp(dist, ff - 0.6, ff - 0.4);
    lens_band_align = ramp(dist, ff - 0.4, ff - 0.2);
}

float opacity(vec3 p, vec3 quad_normal) {
    vec3 wall = min(p - u_life_min, u_life_max - p);
    float s = clamp(5.0 * min(min(wall.x, wall.y), wall.z), 0.0, 1.0);
    float wall_fade = s * s * (3.0 - 2.0 * s);
    float facing = 1.0 - pow(max(1.0 - abs(dot(lens_view, quad_normal)), 1e-10), u_transparency.x);
    float far_dark = clamp(lens_far_blur * lens_apparent * u_darkness.y, 0.0, 1.0);
    float near_dark = clamp(lens_near_blur * lens_apparent * u_darkness.x, 0.0, 1.0);
    return facing * u_transparency.y * a_place.w * lens_band * (1.0 - far_dark) * (1.0 - near_dark) * wall_fade;
}
"#;

// particles_quads: the flake, turned towards the camera as it grows or blurs.
const SPARKLE_VERT_QUADS: &str = r#"
void main() {
    vec3 p = a_place.xyz;
    vec4 q = normalize(a_turn);
    lens(p);
    float align = clamp(lens_apparent * u_near.x + lens_near_curve * u_near.z + lens_band_align, 0.0, 1.0);
    vec4 qa = normalize(q + align * (vec4(0.0, 0.0, 0.0, 1.0) - q));
    vec2 c = a_pos - 0.5;
    vec3 world = p + (axis_x(qa) * c.x + axis_y(qa) * c.y) * lens_size;
    gl_Position = u_mvp * vec4(world, 1.0);
    v_uv_blur = vec4(a_pos, lens_near_curve, lens_far_curve);
    v_normal_alpha = vec4(axis_z(q), opacity(p, axis_z(qa)));
    v_world = world;
}
"#;

// particles_second: a camera-facing glare that only grows while the flake mirrors the spot into
// the eye.
const SPARKLE_VERT_GLARE: &str = r#"
void main() {
    vec3 p = a_place.xyz;
    vec4 q = normalize(a_turn);
    lens(p);
    vec3 n = axis_z(q);
    vec3 h = normalize(normalize(u_spot - p) + lens_view);
    float glint = pow(max(abs(dot(n, h)), 1e-10), u_spec_power);
    vec2 c = a_pos - 0.5;
    vec3 world = p + (vec3(1.0, 0.0, 0.0) * c.x + vec3(0.0, 1.0, 0.0) * c.y) * (lens_size * u_glare.y * glint);
    gl_Position = u_mvp * vec4(world, 1.0);
    v_uv_blur = vec4(a_pos, lens_near_curve, lens_far_curve);
    v_normal_alpha = vec4(n, opacity(p, vec3(0.0, 0.0, 1.0)));
    v_world = world;
}
"#;

const SPARKLE_FRAG_COMMON: &str = r#"
uniform sampler2D u_iridescent;
uniform vec3 u_eye;
uniform vec3 u_spot;
uniform vec3 u_attn;
uniform vec4 u_light;
uniform float u_iridescent_exp;
uniform float u_color_control;
uniform vec4 u_near;
uniform vec4 u_glare;
uniform float u_colour;
uniform float u_gamma;
varying vec4 v_uv_blur;
varying vec4 v_normal_alpha;
varying vec3 v_world;

vec3 iridescence(vec3 n) {
    vec3 c = texture2D(u_iridescent, n.xy * 0.5 + 0.5).rgb;
    return pow(mix(vec3(1.0), c, u_color_control), vec3(u_iridescent_exp));
}

float specular(vec3 n) {
    vec3 l = normalize(u_spot - v_world);
    vec3 v = normalize(u_eye - v_world);
    return pow(abs(dot(n, normalize(l + v))), u_light.w);
}

float attenuation() {
    float d = length(u_spot - v_world);
    return u_attn.x + (u_attn.y + u_attn.z * d) * d;
}
"#;

// Diffuse plus iridescent specular, exposed, on a disc that widens and softens with blur.
const SPARKLE_FRAG_QUADS: &str = r#"
void main() {
    vec3 n = v_normal_alpha.xyz;
    vec3 l = normalize(u_spot - v_world);
    vec3 light = (abs(dot(n, l)) * u_light.x + specular(n) * u_light.y * iridescence(n)) / attenuation();
    vec3 exposed = 1.0 - exp(-u_light.z * light);
    vec2 uv = v_uv_blur.xy;
    float blur = clamp(v_uv_blur.z + v_uv_blur.w, 0.0, 1.0);
    float rho = length(uv * 2.0 - 1.0);
    float e = clamp((rho - (0.45 - 0.6345 * blur)) / (0.1 + 1.269 * blur), 0.0, 1.0);
    float disc = 1.0 - e * e * (3.0 - 2.0 * e);
    float fuzz = mix(0.85, u_near.y, v_uv_blur.z);
    float shape = disc + blur * (1.0 - disc - exp(-disc * fuzz));
    vec3 color = exposed * (shape * v_normal_alpha.w);
    FRAG_COLOR = vec4(color * u_colour * u_gamma, 0.0);
}
"#;

// Iridescent specular only, falling off sharply from the sprite's centre.
const SPARKLE_FRAG_GLARE: &str = r#"
void main() {
    vec3 n = v_normal_alpha.xyz;
    float rho = min(length(v_uv_blur.xy * 2.0 - 1.0), 1.0);
    float falloff = exp(-u_glare.w * pow(rho, u_glare.z));
    vec3 light = specular(n) * u_light.y * iridescence(n) / attenuation();
    vec3 exposed = (1.0 - exp(-u_light.z * light)) * v_normal_alpha.w;
    FRAG_COLOR = vec4(exposed * u_colour * (falloff * u_glare.x * u_gamma), 0.0);
}
"#;

/// A texture with a framebuffer of its own.
struct Target {
    tex: gl::types::GLuint,
    fbo: gl::types::GLuint,
    w: i32,
    h: i32,
}

impl Target {
    fn new(w: i32, h: i32, float: bool) -> Option<Target> {
        let tex = texture(w, h, gl::LINEAR, gl::CLAMP_TO_EDGE, float, None);
        unsafe {
            let mut fbo = 0;
            gl::GenFramebuffers(1, &mut fbo);
            gl::BindFramebuffer(gl::FRAMEBUFFER, fbo);
            gl::FramebufferTexture2D(gl::FRAMEBUFFER, gl::COLOR_ATTACHMENT0, gl::TEXTURE_2D, tex, 0);
            let ok = gl::CheckFramebufferStatus(gl::FRAMEBUFFER) == gl::FRAMEBUFFER_COMPLETE;
            if !ok {
                gl::DeleteFramebuffers(1, &fbo);
                gl::DeleteTextures(1, &tex);
                return None;
            }
            gl::ClearColor(0.0, 0.0, 0.0, 0.0);
            gl::Clear(gl::COLOR_BUFFER_BIT);
            Some(Target { tex, fbo, w, h })
        }
    }

    fn bind(&self) {
        unsafe {
            gl::BindFramebuffer(gl::FRAMEBUFFER, self.fbo);
            gl::Viewport(0, 0, self.w, self.h);
        }
    }
}

impl Drop for Target {
    fn drop(&mut self) {
        unsafe {
            gl::DeleteFramebuffers(1, &self.fbo);
            gl::DeleteTextures(1, &self.tex);
        }
    }
}

/// An RGBA texture, in bytes or in half floats.
fn texture(
    w: i32,
    h: i32,
    filter: gl::types::GLenum,
    wrap: gl::types::GLenum,
    float: bool,
    data: Option<&[u8]>,
) -> gl::types::GLuint {
    let (internal, kind) = match (float, crate::gl::es()) {
        (false, true) => (gl::RGBA as i32, gl::UNSIGNED_BYTE),
        (false, false) => (gl::RGBA8 as i32, gl::UNSIGNED_BYTE),
        (true, true) => (gl::RGBA as i32, HALF_FLOAT_OES),
        (true, false) => (gl::RGBA16F as i32, gl::HALF_FLOAT),
    };
    unsafe {
        let mut tex = 0;
        gl::GenTextures(1, &mut tex);
        gl::BindTexture(gl::TEXTURE_2D, tex);
        gl::PixelStorei(gl::UNPACK_ALIGNMENT, 1);
        gl::TexImage2D(
            gl::TEXTURE_2D,
            0,
            internal,
            w,
            h,
            0,
            gl::RGBA,
            kind,
            data.map_or(std::ptr::null(), |d| d.as_ptr() as *const std::ffi::c_void),
        );
        gl::TexParameteri(gl::TEXTURE_2D, gl::TEXTURE_MIN_FILTER, filter as i32);
        gl::TexParameteri(gl::TEXTURE_2D, gl::TEXTURE_MAG_FILTER, filter as i32);
        gl::TexParameteri(gl::TEXTURE_2D, gl::TEXTURE_WRAP_S, wrap as i32);
        gl::TexParameteri(gl::TEXTURE_2D, gl::TEXTURE_WRAP_T, wrap as i32);
        tex
    }
}

/// Whether the GPU renders to half floats and filters them.
fn half_floats() -> bool {
    if !crate::gl::es() {
        return true;
    }
    let ext = unsafe { gl::GetString(gl::EXTENSIONS) };
    if ext.is_null() {
        return false;
    }
    let ext = unsafe { std::ffi::CStr::from_ptr(ext as *const std::ffi::c_char) }.to_string_lossy();
    let has = |name: &str| ext.split_ascii_whitespace().any(|e| e == name);
    has("GL_OES_texture_half_float")
        && has("GL_OES_texture_half_float_linear")
        && (has("GL_EXT_color_buffer_half_float") || has("GL_EXT_color_buffer_float"))
}

struct Program {
    id: gl::types::GLuint,
    names: Vec<(&'static str, gl::types::GLint)>,
}

impl Program {
    fn new(vert: &str, frag: &str, uniforms: &[&'static str]) -> Result<Program, GfxError> {
        let id = crate::shaders::program(vert, &format!("{HIGHP}{frag}"))?;
        let names = uniforms
            .iter()
            .map(|&n| (n, crate::gl::uniform_location(id, n)))
            .collect();
        Ok(Program { id, names })
    }

    fn at(&self, name: &str) -> gl::types::GLint {
        self.names.iter().find(|(n, _)| *n == name).map_or(-1, |n| n.1)
    }

    fn attrib(&self, name: &str) -> gl::types::GLuint {
        let Ok(c) = std::ffi::CString::new(name) else {
            return 0;
        };
        unsafe { gl::GetAttribLocation(self.id, c.as_ptr()).max(0) as gl::types::GLuint }
    }

    fn use_(&self) {
        unsafe { gl::UseProgram(self.id) };
    }

    fn f1(&self, name: &str, v: f32) {
        unsafe { gl::Uniform1f(self.at(name), v) };
    }

    fn f2(&self, name: &str, v: [f32; 2]) {
        unsafe { gl::Uniform2f(self.at(name), v[0], v[1]) };
    }

    fn f3(&self, name: &str, v: [f32; 3]) {
        unsafe { gl::Uniform3f(self.at(name), v[0], v[1], v[2]) };
    }

    fn f4(&self, name: &str, v: [f32; 4]) {
        unsafe { gl::Uniform4f(self.at(name), v[0], v[1], v[2], v[3]) };
    }

    fn i1(&self, name: &str, v: i32) {
        unsafe { gl::Uniform1i(self.at(name), v) };
    }
}

impl Drop for Program {
    fn drop(&mut self) {
        unsafe { gl::DeleteProgram(self.id) };
    }
}

fn bind_texture(unit: u32, tex: gl::types::GLuint) {
    unsafe {
        gl::ActiveTexture(gl::TEXTURE0 + unit);
        gl::BindTexture(gl::TEXTURE_2D, tex);
    }
}

/// _Stripes, 16 x 4, one stripe's profile across its period, row r for a step of the cell's size
/// on screen, as the scene's lines renderer makes it at start-up (in the PPU's single precision).
fn stripe_texels() -> Vec<u8> {
    let f = |v: f64| v as f32 as f64;
    let mut out = vec![0u8; 16 * 4 * 4];
    for r in 0..4 {
        let r3 = f(r as f64 / 3.0);
        let power = f(f(f(r3 * 20.0) * r3) * r3);
        let scale = f(r3 * 0.5 + 0.5);
        for c in 0..16 {
            let x = f(f(c as f64 / 15.0) - 0.5);
            let base = f(-4.0 * x * x + f(1.01));
            let v = (f(f(base.powf(power) * scale).min(1.0) * 255.0)).trunc() as u8;
            out[4 * (16 * r + c)..4 * (16 * r + c) + 4].copy_from_slice(&[v, v, v, 255]);
        }
    }
    out
}

/// _FresLUT, 512 x 1: red the rim light's weight, a Catmull-Rom curve fitted to the firmware's
/// table to 1.6%, and green, the even light's weight, 1 throughout.
fn fresnel_texels() -> Vec<u8> {
    const K: [[f64; 2]; 19] = [
        [0.0, 0.0],
        [14.0, 0.0],
        [17.0, 0.031],
        [19.0, 0.133],
        [21.0, 0.349],
        [22.0, 0.486],
        [24.0, 0.753],
        [26.0, 0.922],
        [29.0, 0.988],
        [32.0, 0.953],
        [34.0, 0.89],
        [36.0, 0.796],
        [38.0, 0.698],
        [43.0, 0.482],
        [48.0, 0.329],
        [58.0, 0.153],
        [77.0, 0.035],
        [120.0, 0.0],
        [511.0, 0.0],
    ];
    let mut out = vec![0u8; 512 * 4];
    for i in 0..512 {
        let x = i as f64;
        let mut j = 0;
        while j < K.len() - 2 && x > K[j + 1][0] {
            j += 1;
        }
        let t = (x - K[j][0]) / (K[j + 1][0] - K[j][0]);
        let p0 = K[j.saturating_sub(1)][1];
        let p1 = K[j][1];
        let p2 = K[j + 1][1];
        let p3 = K[(j + 2).min(K.len() - 1)][1];
        let v = 0.5
            * (2.0 * p1
                + (p2 - p0) * t
                + (2.0 * p0 - 5.0 * p1 + 4.0 * p2 - p3) * t * t
                + (3.0 * p1 - p0 - 3.0 * p2 + p3) * t * t * t);
        out[4 * i] = (255.0 * v.clamp(0.0, 1.0)).round() as u8;
        out[4 * i + 1] = 255;
        out[4 * i + 3] = 255;
    }
    out
}

/// The mesh's static parts: u is the line's index over 127, v fades the mesh out towards its edges.
fn mesh_texcoords() -> Vec<f32> {
    let n = MODERN_MESH;
    let edge = |i: usize| (10.0 * i.min(n - 1 - i) as f32 / (n - 1) as f32).min(1.0);
    let mut uv = vec![0.0f32; MODERN_VERTICES * 2];
    for iy in 0..n {
        for ix in 0..n {
            let o = 2 * (n * iy + ix);
            uv[o] = iy as f32 / (n - 1) as f32;
            uv[o + 1] = edge(ix) * edge(iy);
        }
    }
    uv
}

/// Strips between neighbouring lines, 17 columns at a time; the console restarts each, which
/// becomes two repeated indices - triangles that draw nothing.
fn mesh_indices() -> Vec<u16> {
    let n = MODERN_MESH;
    let mut out: Vec<u16> = Vec::new();
    for iy in 0..n - 1 {
        let mut first = 0;
        while first < n - 1 {
            let last = (first + 16).min(n - 1);
            let start = (n * iy + first) as u16;
            if let Some(&prev) = out.last() {
                out.push(prev);
                out.push(start);
            }
            for ix in first..=last {
                out.push((n * iy + ix) as u16);
                out.push((n * (iy + 1) + ix) as u16);
            }
            first += 16;
        }
    }
    out
}

fn buffer(kind: gl::types::GLenum, bytes: usize, data: *const std::ffi::c_void, usage: gl::types::GLenum) -> gl::types::GLuint {
    unsafe {
        let mut b = 0;
        gl::GenBuffers(1, &mut b);
        gl::BindBuffer(kind, b);
        gl::BufferData(kind, bytes as isize, data, usage);
        b
    }
}

/// The Modern background's programs, textures and targets.
pub struct ModernPass {
    wave: Program,
    back: Program,
    music: Program,
    copy: Program,
    composite: Program,
    source: Program,
    gaussian: Program,
    accumulate: Program,
    display: Program,
    sparkle_quads: Program,
    sparkle_glare: Program,
    vao: gl::types::GLuint,
    mesh_buf: gl::types::GLuint,
    uv_buf: gl::types::GLuint,
    index_buf: gl::types::GLuint,
    index_count: i32,
    sparkle_buf: gl::types::GLuint,
    sparkle_index: gl::types::GLuint,
    sparkle_quad: gl::types::GLuint,
    stripes: gl::types::GLuint,
    fres: gl::types::GLuint,
    noise_tex: gl::types::GLuint,
    noise: Vec<u8>,
    noise_count: u32,
    months: [gl::types::GLuint; 4],
    iridescent: gl::types::GLuint,
    back_target: Target,
    snapshot: Target,
    float: bool,
    glare: Option<Glare>,
    screen: Option<(Target, Target)>,
    frame: ModernFrame,
    months_at: u64,
    serial: Option<u64>,
    iridescent_set: bool,
}

struct Glare {
    pre: Target,
    pyramid: Vec<Target>,
    across: Vec<Target>,
    down: Vec<Target>,
    acc: Target,
}

impl Glare {
    fn new(float: bool) -> Option<Glare> {
        let mut g = Glare {
            pre: Target::new(GLARE_W, PRE_H, float)?,
            pyramid: Vec::new(),
            across: Vec::new(),
            down: Vec::new(),
            acc: Target::new(GLARE_W, GLARE_H, float)?,
        };
        for l in 0..LEVELS {
            let (w, h) = (GLARE_W >> l, GLARE_H >> l);
            g.pyramid.push(Target::new(w, h, float)?);
            g.across.push(Target::new(w, h, float)?);
            g.down.push(Target::new(w, h, float)?);
        }
        Some(g)
    }
}

impl ModernPass {
    pub fn new() -> Result<Self, GfxError> {
        let wave = Program::new(
            WAVE_VERT,
            WAVE_FRAG,
            &["u_mipmap_bias", "u_brightness", "u_fresnel", "u_stripes", "u_fres", "u_spacing", "u_thinness"],
        )?;
        let back = Program::new(
            FULL_VERT,
            BACK_FRAG,
            &[
                "u_day1",
                "u_day2",
                "u_night1",
                "u_night2",
                "u_day_time",
                "u_night_time",
                "u_month_time",
                "u_night_brightness",
                "u_night_day_blend",
                "u_alpha",
            ],
        )?;
        let music = Program::new(FULL_VERT, MUSIC_FRAG, &["u_day_time", "u_alpha"])?;
        let copy = Program::new(FULL_VERT, COPY_FRAG, &["u_src", "u_scale"])?;
        let composite = Program::new(
            FULL_VERT,
            COMPOSITE_FRAG,
            &[
                "u_back",
                "u_wave",
                "u_noise",
                "u_screen",
                "u_colour1",
                "u_colour2",
                "u_colour3",
                "u_colour4",
                "u_exposure",
                "u_white_sqr_rcp",
                "u_threshold",
                "u_gamma",
            ],
        )?;
        let source = Program::new(FULL_VERT, SOURCE_FRAG, &["u_src", "u_scale"])?;
        let gaussian = Program::new(FULL_VERT, GAUSSIAN_FRAG, &["u_src", "u_step", "u_weights"])?;
        let accumulate = Program::new(
            FULL_VERT,
            ACCUMULATE_FRAG,
            &["u_level0", "u_level1", "u_level2", "u_level3", "u_level4", "u_level5", "u_weight"],
        )?;
        let display = Program::new(
            FULL_VERT,
            DISPLAY_FRAG,
            &[
                "u_screen_tex",
                "u_glare",
                "u_noise",
                "u_screen",
                "u_glare_weight",
                "u_glare_on",
                "u_fade",
                "u_alpha",
            ],
        )?;
        let sparkle_uniforms = [
            "u_mvp",
            "u_eye",
            "u_spot",
            "u_spec_power",
            "u_life_min",
            "u_life_max",
            "u_focus",
            "u_focus_curves",
            "u_size",
            "u_darkness",
            "u_near",
            "u_transparency",
            "u_glare",
            "u_iridescent",
            "u_attn",
            "u_light",
            "u_iridescent_exp",
            "u_color_control",
            "u_colour",
            "u_gamma",
        ];
        let sparkle_quads = Program::new(
            &format!("{SPARKLE_VERT_COMMON}{SPARKLE_VERT_QUADS}"),
            &format!("{SPARKLE_FRAG_COMMON}{SPARKLE_FRAG_QUADS}"),
            &sparkle_uniforms,
        )?;
        let sparkle_glare = Program::new(
            &format!("{SPARKLE_VERT_COMMON}{SPARKLE_VERT_GLARE}"),
            &format!("{SPARKLE_FRAG_COMMON}{SPARKLE_FRAG_GLARE}"),
            &sparkle_uniforms,
        )?;

        let mut vao = 0;
        if !crate::gl::es() {
            unsafe {
                gl::GenVertexArrays(1, &mut vao);
                gl::BindVertexArray(vao);
            }
        }
        let mesh_buf = buffer(
            gl::ARRAY_BUFFER,
            MODERN_VERTICES * MODERN_RECORD * 4,
            std::ptr::null(),
            gl::DYNAMIC_DRAW,
        );
        let uv = mesh_texcoords();
        let uv_buf = buffer(gl::ARRAY_BUFFER, uv.len() * 4, uv.as_ptr() as _, gl::STATIC_DRAW);
        let indices = mesh_indices();
        let index_buf = buffer(
            gl::ELEMENT_ARRAY_BUFFER,
            indices.len() * 2,
            indices.as_ptr() as _,
            gl::STATIC_DRAW,
        );
        let sparkle_buf = buffer(
            gl::ARRAY_BUFFER,
            MODERN_SPARKLES * 4 * MODERN_SPARKLE_VERTEX * 4,
            std::ptr::null(),
            gl::DYNAMIC_DRAW,
        );
        let quads: Vec<u16> = (0..MODERN_SPARKLES as u16)
            .flat_map(|q| [4 * q, 4 * q + 1, 4 * q + 2, 4 * q + 2, 4 * q + 1, 4 * q + 3])
            .collect();
        let sparkle_index = buffer(gl::ELEMENT_ARRAY_BUFFER, quads.len() * 2, quads.as_ptr() as _, gl::STATIC_DRAW);
        let full: [f32; 8] = [0.0, 0.0, 1.0, 0.0, 0.0, 1.0, 1.0, 1.0];
        let sparkle_quad = buffer(gl::ARRAY_BUFFER, 32, full.as_ptr() as _, gl::STATIC_DRAW);
        if !crate::gl::es() {
            unsafe { gl::BindVertexArray(0) };
        }

        let stripes = texture(16, 4, gl::LINEAR, gl::CLAMP_TO_EDGE, false, Some(&stripe_texels()));
        unsafe { gl::TexParameteri(gl::TEXTURE_2D, gl::TEXTURE_WRAP_S, gl::REPEAT as i32) };
        let fres = texture(512, 1, gl::LINEAR, gl::CLAMP_TO_EDGE, false, Some(&fresnel_texels()));
        let noise = vec![0u8; NOISE_SIZE * NOISE_SIZE * 4];
        let noise_tex = texture(NOISE_SIZE as i32, NOISE_SIZE as i32, gl::NEAREST, gl::REPEAT, false, Some(&noise));
        let grey = vec![0u8; MODERN_BACK_W * MODERN_BACK_H * 4];
        let months = [0; 4].map(|_| {
            texture(MODERN_BACK_W as i32, MODERN_BACK_H as i32, gl::LINEAR, gl::CLAMP_TO_EDGE, false, Some(&grey))
        });
        let iridescent = texture(
            MODERN_IRIDESCENT as i32,
            MODERN_IRIDESCENT as i32,
            gl::LINEAR,
            gl::CLAMP_TO_EDGE,
            false,
            None,
        );
        let mut float = half_floats();
        let mut glare = None;
        if float {
            glare = Glare::new(true);
            float = glare.is_some();
        }
        if glare.is_none() {
            glare = Glare::new(false);
        }
        let back_target = Target::new(MODERN_BACK_W as i32, MODERN_BACK_H as i32, false)
            .ok_or(GfxError::Framebuffer(0))?;
        let snapshot = Target::new(MODERN_BACK_W as i32, MODERN_BACK_H as i32, false)
            .ok_or(GfxError::Framebuffer(0))?;
        unsafe { gl::BindFramebuffer(gl::FRAMEBUFFER, 0) };
        eprintln!(
            "slot: modern background ready, glare in {}",
            match (&glare, float) {
                (None, _) => "nothing",
                (Some(_), true) => "half floats",
                (Some(_), false) => "bytes",
            }
        );
        Ok(ModernPass {
            wave,
            back,
            music,
            copy,
            composite,
            source,
            gaussian,
            accumulate,
            display,
            sparkle_quads,
            sparkle_glare,
            vao,
            mesh_buf,
            uv_buf,
            index_buf,
            index_count: indices.len() as i32,
            sparkle_buf,
            sparkle_index,
            sparkle_quad,
            stripes,
            fres,
            noise_tex,
            noise,
            noise_count: 0,
            months,
            iridescent,
            back_target,
            snapshot,
            float,
            glare,
            screen: None,
            frame: ModernFrame::default(),
            months_at: 0,
            serial: None,
            iridescent_set: false,
        })
    }

    /// Takes the scene's frame: its mesh and sparkles go to the GPU, the rest is kept for `draw`.
    pub fn set_frame(&mut self, f: &ModernFrame) {
        unsafe {
            if f.mesh.len() >= MODERN_VERTICES * MODERN_RECORD {
                gl::BindBuffer(gl::ARRAY_BUFFER, self.mesh_buf);
                gl::BufferSubData(
                    gl::ARRAY_BUFFER,
                    0,
                    (MODERN_VERTICES * MODERN_RECORD * 4) as isize,
                    f.mesh.as_ptr() as _,
                );
            }
            let n = f.sparkle_count.min(MODERN_SPARKLES).min(f.sparkles.len() / (4 * MODERN_SPARKLE_VERTEX));
            if n > 0 {
                gl::BindBuffer(gl::ARRAY_BUFFER, self.sparkle_buf);
                gl::BufferSubData(
                    gl::ARRAY_BUFFER,
                    0,
                    (n * 4 * MODERN_SPARKLE_VERTEX * 4) as isize,
                    f.sparkles.as_ptr() as _,
                );
            }
        }
        if f.backdrop.textures != self.months_at && f.months.iter().all(|m| m.len() == MODERN_BACK_W * MODERN_BACK_H * 4) {
            self.months_at = f.backdrop.textures;
            for (tex, data) in self.months.iter().zip(&f.months) {
                unsafe {
                    gl::BindTexture(gl::TEXTURE_2D, *tex);
                    gl::PixelStorei(gl::UNPACK_ALIGNMENT, 1);
                    gl::TexSubImage2D(
                        gl::TEXTURE_2D,
                        0,
                        0,
                        0,
                        MODERN_BACK_W as i32,
                        MODERN_BACK_H as i32,
                        gl::RGBA,
                        gl::UNSIGNED_BYTE,
                        data.as_ptr() as _,
                    );
                }
            }
        }
        if !self.iridescent_set && f.iridescent.len() == MODERN_IRIDESCENT * MODERN_IRIDESCENT * 4 {
            self.iridescent_set = true;
            unsafe {
                gl::BindTexture(gl::TEXTURE_2D, self.iridescent);
                gl::PixelStorei(gl::UNPACK_ALIGNMENT, 1);
                gl::TexSubImage2D(
                    gl::TEXTURE_2D,
                    0,
                    0,
                    0,
                    MODERN_IRIDESCENT as i32,
                    MODERN_IRIDESCENT as i32,
                    gl::RGBA,
                    gl::UNSIGNED_BYTE,
                    f.iridescent.as_ptr() as _,
                );
            }
        }
        self.frame.sparkle_count = f.sparkle_count.min(MODERN_SPARKLES);
        self.frame.wave = f.wave;
        self.frame.post = f.post;
        self.frame.backdrop = f.backdrop;
        self.frame.particles = f.particles;
        self.frame.fade = f.fade;
    }

    fn screen_targets(&mut self, w: i32, h: i32) -> bool {
        if self.screen.as_ref().is_some_and(|(s, _)| s.w == w && s.h == h) {
            return true;
        }
        self.screen = None;
        let (Some(wave), Some(screen)) = (Target::new(w, h, false), Target::new(w, h, false)) else {
            return false;
        };
        self.screen = Some((wave, screen));
        true
    }

    fn full(&self, quad: &crate::quad::Quad) {
        quad.draw();
    }

    /// Draws the frame into `target`, `size` pixels, `alpha` over what is there.
    pub fn draw(&mut self, target: gl::types::GLuint, size: (u32, u32), alpha: f32, quad: &crate::quad::Quad) {
        let (w, h) = (size.0 as i32, size.1 as i32);
        if w == 0 || h == 0 || !self.screen_targets(w, h) {
            return;
        }
        let f = self.frame.clone();
        unsafe {
            gl::Disable(gl::BLEND);
            gl::Disable(gl::DEPTH_TEST);
            gl::Disable(gl::CULL_FACE);
            gl::Disable(gl::SCISSOR_TEST);
        }
        self.draw_backdrop(&f.backdrop, quad);
        self.draw_wave(&f.wave);
        self.present(&f, target, (w, h), alpha, quad);
        self.draw_sparkles(&f, target, (w, h), alpha);
        unsafe {
            gl::BindFramebuffer(gl::FRAMEBUFFER, target);
            gl::Viewport(0, 0, w, h);
            gl::Disable(gl::BLEND);
            gl::ColorMask(gl::TRUE, gl::TRUE, gl::TRUE, gl::TRUE);
            gl::ActiveTexture(gl::TEXTURE0);
        }
    }

    /// The backdrop, into its 64 x 32 buffer: a new moment starts an ease from a copy of the
    /// buffer as it stands, and every frame draws that copy and the program over it by its ease.
    fn draw_backdrop(&mut self, b: &ModernBackdrop, quad: &crate::quad::Quad) {
        if self.serial != Some(b.serial) {
            let first = self.serial.is_none();
            self.serial = Some(b.serial);
            if !first {
                self.snapshot.bind();
                self.copy.use_();
                self.copy.i1("u_src", 0);
                self.copy.f1("u_scale", 1.0);
                bind_texture(0, self.back_target.tex);
                self.full(quad);
            }
        }
        self.back_target.bind();
        self.copy.use_();
        self.copy.i1("u_src", 0);
        self.copy.f1("u_scale", 1.0);
        bind_texture(0, self.snapshot.tex);
        self.full(quad);
        let p = if b.shader == 1 { &self.music } else { &self.back };
        p.use_();
        p.f1("u_day_time", b.day_time);
        p.f1("u_alpha", b.alpha);
        if b.shader != 1 {
            p.f1("u_night_time", b.night_time);
            p.f1("u_month_time", b.month_time);
            p.f1("u_night_brightness", b.night_brightness);
            p.f1("u_night_day_blend", b.night_day_blend);
            for (i, name) in ["u_day1", "u_day2", "u_night1", "u_night2"].iter().enumerate() {
                p.i1(name, i as i32);
                bind_texture(i as u32, self.months[i]);
            }
        }
        unsafe {
            gl::Enable(gl::BLEND);
            gl::BlendFunc(gl::SRC_ALPHA, gl::ONE_MINUS_SRC_ALPHA);
        }
        self.full(quad);
        unsafe { gl::Disable(gl::BLEND) };
    }

    /// The wave adds its light into its own buffer, red and green only.
    fn draw_wave(&self, wv: &ModernWave) {
        let Some((wave, _)) = &self.screen else { return };
        wave.bind();
        let p = &self.wave;
        unsafe {
            gl::ClearColor(0.0, 0.0, 0.0, 0.0);
            gl::Clear(gl::COLOR_BUFFER_BIT);
            gl::Enable(gl::BLEND);
            gl::BlendFunc(gl::ONE, gl::ONE);
            gl::ColorMask(gl::TRUE, gl::TRUE, gl::FALSE, gl::FALSE);
            p.use_();
            p.f1("u_mipmap_bias", wv.mipmap_bias);
            p.f1("u_brightness", wv.brightness);
            p.f1("u_fresnel", wv.fresnel);
            p.f1("u_spacing", wv.spacing);
            p.f1("u_thinness", wv.thinness);
            p.i1("u_stripes", 0);
            p.i1("u_fres", 1);
            bind_texture(0, self.stripes);
            bind_texture(1, self.fres);
            if !crate::gl::es() {
                gl::BindVertexArray(self.vao);
            }
            let (pos, normal, uv) = (p.attrib("a_pos"), p.attrib("a_normal"), p.attrib("a_uv"));
            let record = (MODERN_RECORD * 4) as i32;
            gl::BindBuffer(gl::ARRAY_BUFFER, self.mesh_buf);
            gl::EnableVertexAttribArray(pos);
            gl::VertexAttribPointer(pos, 4, gl::FLOAT, gl::FALSE, record, std::ptr::null());
            gl::EnableVertexAttribArray(normal);
            gl::VertexAttribPointer(normal, 4, gl::FLOAT, gl::FALSE, record, 16 as *const _);
            gl::BindBuffer(gl::ARRAY_BUFFER, self.uv_buf);
            gl::EnableVertexAttribArray(uv);
            gl::VertexAttribPointer(uv, 2, gl::FLOAT, gl::FALSE, 0, std::ptr::null());
            gl::BindBuffer(gl::ELEMENT_ARRAY_BUFFER, self.index_buf);
            gl::DrawElements(gl::TRIANGLE_STRIP, self.index_count, gl::UNSIGNED_SHORT, std::ptr::null());
            gl::DisableVertexAttribArray(normal);
            gl::DisableVertexAttribArray(uv);
            gl::BindBuffer(gl::ELEMENT_ARRAY_BUFFER, 0);
            if !crate::gl::es() {
                gl::BindVertexArray(0);
            }
            gl::ColorMask(gl::TRUE, gl::TRUE, gl::TRUE, gl::TRUE);
            gl::Disable(gl::BLEND);
        }
    }

    /// The composite, the tone curve and the glare, into `target`.
    fn present(&mut self, f: &ModernFrame, target: gl::types::GLuint, (w, h): (i32, i32), alpha: f32, quad: &crate::quad::Quad) {
        self.noise_count = fill_noise(&mut self.noise, self.noise_count, 1.0 / 255.0);
        unsafe {
            gl::BindTexture(gl::TEXTURE_2D, self.noise_tex);
            gl::PixelStorei(gl::UNPACK_ALIGNMENT, 1);
            gl::TexSubImage2D(
                gl::TEXTURE_2D,
                0,
                0,
                0,
                NOISE_SIZE as i32,
                NOISE_SIZE as i32,
                gl::RGBA,
                gl::UNSIGNED_BYTE,
                self.noise.as_ptr() as _,
            );
        }
        let Some((wave, screen)) = &self.screen else { return };
        let u = &f.post;
        screen.bind();
        let c = &self.composite;
        c.use_();
        c.i1("u_back", 0);
        c.i1("u_wave", 1);
        c.i1("u_noise", 2);
        c.f2("u_screen", [w as f32, h as f32]);
        for (i, name) in ["u_colour1", "u_colour2", "u_colour3", "u_colour4"].iter().enumerate() {
            c.f3(name, u.colours[i]);
        }
        c.f1("u_exposure", u.exposure);
        c.f1("u_white_sqr_rcp", u.white_sqr_rcp);
        c.f1("u_threshold", u.threshold);
        c.f1("u_gamma", u.gamma);
        bind_texture(0, self.back_target.tex);
        bind_texture(1, wave.tex);
        bind_texture(2, self.noise_tex);
        self.full(quad);

        // The glare's chain holds 8 times the screen's light at most; in bytes it is kept at an
        // eighth.
        let scale = if self.float { 1.0 } else { 0.125 };
        if let Some(g) = &self.glare {
            g.pre.bind();
            self.source.use_();
            self.source.i1("u_src", 0);
            self.source.f1("u_scale", scale);
            bind_texture(0, screen.tex);
            self.full(quad);
            self.copy.use_();
            self.copy.i1("u_src", 0);
            self.copy.f1("u_scale", 1.0);
            g.pyramid[0].bind();
            bind_texture(0, g.pre.tex);
            self.full(quad);
            for l in 1..LEVELS {
                g.pyramid[l].bind();
                bind_texture(0, g.pyramid[l - 1].tex);
                self.full(quad);
            }
            let gp = &self.gaussian;
            gp.use_();
            gp.i1("u_src", 0);
            unsafe { gl::Uniform3fv(gp.at("u_weights"), 8, u.gauss.as_ptr()) };
            for l in 0..LEVELS {
                let t = &g.pyramid[l];
                g.across[l].bind();
                gp.f2("u_step", [1.0 / t.w as f32, 0.0]);
                bind_texture(0, t.tex);
                self.full(quad);
                g.down[l].bind();
                gp.f2("u_step", [0.0, 1.0 / t.h as f32]);
                bind_texture(0, g.across[l].tex);
                self.full(quad);
            }
            g.acc.bind();
            let a = &self.accumulate;
            a.use_();
            for l in 0..LEVELS {
                a.i1(&format!("u_level{l}"), l as i32);
                bind_texture(l as u32, g.down[l].tex);
            }
            unsafe { gl::Uniform1fv(a.at("u_weight"), LEVELS as i32, u.levels.as_ptr()) };
            self.full(quad);
        }

        unsafe {
            gl::BindFramebuffer(gl::FRAMEBUFFER, target);
            gl::Viewport(0, 0, w, h);
        }
        let d = &self.display;
        d.use_();
        d.i1("u_screen_tex", 0);
        d.i1("u_glare", 1);
        d.i1("u_noise", 2);
        d.f2("u_screen", [w as f32, h as f32]);
        d.f1("u_glare_weight", u.glare_weight / scale);
        d.f1("u_glare_on", if self.glare.is_some() { 1.0 } else { 0.0 });
        d.f1("u_fade", f.fade);
        d.f1("u_alpha", alpha);
        bind_texture(0, screen.tex);
        bind_texture(1, self.glare.as_ref().map_or(self.noise_tex, |g| g.acc.tex));
        bind_texture(2, self.noise_tex);
        unsafe {
            if alpha < 1.0 {
                gl::Enable(gl::BLEND);
                gl::BlendFunc(gl::SRC_ALPHA, gl::ONE_MINUS_SRC_ALPHA);
            }
        }
        self.full(quad);
        unsafe { gl::Disable(gl::BLEND) };
    }

    /// The sparkles over the screen, in the XMB's two passes, adding their light.
    fn draw_sparkles(&self, f: &ModernFrame, target: gl::types::GLuint, (w, h): (i32, i32), alpha: f32) {
        let n = f.sparkle_count;
        if n == 0 {
            return;
        }
        let s = &f.particles;
        unsafe {
            gl::BindFramebuffer(gl::FRAMEBUFFER, target);
            gl::Viewport(0, 0, w, h);
            gl::Enable(gl::BLEND);
            gl::BlendFunc(gl::ONE, gl::ONE);
            gl::ColorMask(gl::TRUE, gl::TRUE, gl::TRUE, gl::FALSE);
            if !crate::gl::es() {
                gl::BindVertexArray(self.vao);
            }
            bind_texture(0, self.iridescent);
        }
        for p in [&self.sparkle_quads, &self.sparkle_glare] {
            p.use_();
            unsafe { gl::UniformMatrix4fv(p.at("u_mvp"), 1, gl::FALSE, s.mvp.as_ptr()) };
            p.f3("u_eye", s.eye);
            p.f3("u_spot", s.spot);
            p.f3("u_attn", s.attn);
            p.f4("u_light", s.light);
            p.f1("u_spec_power", s.light[3]);
            p.f3("u_life_min", s.life_min);
            p.f3("u_life_max", s.life_max);
            p.f4("u_focus", s.focus);
            p.f3("u_focus_curves", s.focus_curves);
            p.f3("u_size", s.size);
            p.f2("u_darkness", s.darkness);
            p.f4("u_near", [s.near[0], s.near[1], s.near[2], 0.0]);
            p.f2("u_transparency", s.transparency);
            p.f4("u_glare", s.glare);
            p.f1("u_iridescent_exp", s.iridescent_exp);
            p.f1("u_color_control", s.color_control);
            p.f1("u_colour", s.colour * f.fade * alpha);
            p.f1("u_gamma", s.gamma);
            p.i1("u_iridescent", 0);
            let stride = (MODERN_SPARKLE_VERTEX * 4) as i32;
            let (corner, place, turn) = (p.attrib("a_pos"), p.attrib("a_place"), p.attrib("a_turn"));
            unsafe {
                gl::BindBuffer(gl::ARRAY_BUFFER, self.sparkle_buf);
                gl::EnableVertexAttribArray(corner);
                gl::VertexAttribPointer(corner, 2, gl::FLOAT, gl::FALSE, stride, std::ptr::null());
                gl::EnableVertexAttribArray(place);
                gl::VertexAttribPointer(place, 4, gl::FLOAT, gl::FALSE, stride, 8 as *const _);
                gl::EnableVertexAttribArray(turn);
                gl::VertexAttribPointer(turn, 4, gl::FLOAT, gl::FALSE, stride, 24 as *const _);
                gl::BindBuffer(gl::ELEMENT_ARRAY_BUFFER, self.sparkle_index);
                gl::DrawElements(gl::TRIANGLES, (n * 6) as i32, gl::UNSIGNED_SHORT, std::ptr::null());
                gl::DisableVertexAttribArray(place);
                gl::DisableVertexAttribArray(turn);
                gl::BindBuffer(gl::ELEMENT_ARRAY_BUFFER, 0);
            }
        }
        unsafe {
            if !crate::gl::es() {
                gl::BindVertexArray(0);
            }
            gl::ColorMask(gl::TRUE, gl::TRUE, gl::TRUE, gl::TRUE);
            gl::Disable(gl::BLEND);
        }
        let _ = self.sparkle_quad;
    }
}

impl Drop for ModernPass {
    fn drop(&mut self) {
        unsafe {
            for b in [self.mesh_buf, self.uv_buf, self.index_buf, self.sparkle_buf, self.sparkle_index, self.sparkle_quad] {
                gl::DeleteBuffers(1, &b);
            }
            for t in [self.stripes, self.fres, self.noise_tex, self.iridescent] {
                gl::DeleteTextures(1, &t);
            }
            gl::DeleteTextures(4, self.months.as_ptr());
            if self.vao != 0 {
                gl::DeleteVertexArrays(1, &self.vao);
            }
        }
    }
}

/// preexpose_Noise's texels as the HDR renderer fills them each frame: texel i takes the integer
/// hash of the counter plus i + 1, as 255 min(1, 8 DITHER u), truncated - 0 to 7 at DITHER's 1/255.
fn fill_noise(out: &mut [u8], count: u32, dither: f64) -> u32 {
    let f = |v: f64| v as f32 as f64;
    for i in 0..NOISE_SIZE * NOISE_SIZE {
        let n = count.wrapping_add(i as u32 + 1);
        let x = (n << 13) ^ n;
        let h = x
            .wrapping_mul(x.wrapping_mul(x).wrapping_mul(15731).wrapping_add(789221))
            .wrapping_add(1376312589)
            & 0x7fffffff;
        let u = f(f(f(h as f64) * -9.313225746154785e-10 + 1.0) * 0.5 + 0.5);
        let v = f(f(f(u * f(dither)) * 8.0).clamp(-1.0, 1.0) * 255.0).trunc() as u8;
        out[4 * i] = v;
        out[4 * i + 1] = v;
        out[4 * i + 2] = v;
        out[4 * i + 3] = 0;
    }
    count.wrapping_add((NOISE_SIZE * NOISE_SIZE) as u32)
}
