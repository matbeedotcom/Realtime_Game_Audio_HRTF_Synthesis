# Individualized HRTF Synthesis

A standalone Rust re-implementation of [Carvalho, D. R. (2021) — *Synthesis of individualized HRTFs based on Neural Networks, Principal Component Analysis and anthropometry*](https://github.com/davircarvalho/Individualized_HRTF_Synthesis). No MATLAB required.

Generate personalized Head-Related Transfer Functions (HRTFs) from your body measurements — then use them with [HeSuVi](https://sourceforge.net/projects/hesuvi/) for better positional audio in games, music, and VR.

## Quick Start

### Download

Grab the latest release from the [Releases page](https://github.com/matbeedotcom/Realtime_Game_Audio_HRTF_Synthesis/releases). The release includes:

- `hrtf-synth-gui.exe` — GUI app (recommended)
- `hrtf-synth.exe` — command-line tool
- `hrtf_model.bin` — required model file (place next to the executables)

### GUI (Recommended)

1. Place `hrtf_model.bin` in a `models/` folder next to the executable, or use the Browse button
2. Run `hrtf-synth-gui.exe`
3. Enter your head and ear measurements (defaults are provided as a starting point)
4. Choose an output location and click **Synthesize**

### CLI

```bash
hrtf-synth --model models/hrtf_model.bin \
           --head-width 15.2 \
           --head-depth 19.5 \
           --ear-left 1.8,0.9,1.7,6.4,0.6,1.3 \
           --ear-right 1.8,0.9,1.7,6.4,0.6,1.3 \
           --output my_hrtf.wav \
           --format wav \
           --sample-rate 48000
```

| Option | Description |
|--------|-------------|
| `--model` | Path to `hrtf_model.bin` |
| `--head-width` | Head width in cm (ear-to-ear distance) |
| `--head-depth` | Head depth in cm (front to back) |
| `--ear-left` | Left ear: d1,d2,d3,d5,d7,d8 (comma-separated, cm) |
| `--ear-right` | Right ear: d1,d2,d3,d5,d7,d8 (comma-separated, cm) |
| `--output` | Output file path |
| `--format` | `wav` (HeSuVi) or `sofa` |
| `--sample-rate` | 44100, 48000, or 96000 |
| `--verbose` | Show detailed progress |

## Using Your HRTF with HeSuVi (Game Audio)

Once you have your personalized `.wav` file:

1. **Install [EqualizerAPO](https://sourceforge.net/projects/equalizerapo/)** — during install, select your headphone output device
2. **Verify EqualizerAPO is active** — run the Configurator from the Start menu, ensure your headphone device has a green checkmark, then reboot
3. **Install [HeSuVi](https://sourceforge.net/projects/hesuvi/)**
4. **Copy your `.wav` file** to `C:\Program Files\EqualizerAPO\config\HeSuVi\hrir\`
5. **Open HeSuVi** and select your file from the HRIR dropdown
6. **Set your game's audio output to 7.1 surround** — HeSuVi will intercept the surround channels and apply your personalized HRTF to create binaural output through your headphones

## How to Measure

### Head

| Parameter | Description | Typical Range |
|-----------|-------------|---------------|
| Head width | Distance between left and right ear canal entrances | 13–17 cm |
| Head depth | Distance from front of head to back | 18–22 cm |

### Ears (measure each ear)

| Parameter | ID | Description | Typical Range |
|-----------|-----|-------------|---------------|
| Cavum concha height | d1 | Height of the lower ear bowl | 1.4–2.2 cm |
| Cymba concha height | d2 | Height of the upper ear bowl | 0.5–1.2 cm |
| Cavum concha width | d3 | Width of the lower ear bowl | 1.3–2.1 cm |
| Pinna height | d5 | Total ear height | 5.5–7.5 cm |
| Pinna width | d7 | Distance from ear canal to back of ear | 0.3–0.9 cm |
| Ear canal entrance width | d8 | Width of ear canal opening | 0.9–1.7 cm |

Use a ruler or calipers. If you can't measure precisely, the GUI provides average defaults that work as a reasonable starting point. See the [CIPIC anthropometry documentation](https://www.ece.ucdavis.edu/cipic/spatial-sound/hrtf-data/) for detailed measurement diagrams.

## Building from Source

Requires [Rust](https://rustup.rs/) 1.70+.

```bash
cd hrtf-synth

# CLI only
cargo build --release

# GUI
cargo build --release --features gui
```

Executables will be in `hrtf-synth/target/release/`.

## How It Works

1. **Input**: 14 anthropometric measurements (head width, head depth, 6 ear parameters per ear)
2. **Neural Network**: 1,296 small feedforward networks (one per spatial direction × ear) predict PCA coefficients
3. **PCA Reconstruction**: Coefficients are transformed back to magnitude spectra
4. **Phase Reconstruction**: Minimum-phase impulse responses with personalized ITD (Interaural Time Difference)
5. **Output**: Complete HRTF set (648 directions) as a HeSuVi-compatible 14-channel WAV

The neural networks were trained on the CIPIC, ARI, ITA, and 3D3A HRTF databases using Bayesian regularization.

## MATLAB Implementation

The original MATLAB version includes a GUI and research/training scripts.

**Requirements**: MATLAB R2020a+, Signal Processing Toolbox, Deep Learning Toolbox, [SOFA API](https://github.com/sofacoustics/SOFAtoolbox)

Run `eac_individualized_hrtf.mlapp` from the `Individualized HRTF App/` folder.

## References

- Carvalho, D. R. (2021). *Synthesis of individualized HRTFs based on Neural Networks, Principal Component Analysis and anthropometry*. Bachelor's thesis, Federal University of Santa Maria. [Link](https://drive.google.com/file/d/1JVDNxQreYzg7jfauMwFnK3Sg21aB5ri5/view)
- [CIPIC HRTF Database](https://www.ece.ucdavis.edu/cipic/spatial-sound/hrtf-data/)
- [ARI HRTF Database](https://www.kfs.oeaw.ac.at/hrtf)
- [ITA HRTF Database](https://www.ita-toolbox.org/)
- [3D3A Lab HRTF Database](https://www.princeton.edu/3D3A/)

## License

MIT License — see [LICENSE](LICENSE) for details.

## Acknowledgments

- Original MATLAB implementation by Davi Rocha Carvalho (Federal University of Santa Maria)
- SOFA API developers
- Dataset contributors (CIPIC, ARI, ITA, 3D3A)
