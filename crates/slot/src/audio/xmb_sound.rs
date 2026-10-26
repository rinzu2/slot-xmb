//! The bar's sound effects, read from the card at start: `System/sounds/<name>.wav`, 16-bit
//! PCM at any rate, mono or stereo. A missing or unreadable file is simply silent, so the bar
//! works the same with no sounds installed.

use std::path::Path;

#[derive(Copy, Clone, PartialEq, Eq, Debug)]
pub enum XmbSound {
    /// Played with the start-up animation.
    Opening,
    /// Backing out of something.
    Cancel,
    /// Moving the selection.
    Cursor,
    /// Opening or choosing something.
    Confirm,
    /// A game left running coming back as the handheld starts.
    Start,
}

impl XmbSound {
    pub const ALL: [XmbSound; 5] = [
        XmbSound::Opening,
        XmbSound::Cancel,
        XmbSound::Cursor,
        XmbSound::Confirm,
        XmbSound::Start,
    ];

    pub fn file_name(self) -> &'static str {
        match self {
            XmbSound::Opening => "opening.wav",
            XmbSound::Cancel => "cancel.wav",
            XmbSound::Cursor => "cursor.wav",
            XmbSound::Confirm => "confirm.wav",
            XmbSound::Start => "start.wav",
        }
    }

    fn index(self) -> usize {
        self as usize
    }
}

/// A decoded sound: interleaved stereo frames at `rate`.
#[derive(Clone, Debug, Default)]
pub struct Clip {
    pub rate: u32,
    pub frames: Vec<[f32; 2]>,
}

#[derive(Clone, Debug, Default)]
pub struct XmbSounds {
    clips: [Option<Clip>; 5],
}

impl XmbSounds {
    pub fn load(root: &Path) -> Self {
        let dir = root.join("System").join("sounds");
        let mut sounds = XmbSounds::default();
        for s in XmbSound::ALL {
            let path = dir.join(s.file_name());
            sounds.clips[s.index()] = std::fs::read(&path).ok().and_then(|b| parse_wav(&b));
        }
        sounds
    }

    pub fn has(&self, s: XmbSound) -> bool {
        self.clips[s.index()].is_some()
    }

    /// The sound as interleaved stereo samples at `sample_rate`, ready to mix.
    pub fn render(&self, s: XmbSound, sample_rate: u32) -> Vec<i16> {
        let Some(clip) = &self.clips[s.index()] else {
            return Vec::new();
        };
        render(clip, sample_rate)
    }
}

pub fn render(clip: &Clip, sample_rate: u32) -> Vec<i16> {
    if clip.frames.is_empty() || clip.rate == 0 || sample_rate == 0 {
        return Vec::new();
    }
    let ratio = clip.rate as f64 / sample_rate as f64;
    let n = (clip.frames.len() as f64 / ratio) as usize;
    let mut out = Vec::with_capacity(n * 2);
    let last = clip.frames.len() - 1;
    for i in 0..n {
        let x = i as f64 * ratio;
        let a = (x as usize).min(last);
        let b = (a + 1).min(last);
        let f = (x - a as f64) as f32;
        for c in 0..2 {
            let v = clip.frames[a][c] * (1.0 - f) + clip.frames[b][c] * f;
            out.push((v * 32767.0).clamp(-32768.0, 32767.0) as i16);
        }
    }
    out
}

/// Reads a RIFF WAVE file of 16-bit PCM.
pub fn parse_wav(bytes: &[u8]) -> Option<Clip> {
    if bytes.len() < 12 || &bytes[0..4] != b"RIFF" || &bytes[8..12] != b"WAVE" {
        return None;
    }
    let u16_at = |i: usize| u16::from_le_bytes([bytes[i], bytes[i + 1]]);
    let u32_at =
        |i: usize| u32::from_le_bytes([bytes[i], bytes[i + 1], bytes[i + 2], bytes[i + 3]]);
    let mut at = 12;
    let (mut channels, mut rate, mut bits, mut format) = (0u16, 0u32, 0u16, 0u16);
    let mut data: Option<&[u8]> = None;
    while at + 8 <= bytes.len() {
        let id = &bytes[at..at + 4];
        let len = u32_at(at + 4) as usize;
        let body = at + 8;
        let end = body.saturating_add(len).min(bytes.len());
        match id {
            b"fmt " if len >= 16 && body + 16 <= bytes.len() => {
                format = u16_at(body);
                channels = u16_at(body + 2);
                rate = u32_at(body + 4);
                bits = u16_at(body + 14);
            }
            b"data" => data = Some(&bytes[body..end]),
            _ => {}
        }
        at = body + len + (len & 1);
    }
    let data = data?;
    // 1 is PCM; 0xfffe (extensible) carries PCM too when the bits say 16.
    if !(format == 1 || format == 0xfffe) || bits != 16 || channels == 0 || rate == 0 {
        return None;
    }
    let ch = channels as usize;
    let frames = data
        .chunks_exact(2 * ch)
        .map(|f| {
            let s = |c: usize| i16::from_le_bytes([f[2 * c], f[2 * c + 1]]) as f32 / 32768.0;
            if ch == 1 {
                [s(0), s(0)]
            } else {
                [s(0), s(1)]
            }
        })
        .collect();
    Some(Clip { rate, frames })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn wav(rate: u32, channels: u16, samples: &[i16]) -> Vec<u8> {
        let data: Vec<u8> = samples.iter().flat_map(|s| s.to_le_bytes()).collect();
        let mut b = Vec::new();
        b.extend_from_slice(b"RIFF");
        b.extend_from_slice(&(36 + data.len() as u32).to_le_bytes());
        b.extend_from_slice(b"WAVEfmt ");
        b.extend_from_slice(&16u32.to_le_bytes());
        b.extend_from_slice(&1u16.to_le_bytes());
        b.extend_from_slice(&channels.to_le_bytes());
        b.extend_from_slice(&rate.to_le_bytes());
        b.extend_from_slice(&(rate * 2 * channels as u32).to_le_bytes());
        b.extend_from_slice(&(2 * channels).to_le_bytes());
        b.extend_from_slice(&16u16.to_le_bytes());
        b.extend_from_slice(b"data");
        b.extend_from_slice(&(data.len() as u32).to_le_bytes());
        b.extend_from_slice(&data);
        b
    }

    #[test]
    fn reads_and_resamples_a_wave() {
        let clip = parse_wav(&wav(24_000, 1, &[0, 16384, 0, -16384])).expect("a clip");
        assert_eq!(clip.rate, 24_000);
        assert_eq!(clip.frames.len(), 4);
        let out = render(&clip, 48_000);
        assert_eq!(out.len(), 16, "twice the frames, two channels each");
        assert!(out[2] > 8000 && out[3] == out[2]);
        assert!(parse_wav(b"not a wave").is_none());
    }
}
