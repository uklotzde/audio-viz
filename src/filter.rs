// SPDX-FileCopyrightText: The audio-viz authors
// SPDX-License-Identifier: MPL-2.0

use biquad::{Biquad as _, Coefficients, DirectForm2Transposed, Hertz, Q_BUTTERWORTH_F32};

use super::{FilteredWaveformBin, WaveformBin, WaveformVal};

// Only needed for initialization of the filter bank
const DEFAULT_SAMPLE_RATE_HZ: f32 = 44_100.0;

/// Crossover low/mid (high pass)
///
/// Same boundary as used by both Rekordbox and
/// [Superpowered](https://docs.superpowered.com/reference/latest/analyzer>)
/// and also used by Rekordbox.
const LOW_LP_FILTER_HZ: f32 = 200.0;

/// Crossover low/mid (low pass)
///
/// Overlapping mids with lows.
const LOW_HP_FILTER_HZ: f32 = LOW_LP_FILTER_HZ / 2.0;

/// Crossover mid/high (low pass)
///
/// Same boundary as used by
/// [Superpowered](https://docs.superpowered.com/reference/latest/analyzer>)
/// whereas Rekordbox uses 2000 Hz.
const HIGH_LP_FILTER_HZ: f32 = 1600.0;

/// Crossover mid/high (high pass)
///
/// Overlapping highs with mids.
const HIGH_HP_FILTER_HZ: f32 = HIGH_LP_FILTER_HZ / 2.0; // Overlap with mid

const MIN_SAMPLES_PER_BIN: u32 = 64;

// 3-band crossover using 4th-order Linkwitz-Riley (LR4) filters (2 cascaded 2nd-order Butterworth)
struct FilterBank {
    low_lp_lr4: [DirectForm2Transposed<f32>; 2],
    low_hp_lr4: [DirectForm2Transposed<f32>; 2],
    high_lp_lr4: [DirectForm2Transposed<f32>; 2],
    high_hp_lr4: [DirectForm2Transposed<f32>; 2],
}

impl Default for FilterBank {
    fn default() -> Self {
        Self::new(Hertz::<f32>::from_hz(DEFAULT_SAMPLE_RATE_HZ).expect("valid sample rate"))
    }
}

#[derive(Debug)]
struct FilteredSample {
    all: f32,
    low: f32,
    mid: f32,
    high: f32,
}

impl FilterBank {
    fn new(fs: Hertz<f32>) -> Self {
        let low_lp_f0 = Hertz::<f32>::from_hz(LOW_LP_FILTER_HZ).expect("valid frequency");
        let low_lp = DirectForm2Transposed::<f32>::new(
            Coefficients::<f32>::from_params(
                biquad::Type::LowPass,
                fs,
                low_lp_f0,
                Q_BUTTERWORTH_F32,
            )
            .expect("valid params"),
        );
        let low_hp_f0 = Hertz::<f32>::from_hz(LOW_HP_FILTER_HZ).expect("valid frequency");
        let low_hp = DirectForm2Transposed::<f32>::new(
            Coefficients::<f32>::from_params(
                biquad::Type::HighPass,
                fs,
                low_hp_f0,
                Q_BUTTERWORTH_F32,
            )
            .expect("valid params"),
        );
        let high_lp_f0 = Hertz::<f32>::from_hz(HIGH_LP_FILTER_HZ).expect("valid frequency");
        let high_lp = DirectForm2Transposed::<f32>::new(
            Coefficients::<f32>::from_params(
                biquad::Type::LowPass,
                fs,
                high_lp_f0,
                Q_BUTTERWORTH_F32,
            )
            .expect("valid params"),
        );
        let high_hp_f0 = Hertz::<f32>::from_hz(HIGH_HP_FILTER_HZ).expect("valid frequency");
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
            low_lp_lr4: [low_lp, low_lp],
            low_hp_lr4: [low_hp, low_hp],
            high_lp_lr4: [high_lp, high_lp],
            high_hp_lr4: [high_hp, high_hp],
        }
    }

    fn shape_input_signal(&mut self, sample: f32) -> f32 {
        // TODO: Apply filtering to shape the input signal according to the
        // ISO 226:2003 equal-loudness-level contour at 40 phons (A-weighting).
        sample
    }

    pub fn run(&mut self, sample: f32) -> FilteredSample {
        let all = self.shape_input_signal(sample);
        let Self {
            low_lp_lr4,
            low_hp_lr4,
            high_lp_lr4,
            high_hp_lr4,
        } = self;
        let low = low_lp_lr4
            .iter_mut()
            .fold(all, |sample, filter| filter.run(sample));
        let mid_high = low_hp_lr4
            .iter_mut()
            .fold(all, |sample, filter| filter.run(sample));
        let mid = high_lp_lr4
            .iter_mut()
            .fold(mid_high, |sample, filter| filter.run(sample));
        let high = high_hp_lr4
            .iter_mut()
            .fold(mid_high, |sample, filter| filter.run(sample));
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
    pub rms_sum: f64,
    pub peak: f32,
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
    fn add_sample(&mut self, filter_bank: &mut FilterBank, sample: f32) {
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
        let rms_div = sample_count as f64;
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

pub struct WaveformFilter {
    samples_per_bin: u32,
    filter_bank: FilterBank,
    filtered_accumulator: FilteredWaveformBinAccumulator,
}

impl Default for WaveformFilter {
    fn default() -> Self {
        Self::new(
            Hertz::<f32>::from_hz(DEFAULT_SAMPLE_RATE_HZ).expect("valid default sample rate"),
            0,
        )
    }
}

impl WaveformFilter {
    fn new(sample_rate: Hertz<f32>, samples_per_bin: u32) -> Self {
        Self {
            samples_per_bin,
            filter_bank: FilterBank::new(sample_rate),
            filtered_accumulator: Default::default(),
        }
    }

    fn finish_bin(&mut self) -> Option<FilteredWaveformBin> {
        std::mem::take(&mut self.filtered_accumulator).finish()
    }

    fn add_sample(&mut self, sample: f32) -> Option<FilteredWaveformBin> {
        let next_bin = if self.filtered_accumulator.sample_count >= self.samples_per_bin {
            self.finish_bin()
        } else {
            None
        };
        self.filtered_accumulator
            .add_sample(&mut self.filter_bank, sample);
        next_bin
    }

    fn finish(mut self) -> Option<FilteredWaveformBin> {
        self.finish_bin()
    }
}

pub type WaveformFiltered = [FilteredWaveformBin];

pub struct WaveformAnalyzer<'a> {
    file_path: &'a Path,
    file_type: Option<&'a Mime>,
    bins_per_sec: NonZeroU8,
    filter: WaveformFilter,
    waveform: FilteredWaveform,
}

fn samples_per_bin(bins_per_sec: NonZeroU8, sample_rate: Hertz<f32>) -> u32 {
    (sample_rate.hz() / f32::from(bins_per_sec.get())).floor() as u32
}
