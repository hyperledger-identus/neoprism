# TLS dependency remediation for issue 369

This assessment accompanies the dependency-only follow-up to the six alerts
recorded against v0.15.0 in [issue 369][issue]. Advisory details and dependency
resolution were checked on 2026-11-17. It is not a risk acceptance, an assertion
that deployed systems were unexploitable, or confirmation that GitHub alerts
have closed.

## Resolution and dependency paths

The shared `Cargo.lock` updates `openssl` 0.10.75 to 0.10.80,
`openssl-sys` 0.9.111 to 0.9.116 (required by the updated wrapper), and
`rustls-webpki` 0.103.6 to 0.103.13. These are compatible patch updates;
workspace manifests, SDK git revision, and Nix inputs are unchanged.
`nix/rustTools.nix` imports that lockfile directly, so Nix uses the same
resolution without a separate dependency hash refresh. Git source output hashes
are unchanged because neither git dependency changed.

0.10.79 is insufficient for the current OpenSSL advisory set:
[GHSA-phqj-4mhp-q6mq][cipher-inplace] also affects 0.10.79 and requires
0.10.80. The selected version covers that advisory and the additional
[AES-KW-PAD overflow][cipher-padding] fixed in 0.10.79. The public advisory
queries for both packages identified no higher patched-version requirement on
the selected stable release lines. This deliberately avoids an unrelated
latest-dependency refresh; newer patch releases exist.

The resolved feature paths are:

- **OpenSSL (Linux/native TLS):** `neoprism-node` enables the indexer's
  `blockfrost` feature and the submitter's `cardano-wallet` and
  `embedded-wallet` features. Blockfrost's generated client enables reqwest
  defaults; the submitter explicitly enables reqwest `native-tls`.
  Both lead through reqwest 0.12.24, native-tls 0.2.14 (also via hyper-tls /
  tokio-native-tls), openssl, and openssl-sys. Thus disabling reqwest defaults
  in the workspace declaration alone does not remove OpenSSL.
- **WebPKI (locked optional TLS path):** the lockfile/metadata path is
  reqwest 0.12.24 → hyper-rustls 0.27.7 / tokio-rustls 0.26.3 →
  rustls 0.23.32 → rustls-webpki. Reqwest's `__rustls` feature activates
  this backend. In the checked workspace graph, including all workspace
  features and all targets, `cargo tree -i rustls-webpki` prints no active
  path: reqwest's Rustls features are not enabled. Cargo metadata includes
  optional dependency resolution; presence there is not evidence of linking.
  The locked package is patched regardless, rather than relying on that
  feature configuration as a waiver.

## Six-alert applicability table

All six baseline package versions match the live alert ranges. “No call found”
below means source inspection of the workspace and native-tls 0.2.14's OpenSSL
adapter, not a complete transitive call-graph proof. OpenSSL's internal C use of
cryptographic operations is distinct from calling the affected Rust wrappers.

