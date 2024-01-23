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
pub struct FilteredWaveformVal {
    pub all: WaveformVal,
    pub low: WaveformVal,
    pub mid: WaveformVal,
    pub high: WaveformVal,
}

impl FilteredWaveformVal {
    /// <https://en.wikipedia.org/wiki/Spectral_flatness>
    #[must_use]
    pub fn spectral_flatness(self) -> f32 {
        let Self {
            all: _,
            low,
            mid,
            high,
        } = self;
        let low = 1.0 + low.to_f32(); // [1, 256]
        let mid = 1.0 + mid.to_f32(); // [1, 256]
        let high = 1.0 + high.to_f32(); // [1, 256]
        let geometric_mean = (low * mid * high).powf(1.0 / 3.0);
        debug_assert!(geometric_mean >= 1.0);
        debug_assert!(geometric_mean <= 256.0);
        let arithmetic_mean = (low + mid + high) / 3.0;
        debug_assert!(arithmetic_mean >= 1.0);
        debug_assert!(arithmetic_mean <= 256.0);
        debug_assert!(arithmetic_mean > 0.0);
        geometric_mean / arithmetic_mean
    }

    fn spectral_rgb(self) -> Srgb<f32> {
        let Self {
            all,
            low,
            mid,
            high,
        } = self;
        let all = all.to_f32();
        let low = low.to_f32();
        let mid = mid.to_f32();
        let high = high.to_f32();
        // The `all` value is needed to control the brightness of the resulting color.
        // Otherwise we would only reach the edges of the RGB space with one component
        // always maxed out.
        let denom = all.max(low).max(mid).max(high);
        if denom == 0.0 {
            return Srgb::new(0.0, 0.0, 0.0);
        }
        let red = low / denom;
        let green = mid / denom;
        let blue = high / denom;
        Srgb::new(red, green, blue)
    }

    /// RGB color
    #[must_use]
    pub fn spectral_rgb_color(self) -> (f32, f32, f32) {
        let rgb = self.spectral_rgb();
        (rgb.red, rgb.green, rgb.blue)
    }

    /// RGB color, de-saturated
    #[must_use]
    pub fn spectral_rgb_color_desaturate(self, desaturate: f32) -> (f32, f32, f32) {
        debug_assert!(desaturate >= 0.0);
        debug_assert!(desaturate <= 1.0);
        let mut rgb = self.spectral_rgb();
        if desaturate < 1.0 {
            let mut hsv = Hsv::from_color(rgb);
            hsv.saturation *= desaturate;
            rgb = Srgb::from_color(hsv);
        }
        (rgb.red, rgb.green, rgb.blue)
    }

    /// RGB color, de-saturated by spectral flatness
    ///
    /// The spectral flatness affects the saturation of the color with
    /// the given ratio.
    #[must_use]
    pub fn spectral_rgb_color_desaturate_flatness(self, flatness_ratio: f32) -> (f32, f32, f32) {
        debug_assert!(flatness_ratio >= 0.0);
        debug_assert!(flatness_ratio <= 1.0);
        if flatness_ratio > 0.0 {
            let flatness = self.spectral_flatness();
            let desaturate = 1.0 - flatness * flatness_ratio;
            self.spectral_rgb_color_desaturate(desaturate)
        } else {
            // Fully saturated.
            self.spectral_rgb_color()
        }
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

impl FilteredWaveformBin {
    #[must_use]
    pub const fn ratio(&self) -> FilteredWaveformVal {
        let Self {
            all,
            low,
            mid,
            high,
        } = self;
        FilteredWaveformVal {
            all: all.ratio,
            low: low.ratio,
            mid: mid.ratio,
            high: high.ratio,
        }
    }

    #[must_use]
    pub const fn peak(&self) -> FilteredWaveformVal {
        let Self {
            all,
            low,
            mid,
            high,
        } = self;
        FilteredWaveformVal {
            all: all.peak,
            low: low.peak,
            mid: mid.peak,
            high: high.peak,
        }
    }

    #[must_use]
    pub fn ratio_amplitude(&self) -> f32 {
        let all = self.all.ratio.to_f32();
        (all * std::f32::consts::SQRT_2).min(1.0)
    }

    #[must_use]
    pub fn peak_amplitude(&self) -> f32 {
        self.all.peak.to_f32()
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
