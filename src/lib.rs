// SPDX-FileCopyrightText: The audio-viz authors
// SPDX-License-Identifier: MPL-2.0

#![allow(rustdoc::invalid_rust_codeblocks)]
#![doc = include_str!("../README.md")]

mod filter;
pub use filter::WaveformFilter;

mod waveform;
pub use waveform::{FilteredWaveform, FilteredWaveformBin, WaveformBin, WaveformVal};