| Alert / advisory | Affected operation and precondition | Observed applicability and limits | Patched disposition |
| --- | --- | --- | --- |
| [32][a32] / [GHSA-ghm9-cr32-g9qj][g32] | `MdCtxRef::digest_final` with an undersized output; affected >=0.10.39, <0.10.78 | OpenSSL native-TLS path is active. No call to this Rust API found in inspected callers; deployed exploitability not established. | openssl 0.10.80, above fix 0.10.78 |
| [33][a33] / [GHSA-hppc-g8h3-xhp3][g33] | Oversized returned lengths from PSK or cookie callbacks; affected >=0.9.24, <0.10.78 | No PSK/cookie callback registration found in inspected callers. Native TLS alone does not demonstrate these callbacks are reachable. | openssl 0.10.80, above fix 0.10.78 |
| [34][a34] / [GHSA-8c75-8mhr-p7r9][g34] | `aes::unwrap_key` inverted output bounds assertion; affected >=0.10.24, <0.10.78 | No call to this Rust API found in inspected callers. This does not rule out all transitive or downstream usage. | openssl 0.10.80, above fix 0.10.78 |
| [36][a36] / [GHSA-pqf5-4pqq-29f5][g36] | `Deriver::derive` / `PkeyCtxRef::derive` short buffer with OpenSSL 1.1.1; affected >=0.9.27, <0.10.78 | No call found in inspected callers. Nix's Linux build input is OpenSSL 3.6.3, not the affected 1.1.1 implementation. Other builds/deployments need their own linkage evidence. | openssl 0.10.80, above fix 0.10.78; not a platform-based waiver |
| [37][a37] / [GHSA-82j2-j2ch-gfr8][g37] | Malformed CRL BIT STRING; attacker-influenced CRL parsing with opt-in revocation configuration; affected <0.103.13 | Rustls path is locked but not activated in the checked workspace feature graph. No workspace CRL/revocation configuration found. Different downstream features/configuration remain outside this observation. | rustls-webpki 0.103.13, the fixed stable version |
| [38][a38] / [GHSA-xp3w-r5p5-63rr][g38] | `X509Ref::ocsp_responders` creates invalid UTF-8 from malformed certificate OCSP URLs; affected >=0.9.7, <0.10.79 | Certificate input exists on native TLS paths, but no call to this Rust OCSP URL accessor found in inspected callers. Certificate processing alone is not proof of accessor reachability. | openssl 0.10.80, above fix 0.10.79 |

## Linked OpenSSL and evidence boundaries

The Linux Nix binary and checks explicitly take `openssl` as a build input and
use pkg-config; vendored OpenSSL is not enabled in the resolved feature graph.
The pinned Nix binary derivation supplies `openssl-3.6.3-dev`. The Rust package
version 0.10.80 and openssl-sys version 0.9.116 are **not** versions of the linked
C library. Exact built-binary ELF dependencies and closure paths belong in the
validation evidence; the derivation alone is not a claim about a running
production process. macOS native-tls uses a different platform backend.

Reproduction commands (inside `nix develop` for Cargo commands):

```sh
cargo tree --locked --workspace --all-features --target all -e features -i openssl
cargo tree --locked --workspace --all-features --target all -e features -i rustls-webpki
cargo metadata --locked --format-version 1
nix derivation show .#neoprism-bin
nix flake check -L
just lint-text
```

The remediation PR records exact-head local/hosted validation, including
conformance and built-binary linkage evidence. Passing tests do not establish
absence of security exposure. The six GitHub alerts must be checked again
**after merge**, and a maintainer must identify the release shipping this
lockfile. Neither alert closure nor release delivery follows from opening a PR.

[issue]: https://github.com/hyperledger-identus/neoprism/issues/369
[a32]: https://github.com/hyperledger-identus/neoprism/security/dependabot/32
[a33]: https://github.com/hyperledger-identus/neoprism/security/dependabot/33
[a34]: https://github.com/hyperledger-identus/neoprism/security/dependabot/34
[a36]: https://github.com/hyperledger-identus/neoprism/security/dependabot/36
[a37]: https://github.com/hyperledger-identus/neoprism/security/dependabot/37
[a38]: https://github.com/hyperledger-identus/neoprism/security/dependabot/38
[g32]: https://github.com/advisories/GHSA-ghm9-cr32-g9qj
[g33]: https://github.com/advisories/GHSA-hppc-g8h3-xhp3
[g34]: https://github.com/advisories/GHSA-8c75-8mhr-p7r9
[g36]: https://github.com/advisories/GHSA-pqf5-4pqq-29f5
[g37]: https://github.com/advisories/GHSA-82j2-j2ch-gfr8
[g38]: https://github.com/advisories/GHSA-xp3w-r5p5-63rr
[cipher-inplace]: https://github.com/advisories/GHSA-phqj-4mhp-q6mq
[cipher-padding]: https://github.com/advisories/GHSA-xv59-967r-8726
