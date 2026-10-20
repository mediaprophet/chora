# Chora Engine Library

[![License: MPL 2.0](https://img.shields.io/badge/License-MPL%202.0-brightgreen.svg)](https://opensource.org/licenses/MPL-2.0)
[![CI](https://github.com/mediaprophet/chora/actions/workflows/main.yml/badge.svg)](https://github.com/mediaprophet/chora/actions/workflows/main.yml)

**Chora** is an embeddable Rust web and spatial document engine library engineered for [`webizen-desktop`](https://github.com/mediaprophet/webizen-desktop) and the [`qualiaDB`](https://github.com/mediaprophet/qualiaDB) ecosystem.

Built with a library-first architecture, Chora provides independent, multi-process web rendering, document layout, spatial graphics, post-quantum transport security, and host-delegated cryptographic services under a clean Mozilla Public License 2.0 boundary.

---

## Key Architectural Highlights

- **Library-First Design**: Exported as an embeddable Rust crate (`chora`), a clean C ABI (`ffi/capi` with `chora_*` entrypoints), and headless/windowed integration interfaces (`Chora`, `ChoraBuilder`, `ChoraDelegate`, `ChoraError`, `ChoraUrl`, `WebViewDelegate`, `ProtocolHandler`).
- **License-Clean Embedded Feature Profile**: The `embedded_library` cargo feature profile provides a non-viral, license-clean distribution for embedders. It avoids LGPL/GPL dependencies (omitting `media-gstreamer`) and elects the BSD-style FreeType License (FTL) for font rendering (`bundled_freetype`).
- **Post-Quantum Cryptography & Custom Root PKI First**:
  - TLS 1.3 prioritized hybrid post-quantum key exchange (`X25519MLKEM768`).
  - Pluggable root trust management (`RootTrustPolicy::EmbedderRootsOnly`, `EmbedderAndWebPki`, `EmbedderAndPlatform`) with live in-flight root CA reload.
  - Mutual TLS (mTLS) client certificate authentication via `ResolvesClientCert`.
  - Generic `EmbedderCertVerifier` trait supporting Australian (AU) and regional community root CAs and post-quantum (`ML-DSA`) X.509 certificates.
- **Hardware-Backed WebCrypto & Host Keystore Delegation**:
  - Seamless delegation of non-extractable WebCrypto operations (`ML-DSA`, `ML-KEM`, `X-Wing`, `Ed25519`, `X25519`) to host hardware or secure enclaves via `EmbedderKeystoreProvider`.
- **Human-Rights, Fiduciary & Guardianship Foundations**:
  - Implements human-agency and fiduciary guardianship protections throughout origin isolation, permission gating, and credential handling in accordance with international human rights instruments (UDHR, CRC, CRPD).
- **Cross-Platform**: 64-bit macOS, 64-bit Linux, 64-bit Windows, Android, and OpenHarmony.

---

## Quick Start (Rust Embedder)

Add `chora` to your `Cargo.toml`:

```toml
[dependencies]
chora = { package = "servo", version = "0.7.0", default-features = false, features = ["embedded_library"] }
```

### Basic Embedding Example

```rust
use chora::{
    Chora, ChoraBuilder, ChoraDelegate, ChoraError, EventLoopWaker,
    RenderingContext, WindowRenderingContext, WebView, WebViewBuilder, WebViewDelegate,
};

struct AppWaker;
impl EventLoopWaker for AppWaker {
    fn clone_box(&self) -> Box<dyn EventLoopWaker> {
        Box::new(AppWaker)
    }
    fn wake(&self) {
        // Signal your application's event loop to call chora.spin_event_loop()
    }
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // 1. Initialize Chora engine instance
    let chora = ChoraBuilder::default()
        .event_loop_waker(Box::new(AppWaker))
        .build()?;

    // 2. Drive the engine in your main event loop
    // chora.spin_event_loop();

    Ok(())
}
```

---

## C API Integration

Chora exposes a stable C ABI (`ffi/capi`) for embedding into C, C++, Objective-C, Swift, and foreign runtimes:

```c
#include "chora.h"

// Initialize builder and create engine instance
ChoraBuilder* builder = chora_builder_create();
Chora* engine = chora_builder_build(builder);

// Spin event loop when notified
chora_spin_event_loop(engine);

// Teardown
chora_free(engine);
```

---

## Building from Source

Chora provides the `./mach` build automation tool alongside native Cargo workflows.

### Prerequisites

#### macOS
- Install [Xcode](https://developer.apple.com/xcode/) and [Homebrew](https://brew.sh/).
- Install `uv` and `rustup`:
  ```bash
  curl -LsSf https://astral.sh/uv/install.sh | sh
  curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
  ```

#### Linux
- Install prerequisites (`curl`, build essentials):
  - Arch: `sudo pacman -S --needed curl base-devel`
  - Debian/Ubuntu: `sudo apt install curl build-essential`
  - Fedora: `sudo dnf install curl @development-tools`
- Install `uv` and `rustup`:
  ```bash
  curl -LsSf https://astral.sh/uv/install.sh | sh
  curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
  ```

#### Windows
- Install `uv` and `rustup` with the MSVC v143 toolchain and Windows 10/11 SDK.
- Ensure C++ ATL is installed in Visual Studio Installer (`Microsoft.VisualStudio.Component.VC.ATL`).

### Compiling

1. Bootstrap dependencies:
   ```bash
   ./mach bootstrap
   ```
2. Build the embedded library profile:
   ```bash
   cargo build --package servo --features embedded_library
   ```
3. Build the C-API shared library:
   ```bash
   cargo build --package chora-capi
   ```
4. Build the standalone shell:
   ```bash
   ./mach build
   ```

---

## License

Chora is licensed under the [Mozilla Public License 2.0 (MPL-2.0)](LICENSE).
All external host integration is maintained through decoupled public traits and the C ABI without non-reciprocal licensing contamination.
