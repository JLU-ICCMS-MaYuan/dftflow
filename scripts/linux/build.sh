#!/usr/bin/env bash
# macOS Docker/OrbStack 也可运行：构建并验证真正的 Linux x86_64 musl 产物。
set -euo pipefail
repo_dir="$(cd "$(dirname "$0")/../.." && pwd)"
cd "$repo_dir"
mkdir -p scripts/linux/.sources
if [[ ! -f scripts/linux/.sources/OpenBLAS-0.3.30.tar.gz ]]; then
    curl -fL --retry 3 https://github.com/OpenMathLib/OpenBLAS/releases/download/v0.3.30/OpenBLAS-0.3.30.tar.gz -o scripts/linux/.sources/OpenBLAS-0.3.30.tar.gz.tmp
    mv scripts/linux/.sources/OpenBLAS-0.3.30.tar.gz{.tmp,}
fi
if [[ ! -f scripts/linux/.sources/spglib-2.6.0.tar.gz ]]; then
    curl -fL --retry 3 https://codeload.github.com/spglib/spglib/tar.gz/refs/tags/v2.6.0 -o scripts/linux/.sources/spglib-2.6.0.tar.gz.tmp
    mv scripts/linux/.sources/spglib-2.6.0.tar.gz{.tmp,}
fi
docker build --platform linux/amd64 -t dftflow-musl-builder:0004 -f scripts/linux/Dockerfile scripts/linux
docker run --rm --platform linux/amd64 \
    -v "$repo_dir:/src" -v "$HOME/.cargo/registry:/usr/local/cargo/registry" \
    -v dftflow-0004-target:/build \
    -e RUSTC_BOOTSTRAP=1 -e CARGO_TARGET_DIR=/build -e PKG_CONFIG_PATH=/usr/lib/pkgconfig:/usr/share/pkgconfig \
    dftflow-musl-builder:0004 bash -ec '
      cargo -Zhost-config -Ztarget-applies-to-host --config .cargo/linux-host.toml test --release --workspace --locked --target x86_64-unknown-linux-musl -j4
      cargo -Zhost-config -Ztarget-applies-to-host --config .cargo/linux-host.toml build --workspace --locked --release --target x86_64-unknown-linux-musl -j4
      mkdir -p dist/linux-x86_64-musl/bin
      for name in qeflow vaspflow scflow wannier90 substitution epw_phonon epw_wannier epw_superconductivity; do
        cp /build/x86_64-unknown-linux-musl/release/$name dist/linux-x86_64-musl/bin/
        strip dist/linux-x86_64-musl/bin/$name
      done
      bash scripts/linux/verify-static.sh dist/linux-x86_64-musl/bin
      cp scripts/install-compat.sh dist/linux-x86_64-musl/
      { rustc --version; uname -sm; pkg-config --modversion openblas; } > dist/linux-x86_64-musl/BUILD-INFO.txt
      cd dist/linux-x86_64-musl
      sha256sum bin/* install-compat.sh BUILD-INFO.txt > SHA256SUMS
    '
# 断网重跑已经编译的全部测试，验证运行不需要网络。
docker run --rm --network none --platform linux/amd64 \
    -v "$repo_dir:/src" -v "$HOME/.cargo/registry:/usr/local/cargo/registry" \
    -v dftflow-0004-target:/build \
    -e RUSTC_BOOTSTRAP=1 -e CARGO_TARGET_DIR=/build -e PKG_CONFIG_PATH=/usr/lib/pkgconfig:/usr/share/pkgconfig \
    dftflow-musl-builder:0004 bash -ec '
      cargo -Zhost-config -Ztarget-applies-to-host --config .cargo/linux-host.toml test --release --workspace --locked --offline --target x86_64-unknown-linux-musl -j4
      bash scripts/verify-entries.sh /src/dist/linux-x86_64-musl/bin
      smoke_dir="$(mktemp -d)"
      cp -r scflow/tests/fixtures/scflow_parity/work "$smoke_dir/"
      cp scflow/tests/fixtures/scflow_parity/input.toml "$smoke_dir/"
      cd "$smoke_dir"
      /src/dist/linux-x86_64-musl/bin/scflow -c input.toml > scflow.log
      cmp results/summary_MgB2_s3_mu0,1.csv /src/scflow/tests/fixtures/scflow_parity/golden/summary_MgB2_s3_mu0,1.csv
      rm -rf "$smoke_dir"
    '
tar -czf dist/dftflow-linux-x86_64-musl.tar.gz -C dist linux-x86_64-musl
