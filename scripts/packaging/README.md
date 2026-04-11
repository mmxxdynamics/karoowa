# Packaging Notes

## Homebrew

The Homebrew formula is at `scripts/homebrew/karoowa.rb`. To publish:

1. Create a `karoowa/homebrew-tap` GitHub repository
2. Copy `karoowa.rb` into the repo root
3. Update SHA256 hashes from `checksums-sha256.txt` in each release
4. Users install with: `brew install karoowa/tap/karoowa`

## Debian / Ubuntu (.deb)

Use `cargo-deb` to build `.deb` packages:

```bash
cargo install cargo-deb
cargo deb -p karoowa
```

Add a `[package.metadata.deb]` section to `core/karoowa/Cargo.toml` when ready.

## RPM (Fedora / RHEL)

Use `cargo-rpm` to build `.rpm` packages:

```bash
cargo install cargo-rpm
cargo rpm build -p karoowa
```

## Windows (Chocolatey / Scoop)

Lower priority for M1. Prebuilt Windows binaries are available on GitHub
Releases. Chocolatey and Scoop manifests can be added post-M1 if demand
warrants.

## Status

All packaging channels are **deferred until the first tagged release**.
The GitHub Actions `release.yml` workflow produces the prebuilt binaries
that these package managers consume.
