//! Nine-band, stereo PCM equalizer. This module only transforms samples handed
//! to it; connecting it to a specific browser's audio stream is a separate task.
//! No microphone, recording, devices, or network access are used here.

use std::f64::consts::PI;

pub const FREQUENCIES_HZ: [f64; 9] = [60.0, 170.0, 310.0, 600.0, 1000.0, 3000.0, 6000.0, 12000.0, 16000.0];

#[derive(Clone, Copy, Debug)]
pub struct EqSettings {
    pub enabled: bool,
    pub preamp_db: f64,
    pub bands_db: [f64; 9],
}

impl Default for EqSettings {
    fn default() -> Self { Self { enabled: true, preamp_db: 0.0, bands_db: [0.0; 9] } }
}

#[derive(Clone, Copy, Debug, Default)]
struct State { z1: f64, z2: f64 }

#[derive(Clone, Copy, Debug)]
struct Biquad {
    b0: f64, b1: f64, b2: f64, a1: f64, a2: f64,
    state: [State; 2],
}

fn bounded(value: f64) -> f64 {
    if value.is_finite() { value.clamp(-12.0, 12.0) } else { 0.0 }
}

impl Biquad {
    fn peaking(sample_rate: f64, freq: f64, gain_db: f64) -> Self {
        // RBJ audio EQ cookbook; Q=1.0. Limit frequency to below Nyquist.
        let w = 2.0 * PI * freq.min(sample_rate * 0.45) / sample_rate;
        let a = 10.0_f64.powf(bounded(gain_db) / 40.0);
        let alpha = w.sin() / 2.0;
        let c = w.cos();
        let a0 = 1.0 + alpha / a;
        Self {
            b0: (1.0 + alpha * a) / a0,
            b1: -2.0 * c / a0,
            b2: (1.0 - alpha * a) / a0,
            a1: -2.0 * c / a0,
            a2: (1.0 - alpha / a) / a0,
            state: [State::default(); 2],
        }
    }

    fn sample(&mut self, channel: usize, x: f64) -> f64 {
        let state = &mut self.state[channel];
        let y = self.b0 * x + state.z1;
        state.z1 = self.b1 * x - self.a1 * y + state.z2;
        state.z2 = self.b2 * x - self.a2 * y;
        y
    }
}

/// Processes interleaved stereo floating-point PCM. Settings updates should be
/// sent from the UI to the audio thread rather than shared under a blocking lock.
pub struct Equalizer {
    sample_rate: f64,
    settings: EqSettings,
    filters: [Biquad; 9],
    preamp: f64,
}

impl Equalizer {
    pub fn new(sample_rate: u32) -> Result<Self, &'static str> {
        if !(8000..=384000).contains(&sample_rate) { return Err("Unsupported sample rate"); }
        let sample_rate = sample_rate as f64;
        Ok(Self { sample_rate, settings: EqSettings::default(),
            filters: std::array::from_fn(|i| Biquad::peaking(sample_rate, FREQUENCIES_HZ[i], 0.0)),
            preamp: 1.0 })
    }

    pub fn set_settings(&mut self, settings: EqSettings) {
        self.settings = EqSettings { enabled: settings.enabled, preamp_db: bounded(settings.preamp_db),
            bands_db: settings.bands_db.map(bounded) };
        self.preamp = 10.0_f64.powf(self.settings.preamp_db / 20.0);
        self.filters = std::array::from_fn(|i| Biquad::peaking(self.sample_rate, FREQUENCIES_HZ[i], self.settings.bands_db[i]));
    }

    pub fn process_interleaved_stereo(&mut self, pcm: &mut [f32]) {
        if !self.settings.enabled { return; }
        for frame in pcm.chunks_exact_mut(2) {
            for (channel, sample) in frame.iter_mut().enumerate() {
                let mut x = *sample as f64 * self.preamp;
                if !x.is_finite() { x = 0.0; }
                for filter in &mut self.filters { x = filter.sample(channel, x); }
                *sample = x.clamp(-1.0, 1.0) as f32;
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sine(frequency: f64, seconds: f64) -> Vec<f32> {
        let n = (48000.0 * seconds) as usize;
        (0..n).flat_map(|i| {
            let x = (2.0 * PI * frequency * i as f64 / 48000.0).sin() as f32 * 0.01;
            [x, x]
        }).collect()
    }
    fn rms(pcm: &[f32]) -> f64 {
        let part = &pcm[pcm.len()/2..];
        (part.iter().map(|&x| (x as f64).powi(2)).sum::<f64>() / part.len() as f64).sqrt()
    }
    #[test]
    fn center_frequency_changes_by_requested_gain() {
        let original = sine(1000.0, 1.0); let mut output = original.clone();
        let mut settings = EqSettings::default(); settings.bands_db[4] = -9.0;
        let mut eq = Equalizer::new(48000).unwrap(); eq.set_settings(settings); eq.process_interleaved_stereo(&mut output);
        let db = 20.0 * (rms(&output)/rms(&original)).log10();
        assert!((db + 9.0).abs() < 0.05, "{db}");
    }
    #[test]
    fn bypass_preserves_samples_and_preamp_affects_level() {
        let original = sine(1000.0, 1.0); let mut output = original.clone();
        let mut eq = Equalizer::new(48000).unwrap();
        eq.set_settings(EqSettings { enabled: false, preamp_db: 12.0, bands_db: [12.0; 9] });
        eq.process_interleaved_stereo(&mut output); assert_eq!(output, original);
        eq.set_settings(EqSettings { preamp_db: -6.0, ..EqSettings::default() });
        eq.process_interleaved_stereo(&mut output);
        let db = 20.0 * (rms(&output)/rms(&original)).log10(); assert!((db + 6.0).abs() < 0.05, "{db}");
    }
    #[test]
    fn stereo_channels_do_not_bleed() {
        let mut pcm = sine(170.0, 1.0); for frame in pcm.chunks_exact_mut(2) { frame[1] = 0.0; }
        let mut eq=Equalizer::new(48000).unwrap(); let mut settings=EqSettings::default(); settings.bands_db[1]=8.0;
        eq.set_settings(settings); eq.process_interleaved_stereo(&mut pcm);
        assert!(pcm.chunks_exact(2).all(|frame| frame[1] == 0.0));
    }
}
