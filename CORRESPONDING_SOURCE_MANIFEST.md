# CStrip2 Visual — Corresponding Source Code & Build Manifest

**Producto:** CStrip2 Visual  
**Versión:** 0.1.0  
**Licencia Principal:** GNU General Public License v3.0 or later (GPL-3.0-or-later)  
**Estándar Aplicable:** GNU GPLv3 Sección 1 y Sección 6 ("Corresponding Source")  
**Estado del Documento:** Candidato para Revisión Técnica y Jurídica (Edición con Vendoring Completo)  
**Fecha de Auditoría:** 12 de octubre de 2026  

---

## 1. Alcance Legal del Código Fuente Correspondiente ("Corresponding Source")

De conformidad con la **Sección 1 de la GNU General Public License v3.0**:

> *"The “Corresponding Source” for a work in object code form means all the source code needed to generate, install, and (for an executable work) run the object code, and to modify the work, including scripts to control those activities. However, it does not include the work's System Libraries, or general-purpose tools or generally available free programs which are used unmodified in performing those activities but which are not part of the work."*

La distribución de los binarios ejecutables de **CStrip2 Visual** (`cstrip2.vst3`, `cstrip2.clap` y en macOS `CStrip2.component`) requiere poner a disposición de los receptores el conjunto completo de código fuente, manifiestos, scripts de control y dependencias archivadas que permitan compilar versiones funcionalmente equivalentes del software.

> [!WARNING]
> **Alcance de la Reproducibilidad y Estado Legal:**  
> Este manifiesto incluye los elementos preparados para compilar el software a partir de las fuentes empaquetadas. La compilación se ha verificado en Windows; la compilación desde este paquete en macOS todavía no se ha comprobado.  
> No se formula una afirmación de reproducibilidad binaria determinista e idéntica bit a bit sin una previa fijación estricta de variables de entorno de bajo nivel (marcas de tiempo de enlace en PE/Mach-O, rutas absolutas de depuración, versiones de LLVM/rustc y banderas de optimización de CPU).  
> Este documento se emite con fines de información técnica y no constituye asesoramiento legal.

---

## 2. Inventario Completo del Paquete de Publicación

El paquete se distribuye de forma autónoma con vendoring completo para posibilitar la compilación sin acceso a red:

```text
CStrip2_Visual_Source_v0.1.0/
├── Cargo.toml                       # Manifiesto raíz del workspace (members: cstrip2, xtask)
├── Cargo.lock                       # Bloqueo determinista de 278 paquetes (SHA-256 verificado)
├── .cargo/
│   └── config.toml                  # Alias xtask y configuración offline hacia vendor/
├── .gitignore                       # Filtro de exclusión de artefactos de compilación
├── README.md                        # Documentación técnica, descripción DSP y guía de compilación
├── LICENSE.txt                      # Texto íntegro oficial de la GNU GPLv3
├── THIRD_PARTY_NOTICES.md           # Licencias íntegras de dependencias y avisos de copyright
├── CORRESPONDING_SOURCE_MANIFEST.md  # Manifiesto técnico del Código Fuente Correspondiente
├── AIRWINDOWS_NOTICE.txt            # Atribución y licencia MIT oficial de Chris Johnson
├── CLAP_NOTICE.txt                  # Copyrights oficiales del estándar CLAP y wrappers
├── STEINBERG_VST3_NOTICE.txt        # Marca VST3 y delimitación MIT SDK vs. GPLv3 vst3-sys
├── APPLE_AUDIOMAC_NOTICE.txt        # Aviso de licencia Apache-2.0 de AudioUnitSDK
├── MAC_INSTALL.txt                  # Guía de instalación y Gatekeeper para macOS
├── cmake/
│   └── auv2/
│       └── CMakeLists.txt           # Configuración CMake del componente AUv2 para macOS
├── .github/
│   └── workflows/
│       └── build-macos.yml          # Pipeline CI/CD para compilación Universal en macOS
├── crates/
│   ├── cstrip2/                     # Crate principal del plugin de audio
│   │   ├── Cargo.toml               # Dependencias del plugin y metadata (GPL-3.0-or-later)
│   │   └── src/
│   │       ├── lib.rs               # Puntos de entrada nih_export_clap! y nih_export_vst3!
│   │       ├── params.rs            # Árbol de parámetros DAW
│   │       ├── presets.rs           # Preajustes de fábrica
│   │       ├── analyzer/            # Motor FFT estéreo lock-free
│   │       ├── dsp/                 # DSP CStrip2 (Capacitor, ButterComp, EQ, Output)
│   │       └── gui/                 # Interfaz egui, widgets y suite f05
│   └── xtask/                       # Herramienta de empaquetado del workspace
│       ├── Cargo.toml
│       └── src/main.rs
├── vendor/                          # Copia archivada local de 278 dependencias (crates.io y Git)
├── clap-wrapper/                    # Fuentes de clap-wrapper tag v0.16.0 (commit 1cca996e)
├── clap/                            # Cabeceras oficiales de CLAP SDK 1.2.3 (commit 309d1dae)
└── AudioUnitSDK/                    # Fuentes de Apple AudioUnitSDK 1.1.0 (commit 53a9a200)
```

