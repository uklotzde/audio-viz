// SPDX-FileCopyrightText: The audio-viz authors
// SPDX-License-Identifier: MPL-2.0

use biquad::{Biquad as _, Coefficients, DirectForm2Transposed, Hertz, Q_BUTTERWORTH_F32};

use super::{FilteredWaveformBin, WaveformBin, WaveformVal};

// Only needed for default initialization.
const DEFAULT_SAMPLE_RATE_HZ: f32 = 44_100.0;

// Only needed for default initialization.
//
// Adopted from [Superpowered](https://docs.superpowered.com/reference/latest/analyzer>)
// which uses a resolution of 150 points/sec resolution.
const DEFAULT_BINS_PER_SEC: f32 = 150.0;

const MIN_SAMPLES_PER_BIN: f32 = 64.0;

/// Crossover low/mid (high pass)
///
/// Same boundary as used by both Rekordbox and
/// [Superpowered](https://docs.superpowered.com/reference/latest/analyzer>)
/// and also used by Rekordbox.
const DEFAULT_LOW_LP_FILTER_HZ: f32 = 200.0;

/// Crossover low/mid (low pass)
///
/// Overlapping mids with lows.
const DEFAULT_LOW_HP_FILTER_HZ: f32 = DEFAULT_LOW_LP_FILTER_HZ / 2.0;

/// Crossover mid/high (low pass)
///
/// Same boundary as used by
/// [Superpowered](https://docs.superpowered.com/reference/latest/analyzer>)
/// whereas Rekordbox uses 2000 Hz.
const DEFAULT_HIGH_LP_FILTER_HZ: f32 = 1600.0;

/// Crossover mid/high (high pass)
///
/// Overlapping highs with mids.
const DEFAULT_HIGH_HP_FILTER_HZ: f32 = DEFAULT_HIGH_LP_FILTER_HZ / 2.0;

#[derive(Debug, Clone, PartialEq)]
pub struct ThreeBandFilterFreqConfig {
    pub low_lp_hz: f32,
    pub low_hp_hz: f32,
    pub high_lp_hz: f32,
    pub high_hp_hz: f32,
}

impl ThreeBandFilterFreqConfig {
    pub const MIN_FREQ_HZ: f32 = 20.0;
    pub const MAX_FREQ_HZ: f32 = 20_000.0;

    pub const DEFAULT: Self = Self {
        low_lp_hz: DEFAULT_LOW_LP_FILTER_HZ,
        low_hp_hz: DEFAULT_LOW_HP_FILTER_HZ,
        high_lp_hz: DEFAULT_HIGH_LP_FILTER_HZ,
        high_hp_hz: DEFAULT_HIGH_HP_FILTER_HZ,
    };
}

impl Default for ThreeBandFilterFreqConfig {
    fn default() -> Self {
        Self::DEFAULT
    }
}

// 3-band crossover using 4th-order Linkwitz-Riley (LR4) LP/HP filters (2 cascaded 2nd-order Butterworth)
// and two 2nd-order Butterworth LP/HP filters for the mid band.
#[derive(Debug)]
struct ThreeBandFilterBank {
    low_lp: [DirectForm2Transposed<f32>; 2],
    mid_bp: [DirectForm2Transposed<f32>; 2],
    high_hp: [DirectForm2Transposed<f32>; 2],
}

impl ThreeBandFilterBank {
    #[allow(clippy::needless_pass_by_value)]
    fn new(fs: Hertz<f32>, config: ThreeBandFilterFreqConfig) -> Self {
        let ThreeBandFilterFreqConfig {
            low_lp_hz,
            low_hp_hz,
            high_lp_hz,
            high_hp_hz,
        } = config;
        debug_assert!(low_hp_hz >= ThreeBandFilterFreqConfig::MIN_FREQ_HZ);
        debug_assert!(low_hp_hz <= low_lp_hz); // Overlapping mids with lows
        debug_assert!(low_lp_hz < high_hp_hz); // Non-empty mids
        debug_assert!(high_hp_hz <= high_lp_hz); // Overlapping mids with highs
        debug_assert!(high_lp_hz <= ThreeBandFilterFreqConfig::MAX_FREQ_HZ);
        let low_lp_f0 = Hertz::<f32>::from_hz(low_lp_hz).expect("valid frequency");
        let low_lp = DirectForm2Transposed::<f32>::new(
            Coefficients::<f32>::from_params(
                biquad::Type::LowPass,
                fs,
                low_lp_f0,
                Q_BUTTERWORTH_F32,
            )
            .expect("valid params"),
        );
        let low_hp_f0 = Hertz::<f32>::from_hz(low_hp_hz).expect("valid frequency");
        let low_hp = DirectForm2Transposed::<f32>::new(
            Coefficients::<f32>::from_params(
                biquad::Type::HighPass,
                fs,
                low_hp_f0,
                Q_BUTTERWORTH_F32,
            )
            .expect("valid params"),
        );
        let high_lp_f0 = Hertz::<f32>::from_hz(high_lp_hz).expect("valid frequency");
        let high_lp = DirectForm2Transposed::<f32>::new(
            Coefficients::<f32>::from_params(
                biquad::Type::LowPass,
                fs,
                high_lp_f0,
                Q_BUTTERWORTH_F32,
            )
            .expect("valid params"),
        );
        let high_hp_f0 = Hertz::<f32>::from_hz(high_hp_hz).expect("valid frequency");
        let high_hp = DirectForm2Transposed::<f32>::new(
            Coefficients::<f32>::from_params(
                biquad::Type::HighPass,
                fs,
                high_hp_f0,
                Q_BUTTERWORTH_F32,
            )
            .expect("valid params"),
        );
        Self {
            low_lp: [low_lp, low_lp],
            mid_bp: [low_hp, high_lp],
            high_hp: [high_hp, high_hp],
        }
    }

