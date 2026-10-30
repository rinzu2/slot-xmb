//! The bar's Original and Classic backgrounds drawn by the GPU program the handheld runs, through
//! whatever GLES 2 the machine has, against the same maths on the CPU (`backdrop_shade`).
#![cfg(target_os = "linux")]

use slot_gfx::{Compositor, Draw, HeadlessSurface};

fn cases() -> Vec<(String, u8, u32, f32, [f32; 4], bool)> {
    let mut v = Vec::new();
    for colour in [0u32, 3, 7, 12, 19, 20, 25, 30, 33] {
        for (style, name) in [(slot_gfx::STYLE_ORIGINAL, "orig"), (slot_gfx::STYLE_CLASSIC, "classic")] {
            for t in [3.0f32, 41.5] {
                v.push((format!("{name}_{colour}_{t}"), style, colour, t, slot_gfx::BACKDROP_PLAIN, false));
            }
        }
    }
    for t in [2.55f32, 2.62, 2.8, 3.3] {
        let shape = slot_ui::xmb_intro_shape(t).unwrap();
        v.push((format!("intro_{t}"), slot_gfx::STYLE_ORIGINAL, 20, t, shape, false));
        v.push((format!("intro_classic_{t}"), slot_gfx::STYLE_CLASSIC, 30, t, shape, false));
    }
    v.push(("ground".into(), slot_gfx::STYLE_ORIGINAL, 19, 7.0, slot_gfx::BACKDROP_PLAIN, true));
    v.push(("ground_classic".into(), slot_gfx::STYLE_CLASSIC, 19, 7.0, slot_gfx::BACKDROP_PLAIN, true));
    v
}

fn render(c: &mut Compositor, window: (u32, u32), style: u8, colour: u32, t: f32, shape: [f32; 4], ground: bool) -> Vec<u8> {
    let mut curves = Vec::new();
    slot_ui::xmb_curves_at(style, t, None, &mut curves);
    let tex = c.create_texture_nearest(slot_gfx::BACKDROP_COLS as u32, slot_gfx::BACKDROP_ROWS as u32, &curves);
    let ground = ground.then(|| {
        let (w, h) = (48u32, 32u32);
        let px: Vec<u8> = (0..w * h).flat_map(|i| [(i % w * 5) as u8, (i / w * 7) as u8, 120, 255]).collect();
        c.create_texture(w, h, &px)
    });
    c.fit(window);
    c.begin_frame();
    c.draw_list(&[Draw::Backdrop(slot_gfx::Backdrop {
        curves: tex,
        style,
        colours: slot_ui::xmb_backdrop_colours(colour),
        edges: slot_ui::xmb_backdrop_edges(colour),
        shape,
        alpha: 1.0,
        ground,
    })]);
    c.read_frame()
}

/// The program and the CPU's `backdrop_shade` give each pixel the same colour, to within a
/// shade: the colours across the screen the program reads from its table are the CPU's own.
#[test]
fn the_program_draws_what_the_cpu_works_out() {
    let Ok(surface) = HeadlessSurface::new() else { return };
    let Ok(mut c) = Compositor::new(&surface) else { return };
    let mut worst = 0.0f32;
    for window in [(640u32, 480u32), (720, 480)] {
        let (w, h) = (window.0 as usize, window.1 as usize);
        for (name, style, colour, t, shape, ground) in cases() {
            if ground {
                continue;
            }
            let px = render(&mut c, window, style, colour, t, shape, false);
            let mut curves = Vec::new();
            slot_ui::xmb_curves_at(style, t, None, &mut curves);
            let colours = slot_ui::xmb_backdrop_colours(colour);
            let edges = slot_ui::xmb_backdrop_edges(colour);
            // Rows away from the waves' edges, where a pixel's colour does not hang on which side
            // of a curve's sample it falls.
            for y in (7..h).step_by(29) {
                for x in (3..w).step_by(17) {
                    let (u, v) = ((x as f32 + 0.5) / w as f32, (y as f32 + 0.5) / h as f32);
                    let want = slot_gfx::backdrop_shade(&curves, style, &colours, &edges, shape, u, v, h as f32);
                    let o = (y * w + x) * 4;
                    let got = [px[o], px[o + 1], px[o + 2]].map(|b| b as f32 / 255.0);
                    let d = (0..3).map(|i| (got[i] - want[i]).abs()).fold(0.0, f32::max);
                    if d > 6.0 / 255.0 {
                        // Next to an edge the CPU's column and the GPU's can fall either side.
                        let near = |row: usize, second: bool| {
                            let col = ((u * slot_gfx::BACKDROP_COLS as f32) as usize).min(slot_gfx::BACKDROP_COLS - 1);
                            let i = (row * slot_gfx::BACKDROP_COLS + col) * 4 + if second { 2 } else { 0 };
                            (slot_gfx::backdrop_decode(curves[i], curves[i + 1]) - v).abs() * h as f32 <= 12.0
                        };
                        if (0..3).any(|r| near(r, false) || near(r, true)) {
                            continue;
                        }
                        panic!("{name} at {x},{y} in {w}x{h}: {got:?} against {want:?}");
                    }
                    worst = worst.max(d);
                }
            }
        }
    }
    eprintln!("worst difference {:.1} shades", worst * 255.0);
}
