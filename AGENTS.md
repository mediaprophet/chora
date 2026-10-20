# Chora Engine Library — Multi-Agent Collaboration Rules

`chora` (`c:\github\chora`) is the embeddable Rust web and spatial document engine library for `webizen-desktop` and the `qualiaDB` ecosystem.

---

## 0. Immovable Rules (All Agents Must Honour These)

| Rule | Detail |
|------|--------|
| **Library-First (`chora`, never `servo` branding)** | This engine is **Chora**, not Servo. All public library types (`Chora`, `ChoraBuilder`, `ChoraDelegate`, `ChoraError`, `ChoraUrl`), C-API exports (`chora_*` in `ffi/capi`), User-Agent strings, privileged DOM interfaces (`navigator.chora`), and package descriptions must be branded **Chora**. Type aliases may be retained strictly where required for internal transitional compatibility, but no external or user-facing surface may be branded "Servo". |
| **MPL-2.0 File-Level Boundary** | Every source file inside `c:\github\chora` is governed by the Mozilla Public License 2.0 (`LICENSE`). Preserve the MPL-2.0 header on all files inside `c:\github\chora`. **Never** add any dependency on `qualia-*` or `webizen-*` crates inside `c:\github\chora`, and never copy code from `qualiaDB` or `webizen-desktop` into `c:\github\chora`. All host integration happens through generic traits (`EmbedderCertVerifier`, `EmbedderKeystoreProvider`, `ProtocolHandler`, `WebViewDelegate`) and the C-ABI (`ffi/capi`). |
| **Human-Rights, Fiduciary & Guardianship Vocabulary** | **Never** use the terms *"sovereign"* or *"self-sovereign"* anywhere in code, comments, commit messages, crate descriptions, or documentation. Infants, children, and dependent or vulnerable persons are not "self-sovereign"; they hold inalienable human rights protected through fiduciary duty, guardianship, human agency, and international human-rights instruments (UDHR, CRC, CRPD). Use **human-rights, fiduciary, guardianship, and human-agency** terminology exclusively. |
| **License-Clean Embedded Feature Profile** | Embedded library builds (`embedded_library` feature in `components/servo/Cargo.toml`) must not enable `media-gstreamer` (avoiding LGPL/GPL GStreamer plugins) and must elect the BSD-style FreeType License (FTL) when `bundled_freetype` is used. |
| **Post-Quantum & Custom Root PKI First** | TLS 1.3 connections prioritize post-quantum hybrid key exchange (`X25519MLKEM768`). Root certificate verification supports embedder-supplied root stores (`RootTrustPolicy::EmbedderRootsOnly`, `EmbedderAndWebPki`, `EmbedderAndPlatform`), live root reload, mutual TLS (`ResolvesClientCert`), and `EmbedderCertVerifier` for Australian (AU) / community root CAs and post-quantum (`ML-DSA`) X.509 certificates. WebCrypto supports delegating non-extractable key operations (`ML-DSA`, `ML-KEM`, `X-Wing`, `Ed25519`, `X25519`) to the host via `EmbedderKeystoreProvider`. |
