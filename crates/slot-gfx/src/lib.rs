macro_rules! eprintln {
    ($($arg:tt)*) => {{
        use std::io::Write as _;
        let _ = writeln!(std::io::stderr(), $($arg)*);
    }};
}

mod backdrop;
mod bezel;
mod draw;
mod fbdev;
mod fbo;
mod gl;
mod grade;
#[cfg(target_os = "macos")]
mod headless;
#[cfg(all(target_os = "linux", feature = "test-support"))]
mod headless_egl;
#[cfg(feature = "host")]
mod host;
mod layout;
mod lcd3x;
mod modern;
mod pipeline;
mod power;
mod quad;
mod shaders;
mod surface;

pub use backdrop::{
    decode as backdrop_decode, encode as backdrop_encode, fragment_source as backdrop_fragment_source,
    put as backdrop_put, shade as backdrop_shade, Backdrop, BackdropColours, BackdropEdges, BACKDROP_COLS, BACKDROP_PLAIN,
    BACKDROP_ROWS, LINES as BACKDROP_LINES, ROW_EDGES, ROW_LINES, STYLE_CLASSIC,
    STYLE_MODERN, STYLE_ORIGINAL,
};
pub use bezel::{framed, game_rect, lifted_rect, screen_rect};
pub use draw::{Draw, TexId};
pub use fbdev::{egl_error, panel_mode, panel_size, FbdevSurface};
pub use fbo::{canvas_size, Compositor, BACKDROP};
pub use grade::{blue_light_gain, BLUE_LIGHT_MAX};
#[cfg(target_os = "macos")]
pub use headless::HeadlessSurface;
#[cfg(all(target_os = "linux", feature = "test-support"))]
pub use headless_egl::HeadlessSurface;
#[cfg(feature = "host")]
pub use host::HostSurface;
pub use layout::Canvas;
pub use modern::{
    ModernBackdrop, ModernFrame, ModernPost, ModernSparkles, ModernWave, MODERN_BACK_H, MODERN_BACK_W,
    MODERN_IRIDESCENT, MODERN_MESH, MODERN_RECORD, MODERN_SPARKLES, MODERN_SPARKLE_VERTEX, MODERN_VERTICES,
};
pub use lcd3x::{lcd3x_factors, lcd3x_mask, ScreenEffect};
pub use pipeline::{SCALE, SRC_H, SRC_W, WHOLE_TEXTURE};
pub use power::{screen_brightness, screen_rect_above, screen_scale, screen_width};
pub use surface::{blit_rect, blit_rect_fit, fit_rect, fit_scale, GfxError, Surface, OUT_H, OUT_W};
