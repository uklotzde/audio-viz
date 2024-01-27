// SPDX-FileCopyrightText: The audio-viz authors
// SPDX-License-Identifier: MPL-2.0

#[derive(Debug, Clone, Copy, Eq, PartialEq, PartialOrd, Ord, Default)]
#[repr(transparent)]
pub struct WaveformVal(pub u8);

impl WaveformVal {
    const MIN_VAL: u8 = u8::MIN;
    const MAX_VAL: u8 = u8::MAX;

    pub(crate) fn from_f32(val: f32) -> Self {
        debug_assert!(val >= f32::from(Self::MIN_VAL));
        let mapped = (val * (f32::from(Self::MAX_VAL) + 1.0)).min(f32::from(Self::MAX_VAL));
        debug_assert!(mapped >= f32::from(Self::MIN_VAL));
        debug_assert!(mapped <= f32::from(Self::MAX_VAL));
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
    /// RGB color with full brightness
    #[must_use]
    pub fn spectral_rgb_color(self) -> (f32, f32, f32) {
        self.spectral_rgb_color_max(1.0)
    }

    /// RGB color with brightness limited by [`Self::all`]
    #[must_use]
    pub fn spectral_rgb_color_all(self) -> (f32, f32, f32) {
        self.spectral_rgb_color_max(self.all.to_f32())
    }

    /// RGB color with custom brightness limited by `max`
    #[must_use]
    pub fn spectral_rgb_color_max(self, max: f32) -> (f32, f32, f32) {
        let Self {
            all: _,
            low,
            mid,
            high,
        } = self;
        let low = low.to_f32();
        let mid = mid.to_f32();
        let high = high.to_f32();
        // The `max` value is used to control the brightness of the resulting color.
        // Otherwise we would only reach the edges of the RGB space with one component
        // always maxed out.
        let denom = max.max(low).max(mid).max(high);
        if denom == 0.0 {
            return (0.0, 0.0, 0.0);
        }
        let red = low / denom;
        let green = mid / denom;
        let blue = high / denom;
        (red, green, blue)
    }
}

#[derive(Debug, Clone, Copy, Default)]
pub struct WaveformBin {
    /// Clamped, absolute peak value in the range `0..=1`
    pub peak: WaveformVal,

    /// Clamped and scaled RMS value in the range `0..=1`.
    pub energy: WaveformVal,
}

#[derive(Debug, Clone, Default)]
pub struct FilteredWaveformBin {
    pub all: WaveformBin,
    pub low: WaveformBin,
    pub mid: WaveformBin,
    pub high: WaveformBin,
}

impl FilteredWaveformBin {
    /// Peak values
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

    /// Scaled RMS values
    #[must_use]
    pub const fn energy(&self) -> FilteredWaveformVal {
        let Self {
            all,
            low,
            mid,
            high,
        } = self;
        FilteredWaveformVal {
            all: all.energy,
            low: low.energy,
            mid: mid.energy,
            high: high.energy,
        }
    }

    /// <https://en.wikipedia.org/wiki/Spectral_flatness>
    #[must_use]
    pub fn spectral_flatness(&self) -> f32 {
        let FilteredWaveformVal {
            all: _,
            low,
            mid,
            high,
        } = self.energy();
        let low = low.to_f32();
        let mid = mid.to_f32();
        let high = high.to_f32();
        let arithmetic_mean = (low + mid + high) / 3.0;
        if arithmetic_mean == 0.0 {
            // Perfectly flat spectrum.
            return 1.0;
        }
        debug_assert!(arithmetic_mean > 0.0);
        debug_assert!(arithmetic_mean <= 1.0);
        let geometric_mean = (low * mid * high).powf(1.0 / 3.0);
        debug_assert!(geometric_mean >= 0.0);
        debug_assert!(geometric_mean <= 1.0);
        geometric_mean / arithmetic_mean
    }
}

#[cfg(test)]
mod tests {
    use super::WaveformVal;

    #[test]
    fn waveform_val_from_f32() {
        assert_eq!(
            WaveformVal::from_f32(0.0),
            WaveformVal(WaveformVal::MIN_VAL)
        );
        assert_eq!(WaveformVal::from_f32(0.25), WaveformVal(64));
        assert_eq!(WaveformVal::from_f32(0.5), WaveformVal(128));
        assert_eq!(WaveformVal::from_f32(0.75), WaveformVal(192));
        assert_eq!(
            WaveformVal::from_f32(1.0),
            WaveformVal(WaveformVal::MAX_VAL)
        );
    }

    #[test]
    fn waveform_val_to_from_f32() {
        for val in WaveformVal::MIN_VAL..=WaveformVal::MAX_VAL {
            let val = WaveformVal(val);
            assert_eq!(val, WaveformVal::from_f32(val.to_f32()));
        }
    }

    #[test]
    fn spectral_flatness_one() {
        for val in WaveformVal::MIN_VAL..=WaveformVal::MAX_VAL {
            let val = WaveformVal(val);
            let bin = super::FilteredWaveformBin {
                all: super::WaveformBin {
                    peak: val,
                    energy: val,
                },
                low: super::WaveformBin {
                    peak: val,
                    energy: val,
                },
                mid: super::WaveformBin {
                    peak: val,
                    energy: val,
                },
                high: super::WaveformBin {
                    peak: val,
                    energy: val,
                },
            };
            let spectral_flatness = bin.spectral_flatness();
            assert!(spectral_flatness > 0.999_999);
        }
    }
}