---

## 3. Conciliación de Dependencias del Workspace

1. **278 Paquetes en `Cargo.lock`:**  
   El archivo `Cargo.lock` incorpora 278 bloques `[[package]]`. Su suma SHA-256 (`b1c7746a099e48dffbdcebf20009e97db9a4016e0479a44d3ba7888a6613285e`) es constante antes y después de cualquier operación `--locked`.
2. **278 Paquetes en `cargo metadata --locked --offline`:**  
   La resolución de metadatos en modo estrictamente desconectado confirma 278 entradas en `packages` y en `resolve.nodes`, sin discrepancias.
3. **Desglose Funcional:**  
   * **264 paquetes:** Pertenecen al grafo transitivo de `crates/cstrip2`.
   * **14 paquetes:** Pertenecen exclusivamente a la herramienta interna `crates/xtask`.
4. **Vendoring Completo:**  
   Todas las dependencias de `crates.io` y los repositorios Git (`baseview`, `egui-baseview`, `clap-sys`, `nih-plug`, `reflink`, `rust-assert-no-alloc`, `vst3-sys`) están archivados dentro de `vendor/`.

---

## 4. Estado de Verificación y Limitaciones por Entorno

> [!IMPORTANT]
> **Registro Expreso de Entornos y Formatos:**
>
> 1. **Windows x86_64 (Entorno Local Verificado):**
>    - `cargo metadata --locked --offline --format-version 1`: **Éxito.** Resolución de las 278 dependencias desde `vendor/` sin conexión a red.
>    - `cargo test -p cstrip2 --locked --offline`: **Éxito.** Los 85 tests de unidad e integración pasan limpiamente (DSP, análisis espectral, GUI y suite f05).
>    - `cargo xtask bundle cstrip2 --release --offline`: **Éxito.** Generación de bundles `target/bundled/cstrip2.clap` y `target/bundled/cstrip2.vst3`.
>
> 2. **macOS Universal 2 (arm64 + x86_64) y AUv2 (No Verificable en este Host):**
>    - No es técnicamente posible ejecutar el compilador Apple Clang, `lipo`, `codesign` ni el comando `auval` en este host Windows.
>    - La compilación en macOS se soporta mediante los scripts de integración continua provistos en `.github/workflows/build-macos.yml`, `cmake/auv2/CMakeLists.txt` y las fuentes locales de `clap-wrapper`, `AudioUnitSDK` y `clap` incorporadas.
>
> 3. **Linux:** No probado ni certificado en este entorno.

---

## 5. Provisión y Distribución del Código Fuente de Terceros (GPLv3 Sección 6)

Para dar estricto cumplimiento a la **Sección 6 de la GNU GPLv3**:

1. **Inclusión Física Completa (Modalidad Autónoma):**  
   Este paquete incluye físicamente el código fuente de las 278 dependencias Rust (en `vendor/`) y las dependencias C/C++ requeridas para el wrapper macOS AUv2 (`clap-wrapper/`, `AudioUnitSDK/` y `clap/`). No se requiere descarga de fuentes desde servidores de terceros para compilar.
2. **Descarga de Red Equivalente (GPLv3 Sección 6(d)):**  
   El archivo ZIP de este paquete debe ofrecerse al público en el mismo servidor y lugar de descarga que los binarios ejecutables distribuidos, sin cargo adicional.
3. **Distribuciones en soporte físico:**  
   Si se distribuye el software por medios distintos de una descarga web, deberán revisarse separadamente las modalidades de entrega del Código Fuente Correspondiente previstas en la Sección 6 de la GPLv3. Este documento no constituye por sí mismo una oferta contractual por escrito.
