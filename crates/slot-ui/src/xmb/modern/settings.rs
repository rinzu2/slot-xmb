//! The scene's settings, under faquino's camelCase names with the firmware's values: LINE1.mnu's,
//! HDR.mnu's and BACKGROUND.mnu's for the wave, PARTICLES.mnu's and PARTICLES_UI.mnu's for the
//! sparkles. Ported from Nagare's `settings.rs` (MIT, see `LICENSE-MIT`).

/// Name-based access, which the scene's sets go through.
pub trait Params {
    fn get(&self, key: &str) -> Option<f64>;
    fn get_mut(&mut self, key: &str) -> Option<&mut f64>;
    fn keys() -> &'static [&'static str];
}

macro_rules! settings_struct {
    ($ty:ident { $( $(#[$m:meta])* $field:ident : $key:literal = $default:expr, )* }) => {
        #[derive(Clone, Debug, PartialEq)]
        pub struct $ty { $( pub $field: f64, )* }
        impl Default for $ty {
            fn default() -> Self { Self { $( $field: $default, )* } }
        }
        impl Params for $ty {
            fn get(&self, key: &str) -> Option<f64> {
                match key { $( $key => Some(self.$field), )* _ => None }
            }
            fn get_mut(&mut self, key: &str) -> Option<&mut f64> {
                match key { $( $key => Some(&mut self.$field), )* _ => None }
            }
            fn keys() -> &'static [&'static str] { &[ $( $key, )* ] }
        }
    };
}

settings_struct!(WaveSettings {
    // LINE1.mnu, the base set, which the scene's sets are differences from.
    damping: "damping" = 0.0001,
    length: "length" = 0.306001,
    tension: "tension" = 0.25,
    timestep: "timestep" = 4.0,
    perturbation: "perturbation" = 0.0998587,
    end_x: "endX" = 3.0,
    end_y: "endY" = 0.4,
    end_z: "endZ" = 0.2,
    pos_x: "posX" = -7.67933,
    pos_y: "posY" = -1.08844,
    pos_z: "posZ" = -6.40287,
    ang_x: "angX" = -0.994157,
    ang_y: "angY" = 0.0867576,
    ang_z: "angZ" = 0.065364,
    ang_rot: "angRot" = 18.1208,
    ffd_scale1_x: "ffdScale1X" = 5.67726,
    ffd_scale1_y: "ffdScale1Y" = 1.00077,
    ffd_scale1_z: "ffdScale1Z" = 1.0,
    ffd_scale2_x: "ffdScale2X" = 2.82755,
    ffd_scale2_y: "ffdScale2Y" = 1.27579,
    ffd_scale2_z: "ffdScale2Z" = 2.88782,
    ffd_offset_x: "ffdOffsetX" = 0.0,
    ffd_offset_y: "ffdOffsetY" = -0.469999,
    ffd_offset_z: "ffdOffsetZ" = 0.0,
    ffd_param1: "ffdParam1" = -1.33509,
    brightness: "brightness" = 0.701917,
    mipmap_bias: "mipmapBias" = 1.86707,
    fresnel: "fresnel" = 0.638971,
    falloff: "falloff" = 1.00318,
    spacing: "spacing" = 407.658,
    thinness: "thinness" = 1.0,
    // HDR.mnu, the base set; DITHER is the code's default, which the files that set it set too.
    exposure: "exposure" = 1.05,
    white_level: "whiteLevel" = 0.899181,
    glare_level: "glareLevel" = 1.10245,
    glare_thresh: "glareThresh" = 0.738857,
    gaussian_rad_r: "gaussianRadR" = 1.23888,
    gaussian_rad_g: "gaussianRadG" = 1.43176,
    gaussian_rad_b: "gaussianRadB" = 1.55787,
    glare_sum_pow: "glareSumPow" = 0.557478,
    dither: "dither" = 0.00392157,
    // BACKGROUND.mnu's four colours: 1 and 2 the backdrop's at the top and the bottom, 3 and 4 the wave's at the left
    // and the right.
    colour1_red: "colour1Red" = 1.0,
    colour1_green: "colour1Green" = 1.0,
    colour1_blue: "colour1Blue" = 1.0,
    colour2_red: "colour2Red" = 1.0,
    colour2_green: "colour2Green" = 1.0,
    colour2_blue: "colour2Blue" = 1.0,
    colour3_red: "colour3Red" = 0.847347,
    colour3_green: "colour3Green" = 0.846155,
    colour3_blue: "colour3Blue" = 0.846448,
    colour4_red: "colour4Red" = 0.92487,
    colour4_green: "colour4Green" = 0.922603,
    colour4_blue: "colour4Blue" = 0.922933,
    // BACKGROUND.mnu's backdrop: the base file leaves these out, so the code's defaults stand for them. DAYSPREAD's, 0,
    // lies below its range, which the scene holds it to, so it reads as 1.
    night_blend: "nightBlend" = 1.0,
    night2day_begin: "night2dayBegin" = 4.0,
    night2day_end: "night2dayEnd" = 6.0,
    day2night_begin: "day2nightBegin" = 18.0,
    day2night_end: "day2nightEnd" = 20.0,
    dayspread: "dayspread" = 0.0,
    night_whit_bias: "nightWhitBias" = 0.5,
    colour_shader: "colourShader" = 0.0,
});

settings_struct!(ParticleSettings {
    // PARTICLES.mnu (firmware 4.93 defaults)
    emit_vel_min: "emitVelMin" = 0.15064,
    emit_vel_mul: "emitVelMul" = 0.19,
    emit_vel_var: "emitVelVar" = 0.282567,
    emit_cone_angle: "emitConeAngle" = 51.8695,
    emit_neg_prob: "emitNegProb" = 0.173899,
    emit_vel_zscale: "emitVelZscale" = 0.0,
    emit_per_frame: "emitPerFrame" = 16.6539,
    emit_prob: "emitProb" = 0.479115,
    aging_speed: "agingSpeed" = 0.00285223,
    aging_variance: "agingVariance" = 0.493003,
    friction: "friction" = 0.030551,
    spin_time_scale: "spinTimeScale" = 2.74,
    delta_time: "deltaTime" = 0.0088883,
    gravity: "gravity" = -6.8e-05,
    wind_dir_x: "windDirX" = 0.340188,
    wind_dir_y: "windDirY" = 0.0,
    wind_dir_z: "windDirZ" = 0.35,
    wind_scale: "windScale" = 0.0,
    wind_scale10: "windScale10" = 0.0,
    brownian_scale: "brownianScale" = 0.225311,
    spot_pos_x: "spotPosX" = 4.16,
    spot_pos_y: "spotPosY" = 2.63,
    spot_pos_z: "spotPosZ" = -7.6,
    spot_attn_x: "spotAttnX" = 1.0,
    spot_attn_y: "spotAttnY" = 0.0,
    spot_attn_z: "spotAttnZ" = 0.0,
    specular_power: "specularPower" = 35.2904,
    specular_coeff: "specularCoeff" = 74.74,
    lambert_coeff: "lambertCoeff" = 8.0,
    exposure: "exposure" = 0.0390458,
    fresnel: "fresnel" = 1.33319,
    color_control: "colorControl" = 1.0,
    iridescent_exp: "iridescentExp" = 1.0,
    global_alpha: "globalAlpha" = 1.0,
    size_middle: "sizeMiddle" = 0.0536233,
    size_near: "sizeNear" = 0.0772033,
    size_far: "sizeFar" = 0.874062,
    near_focus: "nearFocus" = 6.12435,
    near_focus_dist: "nearFocusDist" = 1.3193,
    near_focus_pow: "nearFocusPow" = 2.47206,
    near_darkness: "nearDarkness" = 6.24028,
    near_fuzziness: "nearFuzziness" = 3.79,
    far_focus: "farFocus" = 12.8327,
    far_focus_dist: "farFocusDist" = 3.54129,
    far_focus_pow: "farFocusPow" = 1.83324,
    far_darkness: "farDarkness" = 13.9,
    near_align: "nearAlign" = 0.722,
    size_align: "sizeAlign" = 13.19,
    glare: "glare" = 0.159705,
    glare_scale: "glareScale" = 5.44386,
    glare_p1: "glareP1" = 0.99,
    glare_p2: "glareP2" = 4.44,
    // PARTICLES_UI.mnu (firmware 4.93 defaults)
    ui_brownian: "uiBrownian" = 0.6,
    rshake_brw: "rshakeBrw" = 7.4992,
    dpad_rot_max: "dpadRotMax" = 0.000116654,
    dpad_scale_x: "dpadScaleX" = 1.0,
    dpad_scale_y: "dpadScaleY" = 0.0,
    dshake_rot_max: "dshakeRotMax" = 0.000124431,
    dshake_rot_imp: "dshakeRotImp" = 0.513834,
    dshake_brw_imp: "dshakeBrwImp" = 0.061702,
    dshake_thresh: "dshakeThresh" = 0.722145,
    dshake_x_coeff: "dshakeXCoeff" = 0.402735,
    dshake_g_coeff: "dshakeGCoeff" = 0.444397,
    icon_wind: "iconWind" = 11.3877,
    icon_wind_scl_x: "iconWindSclX" = 0.0,
    icon_wind_scl_y: "iconWindSclY" = 1.0,
    // The modelled PPU side (not from the firmware).
    flow_strength: "flowStrength" = 1.0,
    icon_ease_sec: "iconEaseSec" = 0.065,
});
