# Changelog

All notable changes to **CStrip2 Visual** are documented in this file.

## [v0.1.0] — 2026-10-13

### First Public Release
* **DSP Engine (Airwindows CStrip2 Port):**
  * **Capacitor:** 6-stage interleaved non-linear IIR High-Pass (`Hi-Pass`) and Low-Pass (`Lo-Pass`) input filters.
  * **ButterComp:** Non-linear Class AB dynamic compressor (`Compres`, `Speed`) with bipolar sample alternation, independent positive/negative tracking, golden-ratio moving average, and dual-speed smoothing.
  * **3-Band Complementary Split EQ:** Adjustable `Bass` (`Bass Frq`), `Mid`, and `Treble` (`Treb Frq`) bands with logarithmic density-to-dB mapping and cubic harmonic correction.
  * **Output Stage:** Output gain control with unity gain at approximately `0.333` (one third of the control range, with `0.33` as the control's default value), progressive **Spiral** and **Density** harmonic saturation stages, and **Airwindows Floating Point Dither**.
* **Graphical User Interface & Real-Time Analyzer (`egui` / `NIH-plug`):**
  * Lock-free dual-channel stereo FFT spectrum analyzer (`OUT`, `IN`, `BOTH`) with zero allocations on the real-time audio thread.
  * Real-time **Tone Curve** overlay displaying the combined Capacitor + 3-Band EQ frequency response (`20 Hz – 20 kHz`) with draggable band nodes.
  * Selectable FFT resolution (`31`, `63`, `128`, `CONT`), line mode (`CRV`, `FILL`, `BAR`), decay speed (`SLOW`, `MED`, `FAST`), and peak `HOLD`.
  * Stereo `IN`/`OUT` peak and RMS level meters and dedicated `GR` (ButterComp gain reduction) meter.
  * Built-in factory presets, custom preset saving (`SAVE`), and `A/B` state comparison slots (`COPY A->B`, `COPY B->A`).
* **Platform & Format Support:**
  * **Windows (`x86_64`):** `cstrip2.vst3` (VST3) and `cstrip2.clap` (CLAP).
  * **macOS (`Universal 2`: `arm64` Apple Silicon & `x86_64` Intel):** `CStrip2.vst3` (VST3), `CStrip2.clap` (CLAP), and autonomous self-contained `CStrip2.component` (AUv2).
* **Documentation & Licensing:**
  * Complete English and Spanish User Manuals, Quick Start Guides, and Spanish macOS Installation Guide in `docs/`.
  * Full `GPL-3.0-or-later` licensing package, third-party notices, and offline vendored Corresponding Source archive (`CStrip2_Visual_Source_v0.1.0.zip`, archiving the 276 external dependencies in `vendor/` alongside the 2 workspace crates `cstrip2` and `xtask` for the 278 total packages in `Cargo.lock`).
