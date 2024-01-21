<!-- SPDX-FileCopyrightText: The audio-viz authors -->
<!-- SPDX-License-Identifier: MPL-2.0 -->

# audio-viz

[![Crates.io](https://img.shields.io/crates/v/audio-viz.svg)](https://crates.io/crates/audio-viz)
[![Docs.rs](https://docs.rs/audio-viz/badge.svg)](https://docs.rs/audio-viz)
[![Deps.rs](https://deps.rs/repo/github/uklotzde/audio-viz/status.svg)](https://deps.rs/repo/github/uklotzde/audio-viz)
[![Dependency audit](https://github.com/uklotzde/audio-viz/actions/workflows/dependency-audit.yaml/badge.svg)](https://github.com/uklotzde/audio-viz/actions/workflows/dependency-audit.yaml)
[![Continuous integration](https://github.com/uklotzde/audio-viz/actions/workflows/test.yaml/badge.svg)](https://github.com/uklotzde/audio-viz/actions/workflows/test.yaml)
[![License: MPL 2.0](https://img.shields.io/badge/License-MPL_2.0-brightgreen.svg)](https://opensource.org/licenses/MPL-2.0)

Tooling for colorful audio waveform visualization.

- Analyzes streams of audio samples, divided into windows.
- The results of filtered peak and energy values are stored in one _bin_ per window.
- Each _bin_ only consumes 64 bits for calculating
  - Amplitude
  - Spectral flatness and RGB color

## License

Licensed under the Mozilla Public License 2.0 (MPL-2.0) (see [MPL-2.0.txt](LICENSES/MPL-2.0.txt) or
<https://www.mozilla.org/MPL/2.0/>).

Permissions of this copyleft license are conditioned on making available source code of licensed
files and modifications of those files under the same license (or in certain cases, one of the GNU
licenses). Copyright and license notices must be preserved. Contributors provide an express grant of
patent rights. However, a larger work using the licensed work may be distributed under different
terms and without source code for files added in the larger work.

### Contribution

Any contribution intentionally submitted for inclusion in the work by you shall be licensed under
the Mozilla Public License 2.0 (MPL-2.0).

It is required to add the following header with the corresponding
[SPDX short identifier](https://spdx.dev/ids/) to the top of each file:

```rust
// SPDX-License-Identifier: MPL-2.0
```