    #[allow(clippy::unused_self)] // TODO
    fn shape_input_signal(&mut self, sample: f32) -> f32 {
        // TODO: Apply filtering to shape the input signal according to the
        // ISO 226:2003 equal-loudness-level contour at 40 phons (A-weighting).
        sample
    }

    fn run(&mut self, sample: f32) -> FilteredSample {
        let all = self.shape_input_signal(sample);
        let Self {
            low_lp,
            mid_bp,
            high_hp,
        } = self;
        let low = low_lp
            .iter_mut()
            .fold(all, |sample, filter| filter.run(sample));
        let mid = mid_bp
            .iter_mut()
            .fold(all, |sample, filter| filter.run(sample));
        let high = high_hp
            .iter_mut()
            .fold(all, |sample, filter| filter.run(sample));
        FilteredSample {
            all,
            low,
            mid,
            high,
        }
    }
}

#[derive(Debug, Default)]
struct WaveformBinAccumulator {
    rms_sum: f64,
    peak: f32,
}

#[derive(Debug)]
struct FilteredSample {
    all: f32,
    low: f32,
    mid: f32,
    high: f32,
}

impl WaveformBinAccumulator {
    fn add_sample(&mut self, sample: f32) {
        let sample_f64 = f64::from(sample);
        self.rms_sum += sample_f64 * sample_f64;
        self.peak = self.peak.max(sample.abs());
    }

    fn finish(self, rms_div: f64) -> WaveformBin {
        debug_assert!(rms_div > 0.0);
        let Self { rms_sum, peak } = self;
        #[allow(clippy::cast_possible_truncation)]
        let ratio = (1.0 + (rms_sum / rms_div).sqrt()).log2() as f32;
        WaveformBin {
            ratio: WaveformVal::from_f32(ratio),
            peak: WaveformVal::from_f32(peak),
        }
    }
}

#[derive(Debug, Default)]
struct FilteredWaveformBinAccumulator {
    sample_count: u32,
    all: WaveformBinAccumulator,
    low: WaveformBinAccumulator,
    mid: WaveformBinAccumulator,
    high: WaveformBinAccumulator,
}

impl FilteredWaveformBinAccumulator {
    fn add_sample(&mut self, filter_bank: &mut ThreeBandFilterBank, sample: f32) {
        self.sample_count += 1;
        let FilteredSample {
            all,
            low,
            mid,
            high,
        } = filter_bank.run(sample);
        self.all.add_sample(all);
        self.low.add_sample(low);
        self.mid.add_sample(mid);
        self.high.add_sample(high);
    }

    fn finish(self) -> Option<FilteredWaveformBin> {
        let Self {
            sample_count,
            all,
            low,
            mid,
            high,
        } = self;
        if sample_count == 0 {
            return None;
        }
        let rms_div = f64::from(sample_count);
        let all = all.finish(rms_div);
        let low = low.finish(rms_div);
        let mid = mid.finish(rms_div);
        let high = high.finish(rms_div);
        Some(FilteredWaveformBin {
            all,
            low,
            mid,
            high,
        })
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct WaveformFilterConfig {
    pub sample_rate_hz: f32,
    pub bins_per_sec: f32,
    pub filter_freqs: ThreeBandFilterFreqConfig,
}

impl WaveformFilterConfig {
    pub const DEFAULT: Self = Self {
        sample_rate_hz: DEFAULT_SAMPLE_RATE_HZ,
        bins_per_sec: DEFAULT_BINS_PER_SEC,
        filter_freqs: ThreeBandFilterFreqConfig::DEFAULT,
    };
}

impl Default for WaveformFilterConfig {
    fn default() -> Self {
        Self::DEFAULT
    }
}

#[derive(Debug)]
pub struct WaveformFilter {
    pending_samples_count: f32,
    samples_per_bin: f32,
    filter_bank: ThreeBandFilterBank,
    filtered_accumulator: FilteredWaveformBinAccumulator,
}

impl Default for WaveformFilter {
    fn default() -> Self {
        Self::new(Default::default())
    }
}

impl WaveformFilter {
    #[must_use]
    #[allow(clippy::missing_panics_doc)]
    pub fn new(config: WaveformFilterConfig) -> Self {
        let WaveformFilterConfig {
            sample_rate_hz,
            bins_per_sec,
            filter_freqs,
        } = config;
        let sample_rate = Hertz::<f32>::from_hz(sample_rate_hz).expect("valid sample rate");
        let samples_per_bin = (sample_rate_hz / bins_per_sec).max(MIN_SAMPLES_PER_BIN);
        Self {
            pending_samples_count: 0.0,
            samples_per_bin,
            filter_bank: ThreeBandFilterBank::new(sample_rate, filter_freqs),
            filtered_accumulator: Default::default(),
        }
    }

    fn finish_bin(&mut self) -> Option<FilteredWaveformBin> {
        std::mem::take(&mut self.filtered_accumulator).finish()
    }

    pub fn add_sample(&mut self, sample: f32) -> Option<FilteredWaveformBin> {
        let next_bin = if self.pending_samples_count >= self.samples_per_bin {
            self.pending_samples_count -= self.samples_per_bin;
            self.finish_bin()
        } else {
            None
        };
        self.filtered_accumulator
            .add_sample(&mut self.filter_bank, sample);
        self.pending_samples_count += 1.0;
        next_bin
    }

    #[must_use]
    pub fn finish(mut self) -> Option<FilteredWaveformBin> {
        self.finish_bin()
    }
}
