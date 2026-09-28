<p align="center"><a href="../README.md">MDeck</a> &middot; <a href="tutorial.md">Tutorial</a> &middot; <a href="install.md">Install</a> &middot; <a href="writing-slides.md">Writing slides</a> &middot; <a href="visualizations.md">Visualizations</a> &middot; <a href="themes.md">Themes</a> &middot; <a href="engines.md">Engines</a> &middot; <a href="presenting.md">Presenting</a> &middot; <a href="export.md">Export</a> &middot; <a href="ai.md">AI</a> &middot; <a href="commands.md">Commands</a></p>

# Install

MDeck is a single binary for macOS, Linux and Windows.

```bash
brew install mklab-se/tap/mdeck      # macOS / Linux
cargo install mdeck                  # anywhere with Rust 1.95+
cargo binstall mdeck                 # pre-built binary via cargo-binstall
```

Or download a binary for macOS (Intel and Apple Silicon), Linux, or Windows
from [GitHub Releases](https://github.com/mklab-se/mdeck/releases).

`cargo install` builds from source; on Windows that needs [NASM](https://www.nasm.us/) and
[CMake](https://cmake.org/) on `PATH` (plus the Visual Studio Build Tools most Rust installs already
have) to compile [`aws-lc-rs`](https://github.com/aws/aws-lc-rs), the TLS crypto backend used
transitively via `ailloy`. macOS and Linux need nothing extra, and `brew install` / `cargo binstall`
skip this entirely by using a pre-built binary.

Every engine is a cargo feature, all on by default. To build a smaller binary with only the
engines you use, name them (the plain engine is always there):

```bash
cargo install mdeck --no-default-features --features particles,splitflap
```

<details>
<summary>Software bill of materials (SBOM)</summary>

Every release archive has a matching CycloneDX 1.5 SBOM listing the exact crate versions
compiled into that platform's binary:

```
mdeck-vX.Y.Z-<target>.cdx.json
```

The binaries are also built with [`cargo auditable`](https://github.com/rust-secure-code/cargo-auditable),
so the dependency list travels inside the executable itself. Check a downloaded binary against the
RustSec advisory database with:

```sh
cargo install cargo-audit --features=fix
cargo audit bin ./mdeck
```

`syft` and `trivy` also understand this format.

</details>

Check it works:

```bash
mdeck version
mdeck samples/showcase/launch.md   # from a clone of this repository
```
