# Third-Party Software & Dependency Notices

**Product:** CStrip2 Visual  
**Version:** 0.1.0  
**Target Architectures:** Windows (x86_64), macOS (Universal 2: Apple Silicon arm64 & Intel x86_64)  
**Distribution Formats:** VST3, CLAP, AUv2 (Audio Unit v2)  
**Document Status:** Final Candidate for Legal Review (Revised Edition)  
**Date of Audit:** 12 October 2026  

---

## 1. Marco Legal General y Arquitectura de Binarios

**CStrip2 Visual** está licenciado bajo la **GNU General Public License v3.0 or later (GPL-3.0-or-later)** de acuerdo con lo declarado en `crates/cstrip2/Cargo.toml`.

### 1.1. Formatos Distribuidos y Alcance Copyleft
1. **VST3 (`cstrip2.vst3` / `CStrip2.vst3`):**  
   Enlaza estáticamente la biblioteca de bindings `vst3-sys` (licenciada bajo GNU GPLv3) y el crate principal `cstrip2` (GPL-3.0-or-later).
2. **CLAP (`cstrip2.clap` / `CStrip2.clap`):**  
   En las compilaciones de Windows y macOS, NIH-plug genera una única biblioteca dinámica (`cdylib`). Los puntos de entrada CLAP (`nih_export_clap!`) y VST3 (`nih_export_vst3!`) se compilan y exportan desde el **mismo archivo ejecutable binario**. Por consiguiente, el binario distribuido como CLAP incorpora físicamente código objeto de `vst3-sys` (GPLv3) y de `cstrip2` (GPL-3.0-or-later), quedando sujeto a las obligaciones copyleft de la GNU GPLv3.
3. **AUv2 (`CStrip2.component` en macOS):**  
   Construido mediante `clap-wrapper` (MIT) y Apple `AudioUnitSDK` (Apache-2.0). El bundle resultante aloja físicamente el bundle completo `CStrip2.clap` dentro de `Contents/PlugIns/` (*Thick Bundle pattern*). Al incorporar y ejecutar en el mismo proceso dicho ejecutable bajo GPLv3, el bundle AUv2 distribuido está igualmente sujeto a la disponibilidad del Código Fuente Correspondiente bajo la GNU GPLv3.

**Los destinatarios de cualquiera de los binarios distribuidos tienen derecho a recibir el Código Fuente Correspondiente completo bajo la GNU GPLv3.**

---

## 2. Auditoría del Workspace y Clasificación de Dependencias

Se ha realizado una auditoría determinista del workspace validado (`phase2d_work/workspace`) utilizando un parser TOML sobre `Cargo.lock` y ejecutando `cargo metadata --format-version 1 --locked`.

### 2.1. Conciliación Rigurosa de Cifras
* **278 Paquetes en `Cargo.lock`:**  
  El archivo `Cargo.lock` contiene exactamente **278 bloques `[[package]]`**.
* **278 Paquetes en `cargo metadata --locked`:**  
  El array `packages` y el array `resolve.nodes` de `cargo metadata` contienen exactamente **278 entradas**, coincidiendo 1 a 1 con el lockfile. La ejecución con `--locked` se verificó mediante suma SHA-256 (`b1c7746a099e48dffbdcebf20009e97db9a4016e0479a44d3ba7888a6613285e`), confirmando que el lockfile **no es modificado**.
* **Desglose Workspace (264 + 14 = 278):**
  - **264 paquetes:** Pertenecen al grafo de dependencias del plugin de audio **`cstrip2`** (`crates/cstrip2`).
  - **14 paquetes:** Pertenecen exclusivamente a la herramienta de empaquetado del workspace **`xtask`** (`crates/xtask`) y a sus dependencias directas (`nih_plug_xtask`, `cargo_metadata`, `camino`, `cargo-platform`, `goblin`, `scroll`, `scroll_derive`, `toml`, `toml_edit 0.19`, `toml_datetime 0.6`, `serde_spanned 0.6`, `reflink`, `winnow 0.5`).

### 2.2. Categorías de Dependencias del Plugin `cstrip2` (264 Paquetes)
No todos los 264 paquetes resueltos se incorporan de forma idéntica en los binarios finales:
1. **Herramientas de Compilación y Macros Procedimentales (no enlazadas en runtime):**  
   Crates como `syn`, `quote`, `proc-macro2`, `autocfg`, `version_check`, `bytemuck_derive`, `thiserror-impl`, `serde_derive`, etc., se ejecutan exclusivamente en la máquina host durante la compilación para generar código Rust y **no forman parte del código máquina del ejecutable final**.
2. **Dependencias Específicas de Plataforma:**  
   - En **Windows**: se compilan componentes específicos como `windows-sys`, `winapi` y `windows-targets`.
   - En **macOS**: se compilan componentes específicos como `objc-sys`, `block2` y `core-graphics`.
   - Componentes Linux/X11 (`x11rb`, etc.) y Web/Android presentes en el grafo no son compilados ni enlazados en los binarios distribuidos para Windows y macOS.
3. **Componentes en Tiempo de Ejecución Efectivamente Enlazados:**  
   Código DSP de audio, framework de plugin NIH-plug, motor de interfaz gráfica (`egui`, `glow`, `baseview`), primitivas atómicas (`atomic_float`), colas multihilo (`crossbeam-channel`), serialización (`serde`) y capas ABI (`vst3-sys`, `clap-sys`).

---

## 3. Componentes Principales y Avisos de Copyright Específicos

