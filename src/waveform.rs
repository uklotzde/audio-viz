// SPDX-FileCopyrightText: The audio-viz authors
// SPDX-License-Identifier: MPL-2.0

use palette::{FromColor, Hsv, Srgb};

#[derive(Debug, Clone, Copy, Eq, PartialEq, PartialOrd, Ord, Default)]
#[repr(transparent)]
pub struct WaveformVal(pub u8);

impl WaveformVal {
    const MAX_VAL: u8 = u8::MAX;

    pub(crate) fn from_f32(val: f32) -> Self {
        debug_assert!(val >= f32::from(u8::MIN));
        let mapped = (val * (f32::from(Self::MAX_VAL) + 1.0)).min(f32::from(Self::MAX_VAL));
        debug_assert!(mapped >= f32::from(u8::MIN));
        debug_assert!(mapped <= f32::from(u8::MAX));
        #[allow(clippy::cast_possible_truncation)]
        #[allow(clippy::cast_sign_loss)]
        Self(mapped as u8)
    }

    #[must_use]
    pub fn to_f32(self) -> f32 {
        f32::from(self.0) / f32::from(Self::MAX_VAL)
    }

    #[must_use]
    pub const fn is_zero(self) -> bool {
        self.0 == 0
    }
}

impl From<WaveformVal> for u8 {
    fn from(value: WaveformVal) -> Self {
        value.0
    }
}

#[derive(Debug, Clone, Copy, Default)]
pub struct WaveformBin {
    /// Clamped, logarithmic ratio in the range `0..=1`
    ///
    /// Calculated from the Root Mean Square (RMS) of all samples.
    pub ratio: WaveformVal,

    /// Clamped, absolute peak value in the range `0..=1`
    pub peak: WaveformVal,
}

#[derive(Debug, Clone, Default)]
pub struct FilteredWaveformBin {
    pub all: WaveformBin,
    pub low: WaveformBin,
    pub mid: WaveformBin,
    pub high: WaveformBin,
}

// TODO: Encapsulate in a dedicated (new)type
pub type FilteredWaveform = Vec<FilteredWaveformBin>;

impl FilteredWaveformBin {
    /// <https://en.wikipedia.org/wiki/Spectral_flatness>
    #[must_use]
    pub fn ratio_flatness(&self) -> f32 {
        let Self { low, mid, high, .. } = self;
        let low = 1.0 + low.ratio.to_f32(); // [1, 256]
        let mid = 1.0 + mid.ratio.to_f32(); // [1, 256]
        let high = 1.0 + high.ratio.to_f32(); // [1, 256]
        let geometric_mean = (low * mid * high).powf(1.0 / 3.0);
        let arithmetic_mean = (low + mid + high) / 3.0;
        geometric_mean / arithmetic_mean
    }

    fn ratio_spectral_rgb(&self) -> Srgb<f32> {
        let Self {
            all,
            low,
            mid,
            high,
        } = self;
        let all = all.ratio.to_f32();
        if all == 0.0 {
            return Srgb::new(0.0, 0.0, 0.0);
        }
        let low = low.ratio.to_f32();
        let mid = mid.ratio.to_f32();
        let high = high.ratio.to_f32();
        let red = low.min(all) / all;
        let green = mid.min(all) / all;
        let blue = high.min(all) / all;
        Srgb::new(red, green, blue)
    }

    #[must_use]
    pub fn rgb_color(&self, flatness_to_saturation: f32) -> (f32, f32, f32) {
        let mut rgb = self.ratio_spectral_rgb();
        debug_assert!(flatness_to_saturation >= 0.0);
        debug_assert!(flatness_to_saturation <= 1.0);
        if flatness_to_saturation > 0.0 {
            let mut hsv = Hsv::from_color(rgb);
            let flatness = self.ratio_flatness();
            hsv.saturation = 1.0 - flatness * flatness_to_saturation;
            rgb = Srgb::from_color(hsv);
        }
        (rgb.red, rgb.green, rgb.blue)
    }

    #[must_use]
    pub fn amplitude(&self) -> f32 {
        let all = self.all.ratio.to_f32();
        (all * std::f32::consts::SQRT_2).min(1.0)
    }
}

#[cfg(test)]
mod tests {
    use super::WaveformVal;

    #[test]
    fn waveform_val_from_f32() {
        assert_eq!(WaveformVal::from_f32(0.0), WaveformVal(0));
        assert_eq!(WaveformVal::from_f32(0.25), WaveformVal(64));
        assert_eq!(WaveformVal::from_f32(0.5), WaveformVal(128));
        assert_eq!(WaveformVal::from_f32(0.75), WaveformVal(192));
        assert_eq!(WaveformVal::from_f32(1.0), WaveformVal(255));
    }

    #[test]
    fn waveform_val_to_from_f32() {
        for val in u8::MIN..=u8::MAX {
            let val = WaveformVal(val);
            assert_eq!(val, WaveformVal::from_f32(val.to_f32()));
        }
    }
}
