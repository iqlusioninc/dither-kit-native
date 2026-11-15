# Native port execution

Source: `PLAN.md` and the supplied archive, restored at baseline `abaabe4`.
Implementation branch: `implement-native-ports`.

## Decisions

- This restored checkout is isolated from the source archive; implement here on a feature branch.
- Run independent language/platform implementations in parallel; each owns a separate directory. Shared fixture and Rust reference files remain the source of truth.
- The build host is macOS arm64 with GTK 4.24.1, Swift 6.4.2, and .NET SDK 11. Linux Mesa/EGL and Windows Direct2D require their native CI runners.
- Replace obsolete Rust `epoxy = 0.1` (depends on yanked `xml-rs`) with `libloading` of libepoxy dispatch exports, retaining libepoxy symbol loading.
- ComputeSharp 3.2 requires Windows SDK 22621; use that compile-time TFM while retaining minimum runtime Windows 10 build 19041.
- Strengthen `dither-golden --check`: check stored JSON programs and decoded PNG bytes as well as freshly generated VM/upstream parity. A test demonstrated the original command accepted changed fixture programs.

## Baseline validation

- `cargo test --workspace`: 2 reference VM tests passed (GTK was a stub).
- `cargo run -p dither-golden -- ../fixtures --check`: 39 generated scenes matched upstream exactly.

## Work streams

- GTK renderer, components, gallery and native GPU tests: implemented. Local Rust suite passes; all 39 GLSL fixtures pass on Apple OpenGL (maximum channel delta 1). Linux EGL/Mesa remains for CI.
- Pure Swift encoders and fixture tests: implemented; all 39 independent encodings match. Nine encoder/helper tests pass locally.
- SwiftUI views, Metal, gallery and GPU tests: implemented. Independent full-package run passes nine IR tests plus three UI tests, including all 39 strict logical Metal goldens and all 39 production compositing cases. Maximum observed Metal channel drift is 1.
- Pure C# encoders and fixture tests: implemented; independent Release run passes all 74 tests, including all 39 fixture encodings. Local host uses .NET 11 roll-forward; CI uses .NET 8.
- WinUI controls, Direct2D shader, gallery and GPU tests: implemented. C# static compilation passed with native stages bypassed; Windows HLSL, packaging and pixel execution require CI.
- CI, README and fixture-integrity regression: implemented. Changed/missing artifact regression passes; scoped code review approved.

## Publishing

User authorized a new public repository under `iqlusioninc`.
Created: https://github.com/iqlusioninc/dither-kit-native (no code pushed yet).

## Review and pixel transport

Review caught and fixed initially hidden series that could not be revealed, chart axes that did not use actual geometry, GTK widget callback ownership cycles, and WinUI legend/hover/button behavior. Encoder edge regressions cover Rust formatting/saturating casts and NaN reductions. Native galleries were visually inspected; the GTK screenshot is in `docs/gallery-gtk.png`.

SwiftUI normally transports premultiplied layer colors. Converting low-alpha RGBA8 samples back to straight RGB amplifies quantization. The logical Metal golden test therefore uses a straight-RGBA transport switch in both passes, matching the plan's test-output exception. It still runs the identical interpreter, actual layer sampling, bloom and overlays; it does not recompute the base in the post pass or modify fixtures. Production uses premultiplied transport and has a separate compositing-byte test. The strict logical gate remains maximum channel delta 2 and at most 1% nonidentical pixels.