### 3.1. Algoritmos DSP de Airwindows (CStrip2)
* **Autor:** Chris Johnson ([Airwindows](https://www.airwindows.com))
* **Repositorio:** [https://github.com/airwindows/airwindows](https://github.com/airwindows/airwindows)
* **Licencia:** MIT License
* **Aviso de Copyright Oficial:** `Copyright (c) 2018 Chris Johnson`
* **Descripción:** Implementación de referencia y fórmulas matemáticas de CStrip2 en C++.

### 3.2. Estándar CLAP (CLever Audio Plugin) y Wrapper
* **Cabeceras C de CLAP (v1.2.3):**
  - **Repositorio:** [https://github.com/free-audio/clap](https://github.com/free-audio/clap)
  - **Licencia:** MIT License
  - **Aviso de Copyright Oficial:** `Copyright (c) 2021 Alexandre BIQUE`
* **`clap-sys` Bindings de Rust (v0.5.0):**
  - **Repositorio:** [https://github.com/micahrj/clap-sys](https://github.com/micahrj/clap-sys)
  - **Licencia:** Dual-licensed MIT License OR Apache License, Version 2.0
  - **Aviso de Copyright:** `Copyright (c) 2022 Micah Johnston and Robbert van der Helm`
* **`clap-wrapper` (Wrapper AUv2 para macOS, v0.16.0):**
  - **Repositorio:** [https://github.com/free-audio/clap-wrapper](https://github.com/free-audio/clap-wrapper)
  - **Licencia:** MIT License
  - **Aviso de Copyright Oficial:** `Copyright (c) 2022 defiantnerd`

### 3.3. Bindings VST3 (`vst3-sys`) y Marca Registrada de Steinberg
* **`vst3-sys` (v0.1.0):**
  - **Repositorio:** [https://github.com/robbert-vdh/vst3-sys](https://github.com/robbert-vdh/vst3-sys)
  - **Licencia:** GNU General Public License v3.0 (GPLv3)
  - **Autores:** Mike Hilgendorf, Mirko Covizzi, Robbert van der Helm
* **`vst3-com` (v0.1.0):**
  - **Licencia:** MIT License
  - **Aviso de Copyright:** `Copyright (c) Microsoft Corporation.`
* **Aviso de Marca Registrada:**
  - *VST is a registered trademark of Steinberg Media Technologies GmbH.*
  - Nota: El SDK oficial de Steinberg se distribuye bajo la licencia MIT (desde la versión 3.8.0). La obligación copyleft en este plugin proviene de los bindings comunitarios `vst3-sys` (GPLv3) y del propio crate `cstrip2` (GPL-3.0-or-later).

### 3.4. Apple AudioUnitSDK (macOS AUv2 Wrapper)
* **Repositorio:** [https://github.com/apple/AudioUnitSDK](https://github.com/apple/AudioUnitSDK)
* **Tag:** `AudioUnitSDK-1.1.0` (commit `53a9a20`)
* **Licencia:** Apache License, Version 2.0
* **Aviso de Copyright:** `Copyright (c) 2021 Apple Inc. All rights reserved.`

### 3.5. NIH-plug Plugin Framework
* **Autor:** Robbert van der Helm
* **Repositorio:** [https://github.com/robbert-vdh/nih-plug](https://github.com/robbert-vdh/nih-plug)
* **Licencia:** ISC License
* **Aviso de Copyright:** `Copyright (c) 2022-2024 Robbert van der Helm`

### 3.6. Interfaz Gráfica (GUI), Ventanas y Renderizado
* **`egui` & `egui_glow` (v0.31.1):**
  - **Licencia:** MIT OR Apache-2.0
  - **Aviso de Copyright:** `Copyright (c) 2020 Emil Ernerfeldt <emil.ernerfeldt@gmail.com>`
* **`baseview` (v0.1.0):**
  - **Licencia:** MIT OR Apache-2.0
  - **Autores declarados en Cargo.toml:** William Light, Charles Saracco, Mirko Covizzi, Micah Johnston, Billy Messenger, Anton Lazarev, Joakim Frostegård, Robbert van der Helm.
  - **Nota sobre el aviso:** El archivo `LICENSE-MIT` del repositorio upstream contiene el texto general de la licencia sin una cabecera literal de copyright individualizada. Los nombres proceden del manifiesto oficial del crate.
* **`egui-baseview` (v0.5.0):**
  - **Licencia:** MIT License
  - **Aviso de Copyright:** `Copyright (c) 2021 Billy Messenger`
* **`glow` (v0.16.0):**
  - **Licencia:** MIT OR Apache-2.0 OR Zlib
  - **Aviso de Copyright:** `Copyright (c) 2019 Joshua Groves, Dzmitry Malyshau`

### 3.7. Fuentes Tipográficas Embebidas en el Binario GUI (`epaint_default_fonts`)
* **Ubuntu Light (`Ubuntu-Light.ttf`):**
  - **Autor:** Canonical Ltd.
  - **Licencia:** Ubuntu Font Licence, Version 1.0 (UFL-1.0)
  - **Aviso de Copyright:** `Copyright (c) 2011 Canonical Ltd.`
* **Noto Emoji (`NotoEmoji-Regular.ttf`):**
  - **Autor:** Google LLC
  - **Licencia:** SIL Open Font License, Version 1.1 (OFL-1.1)
  - **Aviso de Copyright:** `Copyright (c) Google LLC`
* **Hack Regular (`Hack-Regular.ttf`):**
  - **Autores / Proyecto:** Source Foundry Authors (Christopher Simpkins)
  - **Licencia:** MIT License (Hack) y Bitstream Vera License (glifos derivados de Bitstream Vera Sans Mono)
  - **Avisos de Copyright:**
    - `Copyright (c) 2018 Source Foundry Authors`
    - `Copyright (c) 2003 by Bitstream, Inc. All Rights Reserved.`
* **Emoji Icon Font (`emoji-icon-font.ttf`):**
  - **Autor:** John Slegers
  - **Licencia:** MIT License
  - **Aviso de Copyright Oficial:** `Copyright (c) 2014 John Slegers`

### 3.8. Otras Dependencias Principales en Runtime
* **`realfft` (v3.5.0):** MIT License — `Copyright (c) 2021 Henrik Enquist <henrik.enquist@gmail.com>`
* **`rustfft` (v6.4.1):** MIT OR Apache-2.0 — `Copyright (c) 2015 The RustFFT Developers`
* **`atomic_float` (v1.1.0 / v0.1.0):** Apache-2.0 OR MIT OR Unlicense / Zlib — `Copyright (c) 2020, 2024 Thom Chiovoloni <chiovolonit@gmail.com>`
* **`crossbeam-channel` (v0.5.17):** MIT OR Apache-2.0 — `Copyright (c) 2019 The Crossbeam Project Developers`
* **`serde` & `serde_json` (v1.0.229 / v1.0.151):** MIT OR Apache-2.0 — `Copyright (c) 2017 David Tolnay, Erick Tryzelaar`
* **`assert_no_alloc` (v1.1.2):** BSD-1-Clause — `Copyright (c) 2020 Florian Jung <flo@windfis.ch>`
* **`slotmap` (v1.1.1):** Zlib License — `Copyright (c) 2021 Orson Peters <orsonpeters@gmail.com>`

---

## 4. Inventario Completo de Dependencias del Plugin `cstrip2` (264 Paquetes)

| Nombre del Paquete | Versión | Licencia SPDX | Fuente / Repositorio |
|:---|:---|:---|:---|
| `ab_glyph` | `0.2.32` | Apache-2.0 | https://github.com/alexheretic/ab-glyph |
| `ab_glyph_rasterizer` | `0.1.10` | Apache-2.0 | https://github.com/alexheretic/ab-glyph |
| `addr2line` | `0.25.1` | Apache-2.0 OR MIT | https://github.com/gimli-rs/addr2line |
| `adler2` | `2.0.1` | 0BSD OR MIT OR Apache-2.0 | https://github.com/oyvindln/adler2 |
| `ahash` | `0.8.12` | MIT OR Apache-2.0 | https://github.com/tkaitchuck/ahash |
| `android-activity` | `0.6.1` | MIT OR Apache-2.0 | https://github.com/rust-mobile/android-activity |
| `android-properties` | `0.2.2` | MIT | https://github.com/miklelappo/android-properties |
| `anyhow` | `1.0.104` | MIT OR Apache-2.0 | https://github.com/dtolnay/anyhow |
| `anymap3` | `1.1.0` | BlueOak-1.0.0 OR MIT OR Apache-2.0 | https://github.com/reivilibre/anymap3 |
| `as-raw-xcb-connection` | `1.0.1` | MIT OR Apache-2.0 | https://github.com/psychon/as-raw-xcb-connection |
| `assert_no_alloc` | `1.1.2` | BSD-1-Clause | https://github.com/Windfisch/rust-assert-no-alloc |
| `atomic-waker` | `1.1.2` | Apache-2.0 OR MIT | https://github.com/smol-rs/atomic-waker |
| `atomic_float` | `1.1.0` | Apache-2.0 OR MIT OR Unlicense | https://github.com/thomcc/atomic_float |
| `atomic_float` | `0.1.0` | Apache-2.0 OR MIT OR Zlib | https://github.com/thomcc/atomic_float |
| `atomic_refcell` | `0.1.14` | Apache-2.0 OR MIT | https://github.com/mozilla/atomic_refcell |
| `atty` | `0.2.14` | MIT | https://github.com/softprops/atty |
| `autocfg` | `1.5.1` | Apache-2.0 OR MIT | https://github.com/cuviper/autocfg |
| `backtrace` | `0.3.76` | MIT OR Apache-2.0 | https://github.com/rust-lang/backtrace-rs |
| `baseview` | `0.1.0` | MIT OR Apache-2.0 | git+https://github.com/RustAudio/baseview.git?rev=9a0b42c09d712777b2edb4c5e0cb6baf21e988f0#9a0b42c09d712777b2edb4c5e0cb6baf21e988f0 |
| `bitflags` | `2.13.2` | MIT OR Apache-2.0 | https://github.com/bitflags/bitflags |
| `bitflags` | `1.3.2` | MIT/Apache-2.0 | https://github.com/bitflags/bitflags |
| `block` | `0.1.6` | MIT | http://github.com/SSheldon/rust-block |
| `block2` | `0.5.1` | MIT | https://github.com/madsmtm/objc2 |
| `bumpalo` | `3.20.3` | MIT OR Apache-2.0 | https://github.com/fitzgen/bumpalo |
| `bytemuck` | `1.25.2` | Zlib OR Apache-2.0 OR MIT | https://github.com/Lokathor/bytemuck |
| `bytemuck_derive` | `1.12.1` | Zlib OR Apache-2.0 OR MIT | https://github.com/Lokathor/bytemuck |
| `bytes` | `1.12.1` | MIT | https://github.com/tokio-rs/bytes |
| `calloop` | `0.13.0` | MIT | https://github.com/Smithay/calloop |
| `cc` | `1.4.6` | MIT OR Apache-2.0 | https://github.com/rust-lang/cc-rs |
| `cfg-if` | `1.0.5` | MIT OR Apache-2.0 | https://github.com/rust-lang/cfg-if |
| `cfg_aliases` | `0.2.2` | MIT | https://github.com/katharostech/cfg_aliases |
| `clap-sys` | `0.5.0` | MIT/Apache-2.0 | https://github.com/micahrj/clap-sys |
| `clipboard-win` | `5.4.1` | BSL-1.0 | https://github.com/DoumanAsh/clipboard-win |
| `cocoa` | `0.24.1` | MIT / Apache-2.0 | https://github.com/servo/core-foundation-rs |
| `cocoa-foundation` | `0.1.2` | MIT OR Apache-2.0 | https://github.com/servo/core-foundation-rs |
| `combine` | `4.6.8` | MIT | https://github.com/Marwes/combine |
| `concurrent-queue` | `2.5.0` | Apache-2.0 OR MIT | https://github.com/smol-rs/concurrent-queue |
| `copypasta` | `0.10.2` | MIT / Apache-2.0 | https://github.com/alacritty/copypasta |
| `core-foundation` | `0.9.4` | MIT OR Apache-2.0 | https://github.com/servo/core-foundation-rs |
| `core-foundation-sys` | `0.8.7` | MIT OR Apache-2.0 | https://github.com/servo/core-foundation-rs |
| `core-graphics` | `0.23.2` | MIT OR Apache-2.0 | https://github.com/servo/core-foundation-rs |
| `core-graphics` | `0.22.3` | MIT / Apache-2.0 | https://github.com/servo/core-foundation-rs |
| `core-graphics-types` | `0.1.3` | MIT OR Apache-2.0 | https://github.com/servo/core-foundation-rs |
| `crossbeam` | `0.8.5` | MIT OR Apache-2.0 | https://github.com/crossbeam-rs/crossbeam |
| `crossbeam-channel` | `0.5.17` | MIT OR Apache-2.0 | https://github.com/crossbeam-rs/crossbeam |
| `crossbeam-deque` | `0.8.8` | MIT OR Apache-2.0 | https://github.com/crossbeam-rs/crossbeam |
| `crossbeam-epoch` | `0.9.21` | MIT OR Apache-2.0 | https://github.com/crossbeam-rs/crossbeam |
| `crossbeam-queue` | `0.3.14` | MIT OR Apache-2.0 | https://github.com/crossbeam-rs/crossbeam |
| `crossbeam-utils` | `0.8.23` | MIT OR Apache-2.0 | https://github.com/crossbeam-rs/crossbeam |
| `cstrip2` | `0.1.0` | GPL-3.0-or-later | local |
| `cursor-icon` | `1.2.0` | MIT OR Apache-2.0 OR Zlib | https://github.com/rust-windowing/cursor-icon |
| `deranged` | `0.5.8` | MIT OR Apache-2.0 | https://github.com/jhpratt/deranged |
| `dispatch` | `0.2.0` | MIT | http://github.com/SSheldon/rust-dispatch |
| `dlib` | `0.5.3` | MIT | https://github.com/elinorbgr/dlib |
| `dpi` | `0.1.2` | Apache-2.0 AND MIT | https://github.com/rust-windowing/winit |
| `ecolor` | `0.31.1` | MIT OR Apache-2.0 | https://github.com/emilk/egui |
| `egui` | `0.31.1` | MIT OR Apache-2.0 | https://github.com/emilk/egui |
| `egui-baseview` | `0.5.0` | MIT | https://github.com/BillyDM/egui-baseview |
| `egui_glow` | `0.31.1` | MIT OR Apache-2.0 | https://github.com/emilk/egui/tree/master/crates/egui_glow |
| `emath` | `0.31.1` | MIT OR Apache-2.0 | https://github.com/emilk/egui/tree/master/crates/emath |
| `epaint` | `0.31.1` | MIT OR Apache-2.0 | https://github.com/emilk/egui/tree/master/crates/epaint |
| `epaint_default_fonts` | `0.31.1` | (MIT OR Apache-2.0) AND OFL-1.1 AND Ubuntu-font-1.0 | https://github.com/emilk/egui/tree/master/crates/epaint_default_fonts |
| `equivalent` | `1.0.2` | Apache-2.0 OR MIT | https://github.com/indexmap-rs/equivalent |
| `errno` | `0.3.14` | MIT OR Apache-2.0 | https://github.com/lambda-fairy/rust-errno |
| `error-code` | `3.4.0` | BSL-1.0 | https://github.com/DoumanAsh/error-code |
| `find-msvc-tools` | `0.1.12` | MIT OR Apache-2.0 | https://github.com/rust-lang/cc-rs |
| `foreign-types` | `0.5.0` | MIT/Apache-2.0 | https://github.com/sfackler/foreign-types |
| `foreign-types` | `0.3.2` | MIT/Apache-2.0 | https://github.com/sfackler/foreign-types |
| `foreign-types-macros` | `0.2.4` | MIT/Apache-2.0 | https://github.com/sfackler/foreign-types |
| `foreign-types-shared` | `0.1.1` | MIT/Apache-2.0 | https://github.com/sfackler/foreign-types |
| `foreign-types-shared` | `0.3.1` | MIT/Apache-2.0 | https://github.com/sfackler/foreign-types |
| `futures-core` | `0.3.34` | MIT OR Apache-2.0 | https://github.com/rust-lang/futures-rs |
| `futures-task` | `0.3.34` | MIT OR Apache-2.0 | https://github.com/rust-lang/futures-rs |
| `futures-util` | `0.3.34` | MIT OR Apache-2.0 | https://github.com/rust-lang/futures-rs |
| `gethostname` | `1.1.0` | Apache-2.0 | https://codeberg.org/swsnr/gethostname.rs.git |
| `getrandom` | `0.4.3` | MIT OR Apache-2.0 | https://github.com/rust-random/getrandom |
| `getrandom` | `0.2.17` | MIT OR Apache-2.0 | https://github.com/rust-random/getrandom |
| `gimli` | `0.32.3` | MIT OR Apache-2.0 | https://github.com/gimli-rs/gimli |
| `glow` | `0.16.0` | MIT OR Apache-2.0 OR Zlib | https://github.com/grovesNL/glow |
| `hashbrown` | `0.17.1` | MIT OR Apache-2.0 | https://github.com/rust-lang/hashbrown |
| `hermit-abi` | `0.5.3` | MIT OR Apache-2.0 | https://github.com/hermit-os/hermit-rs |
| `hermit-abi` | `0.1.19` | MIT/Apache-2.0 | https://github.com/hermitcore/libhermit-rs |
| `indexmap` | `2.14.2` | Apache-2.0 OR MIT | https://github.com/indexmap-rs/indexmap |
| `is-docker` | `0.2.0` | MIT | https://github.com/TheLarkInn/is-docker |
| `is-wsl` | `0.4.0` | MIT | https://github.com/TheLarkInn/is-wsl |
| `itoa` | `1.0.18` | MIT OR Apache-2.0 | https://github.com/dtolnay/itoa |
| `jni` | `0.22.4` | MIT OR Apache-2.0 | https://github.com/jni-rs/jni-rs |
| `jni-macros` | `0.22.4` | MIT OR Apache-2.0 | https://github.com/jni-rs/jni-rs |
| `jni-sys` | `0.4.1` | MIT OR Apache-2.0 | https://github.com/jni-rs/jni-sys |
| `jni-sys` | `0.3.1` | MIT OR Apache-2.0 | https://github.com/jni-rs/jni-sys |
| `jni-sys-macros` | `0.4.1` | MIT OR Apache-2.0 | https://github.com/jni-rs/jni-sys |
| `jobserver` | `0.1.35` | MIT OR Apache-2.0 | https://github.com/rust-lang/jobserver-rs |
| `js-sys` | `0.3.105` | MIT OR Apache-2.0 | https://github.com/wasm-bindgen/wasm-bindgen/tree/master/crates/js-sys |
| `keyboard-types` | `0.6.2` | MIT/Apache-2.0 | https://github.com/pyfisch/keyboard-types |
| `libc` | `0.2.189` | MIT OR Apache-2.0 | https://github.com/rust-lang/libc |
| `libloading` | `0.8.9` | ISC | https://github.com/nagisa/rust_libloading/ |
| `libredox` | `0.1.24` | MIT | https://gitlab.redox-os.org/redox-os/libredox.git |
| `linux-raw-sys` | `0.12.1` | Apache-2.0 WITH LLVM-exception OR Apache-2.0 OR MIT | https://github.com/sunfishcode/linux-raw-sys |
| `linux-raw-sys` | `0.4.15` | Apache-2.0 WITH LLVM-exception OR Apache-2.0 OR MIT | https://github.com/sunfishcode/linux-raw-sys |
| `lock_api` | `0.4.14` | MIT OR Apache-2.0 | https://github.com/Amanieu/parking_lot |
| `log` | `0.4.34` | MIT OR Apache-2.0 | https://github.com/rust-lang/log |
| `malloc_buf` | `0.0.6` | MIT | https://github.com/SSheldon/malloc_buf |
| `memchr` | `2.8.3` | Unlicense OR MIT | https://github.com/BurntSushi/memchr |
| `memoffset` | `0.6.5` | MIT | https://github.com/Gilnaa/memoffset |
| `memoffset` | `0.9.1` | MIT | https://github.com/Gilnaa/memoffset |
| `midi-consts` | `0.1.0` | MIT OR Apache-2.0 | https://codeberg.org/PieterPenninckx/midi-consts |
| `miniz_oxide` | `0.8.9` | MIT OR Zlib OR Apache-2.0 | https://github.com/Frommi/miniz_oxide/tree/master/miniz_oxide |
| `ndk` | `0.9.0` | MIT OR Apache-2.0 | https://github.com/rust-mobile/ndk |
| `ndk-context` | `0.1.1` | MIT OR Apache-2.0 | https://github.com/rust-windowing/android-ndk-rs |
| `ndk-sys` | `0.6.0+11769913` | MIT OR Apache-2.0 | https://github.com/rust-mobile/ndk |
| `nih_log` | `0.3.1` | ISC | https://github.com/robbert-vdh/nih-log |
| `nih_plug` | `0.0.0` | ISC | https://github.com/robbert-vdh/nih-plug |
| `nih_plug_derive` | `0.1.0` | ISC | git+https://github.com/robbert-vdh/nih-plug.git#de421011f41a6d10fc8c7a6084e4f4dee0143683 |
| `nih_plug_egui` | `0.0.0` | ISC | git+https://github.com/robbert-vdh/nih-plug.git#de421011f41a6d10fc8c7a6084e4f4dee0143683 |
| `nix` | `0.22.3` | MIT | https://github.com/nix-rust/nix |
| `nohash-hasher` | `0.2.0` | Apache-2.0 OR MIT | https://github.com/paritytech/nohash-hasher |
| `num-complex` | `0.4.6` | MIT OR Apache-2.0 | https://github.com/rust-num/num-complex |
| `num-conv` | `0.2.2` | MIT OR Apache-2.0 | https://github.com/jhpratt/num-conv |
| `num-integer` | `0.1.47` | MIT OR Apache-2.0 | https://github.com/rust-num/num-integer |
| `num-traits` | `0.2.19` | MIT OR Apache-2.0 | https://github.com/rust-num/num-traits |
| `num_enum` | `0.7.6` | BSD-3-Clause OR MIT OR Apache-2.0 | https://github.com/illicitonion/num_enum |
| `num_enum_derive` | `0.7.6` | BSD-3-Clause OR MIT OR Apache-2.0 | https://github.com/illicitonion/num_enum |
| `num_threads` | `0.1.7` | MIT OR Apache-2.0 | https://github.com/jhpratt/num_threads |
| `objc` | `0.2.7` | MIT | http://github.com/SSheldon/rust-objc |
| `objc-sys` | `0.3.5` | MIT | https://github.com/madsmtm/objc2 |
| `objc2` | `0.5.2` | MIT | https://github.com/madsmtm/objc2 |
| `objc2-app-kit` | `0.2.2` | MIT | https://github.com/madsmtm/objc2 |
| `objc2-cloud-kit` | `0.2.2` | MIT | https://github.com/madsmtm/objc2 |
| `objc2-contacts` | `0.2.2` | MIT | https://github.com/madsmtm/objc2 |
| `objc2-core-data` | `0.2.2` | MIT | https://github.com/madsmtm/objc2 |
| `objc2-core-image` | `0.2.2` | MIT | https://github.com/madsmtm/objc2 |
| `objc2-core-location` | `0.2.2` | MIT | https://github.com/madsmtm/objc2 |
| `objc2-encode` | `4.1.0` | MIT | https://github.com/madsmtm/objc2 |
| `objc2-foundation` | `0.2.2` | MIT | https://github.com/madsmtm/objc2 |
| `objc2-link-presentation` | `0.2.2` | MIT | https://github.com/madsmtm/objc2 |
| `objc2-metal` | `0.2.2` | MIT | https://github.com/madsmtm/objc2 |
| `objc2-quartz-core` | `0.2.2` | MIT | https://github.com/madsmtm/objc2 |
| `objc2-symbols` | `0.2.2` | MIT | https://github.com/madsmtm/objc2 |
| `objc2-ui-kit` | `0.2.2` | MIT | https://github.com/madsmtm/objc2 |
| `objc2-uniform-type-identifiers` | `0.2.2` | MIT | https://github.com/madsmtm/objc2 |
| `objc2-user-notifications` | `0.2.2` | MIT | https://github.com/madsmtm/objc2 |
| `object` | `0.37.3` | Apache-2.0 OR MIT | https://github.com/gimli-rs/object |
| `once_cell` | `1.21.4` | MIT OR Apache-2.0 | https://github.com/matklad/once_cell |
| `open` | `5.4.4` | MIT | https://github.com/Byron/open-rs |
| `orbclient` | `0.3.55` | MIT | https://gitlab.redox-os.org/redox-os/orbclient |
| `owned_ttf_parser` | `0.25.1` | Apache-2.0 | https://github.com/alexheretic/owned-ttf-parser |
| `parking_lot` | `0.12.5` | MIT OR Apache-2.0 | https://github.com/Amanieu/parking_lot |
| `parking_lot_core` | `0.9.12` | MIT OR Apache-2.0 | https://github.com/Amanieu/parking_lot |
| `percent-encoding` | `2.3.2` | MIT OR Apache-2.0 | https://github.com/servo/rust-url/ |
| `pin-project` | `1.1.13` | Apache-2.0 OR MIT | https://github.com/taiki-e/pin-project |
| `pin-project-internal` | `1.1.13` | Apache-2.0 OR MIT | https://github.com/taiki-e/pin-project |
| `pin-project-lite` | `0.2.17` | Apache-2.0 OR MIT | https://github.com/taiki-e/pin-project-lite |
| `pkg-config` | `0.3.34` | MIT OR Apache-2.0 | https://github.com/rust-lang/pkg-config-rs |
| `plain` | `0.2.3` | MIT/Apache-2.0 | https://github.com/randomites/plain |
| `polling` | `3.11.0` | Apache-2.0 OR MIT | https://github.com/smol-rs/polling |
| `powerfmt` | `0.2.0` | MIT OR Apache-2.0 | https://github.com/jhpratt/powerfmt |
| `primal-check` | `0.3.4` | MIT OR Apache-2.0 | https://github.com/huonw/primal |
| `proc-macro-crate` | `3.5.0` | MIT OR Apache-2.0 | https://github.com/bkchr/proc-macro-crate |
| `proc-macro2` | `1.0.107` | MIT OR Apache-2.0 | https://github.com/dtolnay/proc-macro2 |
| `profiling` | `1.0.18` | MIT OR Apache-2.0 | https://github.com/aclysma/profiling |
| `quote` | `1.0.47` | MIT OR Apache-2.0 | https://github.com/dtolnay/quote |
| `r-efi` | `6.0.0` | MIT OR Apache-2.0 OR LGPL-2.1-or-later | https://github.com/r-efi/r-efi |
| `raw-window-handle` | `0.6.2` | MIT OR Apache-2.0 OR Zlib | https://github.com/rust-windowing/raw-window-handle |
| `raw-window-handle` | `0.5.2` | MIT OR Apache-2.0 OR Zlib | https://github.com/rust-windowing/raw-window-handle |
| `realfft` | `3.5.0` | MIT | https://github.com/HEnquist/realfft |
| `redox_syscall` | `0.4.1` | MIT | https://gitlab.redox-os.org/redox-os/syscall |
| `redox_syscall` | `0.5.18` | MIT | https://gitlab.redox-os.org/redox-os/syscall |
| `redox_syscall` | `0.9.4` | MIT | https://gitlab.redox-os.org/redox-os/kernel |
| `rustc-demangle` | `0.1.28` | MIT/Apache-2.0 | https://github.com/rust-lang/rustc-demangle |
| `rustc_version` | `0.4.1` | MIT OR Apache-2.0 | https://github.com/djc/rustc-version-rs |
| `rustfft` | `6.4.1` | MIT OR Apache-2.0 | https://github.com/ejmahler/RustFFT |
| `rustix` | `1.1.5` | Apache-2.0 WITH LLVM-exception OR Apache-2.0 OR MIT | https://github.com/bytecodealliance/rustix |
| `rustix` | `0.38.44` | Apache-2.0 WITH LLVM-exception OR Apache-2.0 OR MIT | https://github.com/bytecodealliance/rustix |
| `rustversion` | `1.0.23` | MIT OR Apache-2.0 | https://github.com/dtolnay/rustversion |
| `same-file` | `1.0.6` | Unlicense/MIT | https://github.com/BurntSushi/same-file |
| `scopeguard` | `1.2.0` | MIT OR Apache-2.0 | https://github.com/bluss/scopeguard |
| `semver` | `1.0.28` | MIT OR Apache-2.0 | https://github.com/dtolnay/semver |
| `serde` | `1.0.229` | MIT OR Apache-2.0 | https://github.com/serde-rs/serde |
| `serde_core` | `1.0.229` | MIT OR Apache-2.0 | https://github.com/serde-rs/serde |
| `serde_derive` | `1.0.229` | MIT OR Apache-2.0 | https://github.com/serde-rs/serde |
| `serde_json` | `1.0.151` | MIT OR Apache-2.0 | https://github.com/serde-rs/json |
| `shlex` | `2.0.1` | MIT OR Apache-2.0 | https://github.com/comex/rust-shlex |
| `simd_cesu8` | `1.2.0` | Apache-2.0 OR MIT | https://github.com/seancroach/simd_cesu8 |
| `simdutf8` | `0.1.5` | MIT OR Apache-2.0 | https://github.com/rusticstuff/simdutf8 |
| `slab` | `0.4.12` | MIT | https://github.com/tokio-rs/slab |
| `slotmap` | `1.1.1` | Zlib | https://github.com/orlp/slotmap |
| `smallvec` | `1.16.1` | MIT OR Apache-2.0 | https://github.com/servo/rust-smallvec |
| `smol_str` | `0.2.2` | MIT OR Apache-2.0 | https://github.com/rust-analyzer/smol_str |
| `strength_reduce` | `0.2.4` | MIT OR Apache-2.0 | http://github.com/ejmahler/strength_reduce |
| `syn` | `1.0.109` | MIT OR Apache-2.0 | https://github.com/dtolnay/syn |
| `syn` | `2.0.119` | MIT OR Apache-2.0 | https://github.com/dtolnay/syn |
| `syn` | `3.0.6` | MIT OR Apache-2.0 | https://github.com/dtolnay/syn |
| `termcolor` | `1.4.1` | Unlicense OR MIT | https://github.com/BurntSushi/termcolor |
| `thiserror` | `1.0.69` | MIT OR Apache-2.0 | https://github.com/dtolnay/thiserror |
| `thiserror` | `2.0.20` | MIT OR Apache-2.0 | https://github.com/dtolnay/thiserror |
| `thiserror-impl` | `1.0.69` | MIT OR Apache-2.0 | https://github.com/dtolnay/thiserror |
| `thiserror-impl` | `2.0.20` | MIT OR Apache-2.0 | https://github.com/dtolnay/thiserror |
| `time` | `0.3.55` | MIT OR Apache-2.0 | https://github.com/time-rs/time |
| `time-core` | `0.1.9` | MIT OR Apache-2.0 | https://github.com/time-rs/time |
| `time-macros` | `0.2.32` | MIT OR Apache-2.0 | https://github.com/time-rs/time |
| `toml_datetime` | `1.1.1+spec-1.1.0` | MIT OR Apache-2.0 | https://github.com/toml-rs/toml |
| `toml_edit` | `0.25.15+spec-1.1.0` | MIT OR Apache-2.0 | https://github.com/toml-rs/toml |
| `toml_parser` | `1.1.3+spec-1.1.0` | MIT OR Apache-2.0 | https://github.com/toml-rs/toml |
| `tracing` | `0.1.44` | MIT | https://github.com/tokio-rs/tracing |
| `tracing-core` | `0.1.36` | MIT | https://github.com/tokio-rs/tracing |
| `transpose` | `0.2.3` | MIT OR Apache-2.0 | https://github.com/ejmahler/transpose |
| `ttf-parser` | `0.25.1` | MIT OR Apache-2.0 | https://github.com/harfbuzz/ttf-parser |
| `unicode-ident` | `1.0.26` | (MIT OR Apache-2.0) AND Unicode-3.0 | https://github.com/dtolnay/unicode-ident |
| `unicode-segmentation` | `1.13.3` | MIT OR Apache-2.0 | https://github.com/unicode-rs/unicode-segmentation |
| `uuid` | `0.8.2` | Apache-2.0 OR MIT | https://github.com/uuid-rs/uuid |
| `version_check` | `0.9.5` | MIT/Apache-2.0 | https://github.com/SergioBenitez/version_check |
| `vst3-com` | `0.1.0` | MIT | git+https://github.com/robbert-vdh/vst3-sys.git?branch=fix/drop-box-from-raw#b3ff4d775940f5b476b9d1cca02a90e07e1922a2 |
| `vst3-com-macros` | `0.2.0` | MIT | git+https://github.com/robbert-vdh/vst3-sys.git?branch=fix/drop-box-from-raw#b3ff4d775940f5b476b9d1cca02a90e07e1922a2 |
| `vst3-com-macros-support` | `0.2.0` | MIT | git+https://github.com/robbert-vdh/vst3-sys.git?branch=fix/drop-box-from-raw#b3ff4d775940f5b476b9d1cca02a90e07e1922a2 |
| `vst3-sys` | `0.1.0` | GPLv3 | git+https://github.com/robbert-vdh/vst3-sys.git?branch=fix/drop-box-from-raw#b3ff4d775940f5b476b9d1cca02a90e07e1922a2 |
| `walkdir` | `2.5.0` | Unlicense/MIT | https://github.com/BurntSushi/walkdir |
| `wasi` | `0.11.1+wasi-snapshot-preview1` | Apache-2.0 WITH LLVM-exception OR Apache-2.0 OR MIT | https://github.com/bytecodealliance/wasi |
| `wasm-bindgen` | `0.2.128` | MIT OR Apache-2.0 | https://github.com/wasm-bindgen/wasm-bindgen |
| `wasm-bindgen-futures` | `0.4.78` | MIT OR Apache-2.0 | https://github.com/wasm-bindgen/wasm-bindgen/tree/master/crates/futures |
| `wasm-bindgen-macro` | `0.2.128` | MIT OR Apache-2.0 | https://github.com/wasm-bindgen/wasm-bindgen/tree/master/crates/macro |
| `wasm-bindgen-macro-support` | `0.2.128` | MIT OR Apache-2.0 | https://github.com/wasm-bindgen/wasm-bindgen/tree/main/crates/macro-support |
| `wasm-bindgen-shared` | `0.2.128` | MIT OR Apache-2.0 | https://github.com/wasm-bindgen/wasm-bindgen/tree/master/crates/shared |
| `web-sys` | `0.3.105` | MIT OR Apache-2.0 | https://github.com/wasm-bindgen/wasm-bindgen/tree/master/crates/web-sys |
| `web-time` | `1.1.0` | MIT OR Apache-2.0 | https://github.com/daxpedda/web-time |
| `widestring` | `1.2.1` | MIT OR Apache-2.0 | https://github.com/VoidStarKat/widestring-rs |
| `winapi` | `0.3.9` | MIT/Apache-2.0 | https://github.com/retep998/winapi-rs |
| `winapi-i686-pc-windows-gnu` | `0.4.0` | MIT/Apache-2.0 | https://github.com/retep998/winapi-rs |
| `winapi-util` | `0.1.11` | Unlicense OR MIT | https://github.com/BurntSushi/winapi-util |
| `winapi-x86_64-pc-windows-gnu` | `0.4.0` | MIT/Apache-2.0 | https://github.com/retep998/winapi-rs |
| `windows` | `0.44.0` | MIT OR Apache-2.0 | https://github.com/microsoft/windows-rs |
| `windows-link` | `0.2.1` | MIT OR Apache-2.0 | https://github.com/microsoft/windows-rs |
| `windows-sys` | `0.52.0` | MIT OR Apache-2.0 | https://github.com/microsoft/windows-rs |
| `windows-sys` | `0.59.0` | MIT OR Apache-2.0 | https://github.com/microsoft/windows-rs |
| `windows-sys` | `0.61.2` | MIT OR Apache-2.0 | https://github.com/microsoft/windows-rs |
| `windows-targets` | `0.42.2` | MIT OR Apache-2.0 | https://github.com/microsoft/windows-rs |
| `windows-targets` | `0.52.6` | MIT OR Apache-2.0 | https://github.com/microsoft/windows-rs |
| `windows_aarch64_gnullvm` | `0.52.6` | MIT OR Apache-2.0 | https://github.com/microsoft/windows-rs |
| `windows_aarch64_gnullvm` | `0.42.2` | MIT OR Apache-2.0 | https://github.com/microsoft/windows-rs |
| `windows_aarch64_msvc` | `0.42.2` | MIT OR Apache-2.0 | https://github.com/microsoft/windows-rs |
| `windows_aarch64_msvc` | `0.52.6` | MIT OR Apache-2.0 | https://github.com/microsoft/windows-rs |
| `windows_i686_gnu` | `0.52.6` | MIT OR Apache-2.0 | https://github.com/microsoft/windows-rs |
| `windows_i686_gnu` | `0.42.2` | MIT OR Apache-2.0 | https://github.com/microsoft/windows-rs |
| `windows_i686_gnullvm` | `0.52.6` | MIT OR Apache-2.0 | https://github.com/microsoft/windows-rs |
| `windows_i686_msvc` | `0.42.2` | MIT OR Apache-2.0 | https://github.com/microsoft/windows-rs |
| `windows_i686_msvc` | `0.52.6` | MIT OR Apache-2.0 | https://github.com/microsoft/windows-rs |
| `windows_x86_64_gnu` | `0.52.6` | MIT OR Apache-2.0 | https://github.com/microsoft/windows-rs |
| `windows_x86_64_gnu` | `0.42.2` | MIT OR Apache-2.0 | https://github.com/microsoft/windows-rs |
| `windows_x86_64_gnullvm` | `0.52.6` | MIT OR Apache-2.0 | https://github.com/microsoft/windows-rs |
| `windows_x86_64_gnullvm` | `0.42.2` | MIT OR Apache-2.0 | https://github.com/microsoft/windows-rs |
| `windows_x86_64_msvc` | `0.42.2` | MIT OR Apache-2.0 | https://github.com/microsoft/windows-rs |
| `windows_x86_64_msvc` | `0.52.6` | MIT OR Apache-2.0 | https://github.com/microsoft/windows-rs |
| `winit` | `0.30.13` | Apache-2.0 | https://github.com/rust-windowing/winit |
| `winnow` | `1.0.4` | MIT | https://github.com/winnow-rs/winnow |
| `x11` | `2.21.0` | MIT | https://github.com/AltF02/x11-rs.git |
| `x11-clipboard` | `0.9.3` | MIT | https://github.com/quininer/x11-clipboard |
| `x11-dl` | `2.21.0` | MIT | https://github.com/AltF02/x11-rs.git |
| `x11rb` | `0.13.2` | MIT OR Apache-2.0 | https://github.com/psychon/x11rb |
| `x11rb-protocol` | `0.13.2` | MIT OR Apache-2.0 | https://github.com/psychon/x11rb |
| `xcursor` | `0.3.11` | MIT | https://github.com/esposm03/xcursor-rs |
| `xkbcommon-dl` | `0.4.2` | MIT | https://github.com/rust-windowing/xkbcommon-dl |
| `xkeysym` | `0.2.1` | MIT OR Apache-2.0 OR Zlib | https://github.com/notgull/xkeysym |
| `zerocopy` | `0.8.57` | BSD-2-Clause OR Apache-2.0 OR MIT | https://github.com/google/zerocopy |
| `zerocopy-derive` | `0.8.57` | BSD-2-Clause OR Apache-2.0 OR MIT | https://github.com/google/zerocopy |
| `zmij` | `1.0.23` | MIT | https://github.com/dtolnay/zmij |

---

## 5. Textos Íntegros Oficiales de Licencias de Terceros

### 5.1. GNU General Public License, Version 3.0 (GPLv3)
*El texto íntegro completo oficial de la GNU General Public License v3.0 se acompaña de forma íntegra en el archivo [`LICENSE.txt`](LICENSE.txt).*

---

### 5.2. MIT License (Avisos de Copyright y Atribuciones)

#### A. Avisos de Copyright Literales Verificados (Presentes en Archivos Upstream):
* **Airwindows (CStrip2 DSP):** `Copyright (c) 2018 Chris Johnson` (en `LICENSE`)
* **CLAP Specification (Headers C v1.2.3):** `Copyright (c) 2021 Alexandre BIQUE` (en `LICENSE`)
* **clap-wrapper (v0.16.0):** `Copyright (c) 2022 defiantnerd` (en `LICENSE`)
* **clap-sys (v0.5.0):** `Copyright (c) 2022 Micah Johnston and Robbert van der Helm` (en `LICENSE-MIT`)
* **Microsoft Corporation (`vst3-com`):** `Copyright (c) Microsoft Corporation.` (en `LICENSE`)
* **egui & epaint:** `Copyright (c) 2020 Emil Ernerfeldt <emil.ernerfeldt@gmail.com>` (en `LICENSE-MIT`)
* **egui-baseview:** `Copyright (c) 2021 Billy Messenger` (en `LICENSE`)
* **realfft:** `Copyright (c) 2021 Henrik Enquist <henrik.enquist@gmail.com>` (en `LICENSE-MIT`)
* **rustfft:** `Copyright (c) 2015 The RustFFT Developers` (en `LICENSE-MIT`)
* **atomic_float:** `Copyright (c) 2020 Thom Chiovoloni`, `Copyright (c) 2024 Thom Chiovoloni` (en `LICENSE-MIT`)
* **Crossbeam (`crossbeam-channel`):** `Copyright (c) 2019 The Crossbeam Project Developers` (en `LICENSE-MIT`)
* **Serde:** `Copyright (c) 2017 David Tolnay`, `Copyright (c) 2017 Erick Tryzelaar` (en `LICENSE-MIT`)
* **Hack Font (`Hack-Regular.txt`):** `Copyright 2018 Source Foundry Authors` (en `fonts/Hack-Regular.txt`)
* **Bitstream Vera Sans Mono:** `Copyright (c) 2003 by Bitstream, Inc. All Rights Reserved.` (en `fonts/Hack-Regular.txt`)
* **Emoji Icon Font:** `Copyright (c) 2014 John Slegers` (en `fonts/emoji-icon-font-mit-license.txt`)

#### B. Atribuciones Derivadas de Manifiestos de Cargo (Sin Aviso Literal en el Archivo de Licencia):
*Los siguientes crates adoptan la licencia MIT o dual MIT/Apache-2.0 incluyendo el texto genérico de la licencia sin una línea formal de copyright con años y nombres. En estricta precisión legal, no se presentan como avisos oficiales reconstruidos, sino como los autores acreditados en sus respectivos manifiestos `Cargo.toml`:*
* **`baseview` (v0.1.0):** William Light, Charles Saracco, Mirko Covizzi, Micah Johnston, Billy Messenger, Anton Lazarev, Joakim Frostegård, Robbert van der Helm.
* **`block` (v0.1.6) / `objc` (v0.2.7):** Steven Sheldon.
* **`block2` (v0.5.1) / `objc2` (v0.5.2) / `objc-sys` (v0.3.5) / `objc2-encode` (v4.1.0):** Steven Sheldon, Mads Marquart.
* **`bytes` (v1.12.1):** Carl Lerche, Sean McArthur.
* **`calloop` (v0.13.0) / `dlib` (v0.5.3):** Elinor Berger.
* **`memoffset` (v0.9.1 / v0.6.5):** Gilad Naaman.
* **`nix` (v0.22.3):** The nix-rust Project Developers.
* **`slab` (v0.4.12):** Carl Lerche.
* **`tracing` (v0.1.44) / `tracing-core` (v0.1.36):** Eliza Weisman, Tokio Contributors.
* **`x11` (v2.21.0) / `x11-dl` (v2.21.0):** daggerbot, Erle Pereira, AltF02.
* **`zmij` (v1.0.23):** David Tolnay.

#### Texto Oficial de Condiciones:
```text
Permission is hereby granted, free of charge, to any person obtaining a copy
of this software and associated documentation files (the "Software"), to deal
in the Software without restriction, including without limitation the rights
to use, copy, modify, merge, publish, distribute, sublicense, and/or sell
copies of the Software, and to permit persons to whom the Software is
furnished to do so, subject to the following conditions:

The above copyright notice and this permission notice shall be included in all
copies or substantial portions of the Software.

THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND, EXPRESS OR
IMPLIED, INCLUDING BUT NOT LIMITED TO THE WARRANTIES OF MERCHANTABILITY,
FITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT. IN NO EVENT SHALL THE
AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY CLAIM, DAMAGES OR OTHER
LIABILITY, WHETHER IN AN ACTION OF CONTRACT, TORT OR OTHERWISE, ARISING FROM,
OUT OF OR IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER DEALINGS IN THE
SOFTWARE.
```

---

### 5.3. Apache License, Version 2.0 (Texto Íntegro Oficial)
*Texto oficial completo obtenido de las fuentes de las dependencias incorporadas (`AudioUnitSDK`, `serde`, `winit`, etc.):*

```text
                              Apache License
                        Version 2.0, January 2004
                     http://www.apache.org/licenses/

TERMS AND CONDITIONS FOR USE, REPRODUCTION, AND DISTRIBUTION

1. Definitions.

   "License" shall mean the terms and conditions for use, reproduction,
   and distribution as defined by Sections 1 through 9 of this document.

   "Licensor" shall mean the copyright owner or entity authorized by
   the copyright owner that is granting the License.

   "Legal Entity" shall mean the union of the acting entity and all
   other entities that control, are controlled by, or are under common
   control with that entity. For the purposes of this definition,
   "control" means (i) the power, direct or indirect, to cause the
   direction or management of such entity, whether by contract or
   otherwise, or (ii) ownership of fifty percent (50%) or more of the
   outstanding shares, or (iii) beneficial ownership of such entity.

   "You" (or "Your") shall mean an individual or Legal Entity
   exercising permissions granted by this License.

   "Source" form shall mean the preferred form for making modifications,
   including but not limited to software source code, documentation
   source, and configuration files.

   "Object" form shall mean any form resulting from mechanical
   transformation or translation of a Source form, including but
   not limited to compiled object code, generated documentation,
   and conversions to other media types.

   "Work" shall mean the work of authorship, whether in Source or
   Object form, made available under the License, as indicated by a
   copyright notice that is included in or attached to the work
   (an example is provided in the Appendix below).

   "Derivative Works" shall mean any work, whether in Source or Object
   form, that is based on (or derived from) the Work and for which the
   editorial revisions, annotations, elaborations, or other modifications
   represent, as a whole, an original work of authorship. For the purposes
   of this License, Derivative Works shall not include works that remain
   separable from, or merely link (or bind by name) to the interfaces of,
   the Work and Derivative Works thereof.

   "Contribution" shall mean any work of authorship, including
   the original version of the Work and any modifications or additions
   to that Work or Derivative Works thereof, that is intentionally
   submitted to Licensor for inclusion in the Work by the copyright owner
   or by an individual or Legal Entity authorized to submit on behalf of
   the copyright owner. For the purposes of this definition, "submitted"
   means any form of electronic, verbal, or written communication sent
   to the Licensor or its representatives, including but not limited to
   communication on electronic mailing lists, source code control systems,
   and issue tracking systems that are managed by, or on behalf of, the
   Licensor for the purpose of discussing and improving the Work, but
   excluding communication that is conspicuously marked or otherwise
   designated in writing by the copyright owner as "Not a Contribution."

   "Contributor" shall mean Licensor and any individual or Legal Entity
   on behalf of whom a Contribution has been received by Licensor and
   subsequently incorporated within the Work.

2. Grant of Copyright License. Subject to the terms and conditions of
   this License, each Contributor hereby grants to You a perpetual,
   worldwide, non-exclusive, no-charge, royalty-free, irrevocable
   copyright license to reproduce, prepare Derivative Works of,
   publicly display, publicly perform, sublicense, and distribute the
   Work and such Derivative Works in Source or Object form.

3. Grant of Patent License. Subject to the terms and conditions of
   this License, each Contributor hereby grants to You a perpetual,
   worldwide, non-exclusive, no-charge, royalty-free, irrevocable
   (except as stated in this section) patent license to make, have made,
   use, offer to sell, sell, import, and otherwise transfer the Work,
   where such license applies only to those patent claims licensable
   by such Contributor that are necessarily infringed by their
   Contribution(s) alone or by combination of their Contribution(s)
   with the Work to which such Contribution(s) was submitted. If You
   institute patent litigation against any entity (including a
   cross-claim or counterclaim in a lawsuit) alleging that the Work
   or a Contribution incorporated within the Work constitutes direct
   or contributory patent infringement, then any patent licenses
   granted to You under this License for that Work shall terminate
   as of the date such litigation is filed.

4. Redistribution. You may reproduce and distribute copies of the
   Work or Derivative Works thereof in any medium, with or without
   modifications, and in Source or Object form, provided that You
   meet the following conditions:

   (a) You must give any other recipients of the Work or
       Derivative Works a copy of this License; and

   (b) You must cause any modified files to carry prominent notices
       stating that You changed the files; and

   (c) You must retain, in the Source form of any Derivative Works
       that You distribute, all copyright, patent, trademark, and
       attribution notices from the Source form of the Work,
       excluding those notices that do not pertain to any part of
       the Derivative Works; and

   (d) If the Work includes a "NOTICE" text file as part of its
       distribution, then any Derivative Works that You distribute must
       include a readable copy of the attribution notices contained
       within such NOTICE file, excluding those notices that do not
       pertain to any part of the Derivative Works, in at least one
       of the following places: within a NOTICE text file distributed
       as part of the Derivative Works; within the Source form or
       documentation, if provided along with the Derivative Works; or,
       within a display generated by the Derivative Works, if and
       wherever such third-party notices normally appear. The contents
       of the NOTICE file are for informational purposes only and
       do not modify the License. You may add Your own attribution
       notices within Derivative Works that You distribute, alongside
       or as an addendum to the NOTICE text from the Work, provided
       that such additional attribution notices cannot be construed
       as modifying the License.

   You may add Your own copyright statement to Your modifications and
   may provide additional or different license terms and conditions
   for use, reproduction, or distribution of Your modifications, or
   for any such Derivative Works as a whole, provided Your use,
   reproduction, and distribution of the Work otherwise complies with
   the conditions stated in this License.

5. Submission of Contributions. Unless You explicitly state otherwise,
   any Contribution intentionally submitted for inclusion in the Work
   by You to the Licensor shall be under the terms and conditions of
   this License, without any additional terms or conditions.
   Notwithstanding the above, nothing herein shall supersede or modify
   the terms of any separate license agreement you may have executed
   with Licensor regarding such Contributions.

6. Trademarks. This License does not grant permission to use the trade
   names, trademarks, service marks, or product names of the Licensor,
   except as required for reasonable and customary use in describing the
   origin of the Work and reproducing the content of the NOTICE file.

7. Disclaimer of Warranty. Unless required by applicable law or
   agreed to in writing, Licensor provides the Work (and each
   Contributor provides its Contributions) on an "AS IS" BASIS,
   WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or
   implied, including, without limitation, any warranties or conditions
   of TITLE, NON-INFRINGEMENT, MERCHANTABILITY, or FITNESS FOR A
   PARTICULAR PURPOSE. You are solely responsible for determining the
   appropriateness of using or redistributing the Work and assume any
   risks associated with Your exercise of permissions under this License.

8. Limitation of Liability. In no event and under no legal theory,
   whether in tort (including negligence), contract, or otherwise,
   unless required by applicable law (such as deliberate and grossly
   negligent acts) or agreed to in writing, shall any Contributor be
   liable to You for damages, including any direct, indirect, special,
   incidental, or consequential damages of any character arising as a
   result of this License or out of the use or inability to use the
   Work (including but not limited to damages for loss of goodwill,
   work stoppage, computer failure or malfunction, or any and all
   other commercial damages or losses), even if such Contributor
   has been advised of the possibility of such damages.

9. Accepting Warranty or Additional Liability. While redistributing
   the Work or Derivative Works thereof, You may choose to offer,
   and charge a fee for, acceptance of support, warranty, indemnity,
   or other liability obligations and/or rights consistent with this
   License. However, in accepting such obligations, You may act only
   on Your own behalf and on Your sole responsibility, not on behalf
   of any other Contributor, and only if You agree to indemnify,
   defend, and hold each Contributor harmless for any liability
   incurred by, or claims asserted against, such Contributor by reason
   of your accepting any such warranty or additional liability.

END OF TERMS AND CONDITIONS

```

---

### 5.4. SIL Open Font License, Version 1.1 (Texto Íntegro Oficial)
*Texto oficial completo obtenido del archivo fuente `OFL.txt` de `epaint_default_fonts`:*

```text
This Font Software is licensed under the SIL Open Font License,
Version 1.1.

This license is copied below, and is also available with a FAQ at:
http://scripts.sil.org/OFL

-----------------------------------------------------------
SIL OPEN FONT LICENSE Version 1.1 - 26 February 2007
-----------------------------------------------------------

PREAMBLE
The goals of the Open Font License (OFL) are to stimulate worldwide
development of collaborative font projects, to support the font
creation efforts of academic and linguistic communities, and to
provide a free and open framework in which fonts may be shared and
improved in partnership with others.

The OFL allows the licensed fonts to be used, studied, modified and
redistributed freely as long as they are not sold by themselves. The
fonts, including any derivative works, can be bundled, embedded,
redistributed and/or sold with any software provided that any reserved
names are not used by derivative works. The fonts and derivatives,
however, cannot be released under any other type of license. The
requirement for fonts to remain under this license does not apply to
any document created using the fonts or their derivatives.

DEFINITIONS
"Font Software" refers to the set of files released by the Copyright
Holder(s) under this license and clearly marked as such. This may
include source files, build scripts and documentation.

"Reserved Font Name" refers to any names specified as such after the
copyright statement(s).

"Original Version" refers to the collection of Font Software
components as distributed by the Copyright Holder(s).

"Modified Version" refers to any derivative made by adding to,
deleting, or substituting -- in part or in whole -- any of the
components of the Original Version, by changing formats or by porting
the Font Software to a new environment.

"Author" refers to any designer, engineer, programmer, technical
writer or other person who contributed to the Font Software.

PERMISSION & CONDITIONS
Permission is hereby granted, free of charge, to any person obtaining
a copy of the Font Software, to use, study, copy, merge, embed,
modify, redistribute, and sell modified and unmodified copies of the
Font Software, subject to the following conditions:

1) Neither the Font Software nor any of its individual components, in
Original or Modified Versions, may be sold by itself.

2) Original or Modified Versions of the Font Software may be bundled,
redistributed and/or sold with any software, provided that each copy
contains the above copyright notice and this license. These can be
included either as stand-alone text files, human-readable headers or
in the appropriate machine-readable metadata fields within text or
binary files as long as those fields can be easily viewed by the user.

3) No Modified Version of the Font Software may use the Reserved Font
Name(s) unless explicit written permission is granted by the
corresponding Copyright Holder. This restriction only applies to the
primary font name as presented to the users.

4) The name(s) of the Copyright Holder(s) or the Author(s) of the Font
Software shall not be used to promote, endorse or advertise any
Modified Version, except to acknowledge the contribution(s) of the
Copyright Holder(s) and the Author(s) or with their explicit written
permission.

5) The Font Software, modified or unmodified, in part or in whole,
must be distributed entirely under this license, and must not be
distributed under any other license. The requirement for fonts to
remain under this license does not apply to any document created using
the Font Software.

TERMINATION
This license becomes null and void if any of the above conditions are
not met.

DISCLAIMER
THE FONT SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND,
EXPRESS OR IMPLIED, INCLUDING BUT NOT LIMITED TO ANY WARRANTIES OF
MERCHANTABILITY, FITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT
OF COPYRIGHT, PATENT, TRADEMARK, OR OTHER RIGHT. IN NO EVENT SHALL THE
COPYRIGHT HOLDER BE LIABLE FOR ANY CLAIM, DAMAGES OR OTHER LIABILITY,
INCLUDING ANY GENERAL, SPECIAL, INDIRECT, INCIDENTAL, OR CONSEQUENTIAL
DAMAGES, WHETHER IN AN ACTION OF CONTRACT, TORT OR OTHERWISE, ARISING
FROM, OUT OF THE USE OR INABILITY TO USE THE FONT SOFTWARE OR FROM
OTHER DEALINGS IN THE FONT SOFTWARE.

```

---

### 5.5. Ubuntu Font Licence, Version 1.0 (Texto Íntegro Oficial)
*Texto oficial completo obtenido del archivo fuente `UFL.txt` de `epaint_default_fonts`:*

```text
-------------------------------
UBUNTU FONT LICENCE Version 1.0
-------------------------------

PREAMBLE
This licence allows the licensed fonts to be used, studied, modified and
redistributed freely. The fonts, including any derivative works, can be
bundled, embedded, and redistributed provided the terms of this licence
are met. The fonts and derivatives, however, cannot be released under
any other licence. The requirement for fonts to remain under this
licence does not require any document created using the fonts or their
derivatives to be published under this licence, as long as the primary
purpose of the document is not to be a vehicle for the distribution of
the fonts.

DEFINITIONS
"Font Software" refers to the set of files released by the Copyright
Holder(s) under this licence and clearly marked as such. This may
include source files, build scripts and documentation.

"Original Version" refers to the collection of Font Software components
as received under this licence.

"Modified Version" refers to any derivative made by adding to, deleting,
or substituting -- in part or in whole -- any of the components of the
Original Version, by changing formats or by porting the Font Software to
a new environment.

"Copyright Holder(s)" refers to all individuals and companies who have a
copyright ownership of the Font Software.

"Substantially Changed" refers to Modified Versions which can be easily
identified as dissimilar to the Font Software by users of the Font
Software comparing the Original Version with the Modified Version.

To "Propagate" a work means to do anything with it that, without
permission, would make you directly or secondarily liable for
infringement under applicable copyright law, except executing it on a
computer or modifying a private copy. Propagation includes copying,
distribution (with or without modification and with or without charging
a redistribution fee), making available to the public, and in some
countries other activities as well.

PERMISSION & CONDITIONS
This licence does not grant any rights under trademark law and all such
rights are reserved.

Permission is hereby granted, free of charge, to any person obtaining a
copy of the Font Software, to propagate the Font Software, subject to
the below conditions:

1) Each copy of the Font Software must contain the above copyright
notice and this licence. These can be included either as stand-alone
text files, human-readable headers or in the appropriate machine-
readable metadata fields within text or binary files as long as those
fields can be easily viewed by the user.

2) The font name complies with the following:
(a) The Original Version must retain its name, unmodified.
(b) Modified Versions which are Substantially Changed must be renamed to
avoid use of the name of the Original Version or similar names entirely.
(c) Modified Versions which are not Substantially Changed must be
renamed to both (i) retain the name of the Original Version and (ii) add
additional naming elements to distinguish the Modified Version from the
Original Version. The name of such Modified Versions must be the name of
the Original Version, with "derivative X" where X represents the name of
the new work, appended to that name.

3) The name(s) of the Copyright Holder(s) and any contributor to the
Font Software shall not be used to promote, endorse or advertise any
Modified Version, except (i) as required by this licence, (ii) to
acknowledge the contribution(s) of the Copyright Holder(s) or (iii) with
their explicit written permission.

4) The Font Software, modified or unmodified, in part or in whole, must
be distributed entirely under this licence, and must not be distributed
under any other licence. The requirement for fonts to remain under this
licence does not affect any document created using the Font Software,
except any version of the Font Software extracted from a document
created using the Font Software may only be distributed under this
licence.

TERMINATION
This licence becomes null and void if any of the above conditions are
not met.

DISCLAIMER
THE FONT SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND,
EXPRESS OR IMPLIED, INCLUDING BUT NOT LIMITED TO ANY WARRANTIES OF
MERCHANTABILITY, FITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT OF
COPYRIGHT, PATENT, TRADEMARK, OR OTHER RIGHT. IN NO EVENT SHALL THE
COPYRIGHT HOLDER BE LIABLE FOR ANY CLAIM, DAMAGES OR OTHER LIABILITY,
INCLUDING ANY GENERAL, SPECIAL, INDIRECT, INCIDENTAL, OR CONSEQUENTIAL
DAMAGES, WHETHER IN AN ACTION OF CONTRACT, TORT OR OTHERWISE, ARISING
FROM, OUT OF THE USE OR INABILITY TO USE THE FONT SOFTWARE OR FROM OTHER
DEALINGS IN THE FONT SOFTWARE.

```

---

### 5.6. ISC License (Texto Íntegro Oficial)
*Aplicable a `nih_plug`, `nih_plug_egui`, `nih_plug_derive`:*

```text
ISC License

Copyright (c) 2022-2024 Robbert van der Helm

Permission to use, copy, modify, and/or distribute this software for any
purpose with or without fee is hereby granted, provided that the above
copyright notice and this permission notice appear in all copies.

THE SOFTWARE IS PROVIDED "AS IS" AND THE AUTHOR DISCLAIMS ALL WARRANTIES WITH
REGARD TO THIS SOFTWARE INCLUDING ALL IMPLIED WARRANTIES OF MERCHANTABILITY
AND FITNESS. IN NO EVENT SHALL THE AUTHOR BE LIABLE FOR ANY SPECIAL, DIRECT,
INDIRECT, OR CONSEQUENTIAL DAMAGES OR ANY DAMAGES WHATSOEVER RESULTING FROM
LOSS OF USE, DATA OR PROFITS, WHETHER IN AN ACTION OF CONTRACT, NEGLIGENCE OR
OTHER TORTIOUS ACTION, ARISING OUT OF OR IN CONNECTION WITH THE USE OR
PERFORMANCE OF THIS SOFTWARE.
```

---

### 5.7. BSD 1-Clause License (Texto Íntegro Oficial)
*Aplicable a `assert_no_alloc`:*

```text
Copyright (c) 2020 Florian Jung <flo@windfis.ch>

Redistribution and use in source and binary forms, with or without
modification, are permitted provided that the following conditions are met:

1. Redistributions of source code must retain the above copyright notice, this
   list of conditions and the following disclaimer.

THIS SOFTWARE IS PROVIDED BY THE COPYRIGHT HOLDERS AND CONTRIBUTORS "AS IS"
AND ANY EXPRESS OR IMPLIED WARRANTIES, INCLUDING, BUT NOT LIMITED TO, THE
IMPLIED WARRANTIES OF MERCHANTABILITY AND FITNESS FOR A PARTICULAR PURPOSE ARE
DISCLAIMED. IN NO EVENT SHALL THE COPYRIGHT HOLDER OR CONTRIBUTORS BE LIABLE
FOR ANY DIRECT, INDIRECT, INCIDENTAL, SPECIAL, EXEMPLARY, OR CONSEQUENTIAL
DAMAGES (INCLUDING, BUT NOT LIMITED TO, PROCUREMENT OF SUBSTITUTE GOODS OR
SERVICES; LOSS OF USE, DATA OR PROFITS; OR BUSINESS INTERRUPTION) HOWEVER
CAUSED AND ON ANY THEORY OF LIABILITY, WHETHER IN CONTRACT, STRICT LIABILITY,
OR TORT INCLUDING NEGLIGENCE OR OTHERWISE ARISING IN ANY WAY OUT OF THE USE
OF THIS SOFTWARE, EVEN IF ADVISED OF THE POSSIBILITY OF SUCH DAMAGE.
```

---

### 5.8. BSD 2-Clause License (Texto Íntegro Oficial)
*Aplicable a `zerocopy` y crates asociados:*

```text
Redistribution and use in source and binary forms, with or without
modification, are permitted provided that the following conditions are met:

1. Redistributions of source code must retain the above copyright notice, this
   list of conditions and the following disclaimer.

2. Redistributions in binary form must reproduce the above copyright notice,
   this list of conditions and the following disclaimer in the documentation
   and/or other materials provided with the distribution.

THIS SOFTWARE IS PROVIDED BY THE COPYRIGHT HOLDERS AND CONTRIBUTORS "AS IS"
AND ANY EXPRESS OR IMPLIED WARRANTIES, INCLUDING, BUT NOT LIMITED TO, THE
IMPLIED WARRANTIES OF MERCHANTABILITY AND FITNESS FOR A PARTICULAR PURPOSE ARE
DISCLAIMED. IN NO EVENT SHALL THE COPYRIGHT HOLDER OR CONTRIBUTORS BE LIABLE
FOR ANY DIRECT, INDIRECT, INCIDENTAL, SPECIAL, EXEMPLARY, OR CONSEQUENTIAL
DAMAGES (INCLUDING, BUT NOT LIMITED TO, PROCUREMENT OF SUBSTITUTE GOODS OR
SERVICES; LOSS OF USE, DATA OR PROFITS; OR BUSINESS INTERRUPTION) HOWEVER
CAUSED AND ON ANY THEORY OF LIABILITY, WHETHER IN CONTRACT, STRICT LIABILITY,
OR TORT INCLUDING NEGLIGENCE OR OTHERWISE ARISING IN ANY WAY OUT OF THE USE
OF THIS SOFTWARE, EVEN IF ADVISED OF THE POSSIBILITY OF SUCH DAMAGE.
```

---

### 5.9. Zlib License (Texto Íntegro Oficial)
*Aplicable a `slotmap` y dependencias tri-licenciadas:*

```text
This software is provided 'as-is', without any express or implied
warranty. In no event will the authors be held liable for any damages
arising from the use of this software.

Permission is granted to anyone to use this software for any purpose,
including commercial applications, and to alter it and redistribute it
freely, subject to the following restrictions:

1. The origin of this software must not be misrepresented; you must not
   claim that you wrote the original software. If you use this software
   in a product, an acknowledgment in the product documentation would be
   appreciated but is not required.
2. Altered source versions must be plainly marked as such, and must not be
   misrepresented as being the original software.
3. This notice may not be removed or altered from any source distribution.
```


---

### 5.10. Bitstream Vera License (Íntegro Oficial para Fuente Hack / Bitstream Vera)
*Aviso y texto oficial de licencia requerido para la tipografía Hack (`Hack-Regular.ttf` / `Hack-Regular.txt` embebida a través de `epaint_default_fonts`):*

```text
BITSTREAM VERA LICENSE

Copyright (c) 2003 by Bitstream, Inc. All Rights Reserved. Bitstream Vera is a
trademark of Bitstream, Inc.

Permission is hereby granted, free of charge, to any person obtaining a copy of
the fonts accompanying this license ("Fonts") and associated documentation files
(the "Font Software"), to reproduce and distribute the Font Software, including
without limitation the rights to use, copy, merge, publish, distribute, and/or
sell copies of the Font Software, and to permit persons to whom the Font Software
is furnished to do so, subject to the following conditions:

The above copyright and trademark notices and this permission notice shall be
included in all copies of one or more of the Font Software typefaces.

The Font Software may be modified, altered, or added to, and in particular the
designs of glyphs or characters in the Fonts may be modified and additional
glyphs or characters may be added to the Fonts, only if the fonts are renamed to
names not containing either the words "Bitstream" or the word "Vera".

This License becomes null and void to the extent applicable to Fonts or Font
Software that has been modified and is distributed under the "Bitstream Vera"
names.

The Font Software may be sold as part of a larger software package but no copy
of one or more of the Font Software typefaces may be sold by itself.

THE FONT SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND, EXPRESS OR
IMPLIED, INCLUDING BUT NOT LIMITED TO ANY WARRANTIES OF MERCHANTABILITY, FITNESS
FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT OF COPYRIGHT, PATENT, TRADEMARK, OR
OTHER RIGHT. IN NO EVENT SHALL BITSTREAM OR THE GNOME FOUNDATION BE LIABLE FOR
ANY CLAIM, DAMAGES OR OTHER LIABILITY, INCLUDING ANY GENERAL, SPECIAL, INDIRECT,
INCIDENTAL, OR CONSEQUENTIAL DAMAGES, WHETHER IN AN ACTION OF CONTRACT, TORT OR
OTHERWISE, ARISING FROM, OUT OF THE USE OR INABILITY TO USE THE FONT SOFTWARE OR
FROM OTHER DEALINGS IN THE FONT SOFTWARE.

Except as contained in this notice, the names of Gnome, the Gnome Foundation, and
Bitstream Inc., shall not be used in advertising or otherwise to promote the
sale, use or other dealings in this Font Software without prior written
authorization from the Gnome Foundation or Bitstream Inc., respectively. For
further information, contact: fonts at gnome dot org.
```
