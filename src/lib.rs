// SPDX-FileCopyrightText: The audio-viz authors
// SPDX-License-Identifier: MPL-2.0

#![expect(rustdoc::invalid_rust_codeblocks)]
#![doc = include_str!("../README.md")]

mod filter;
pub use filter::{ThreeBandFilterFreqConfig, WaveformFilter, WaveformFilterConfig};

mod waveform;
pub use waveform::{FilteredWaveformBin, FilteredWaveformVal, WaveformBin, WaveformVal};
