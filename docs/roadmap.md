# Roadmap

This page tracks known future improvements. It only lists work that is not
complete yet; current behavior is documented in the install, metrics, and JSON
pages.

## Distribution

### Developer ID Signing And Notarization

Current release binaries are ad-hoc signed, not Developer ID signed or
notarized. They run from the command line after installation, but macOS
Gatekeeper assessment rejects them:

```sh
spctl --assess --type execute -vv "$(command -v macvmtop)"
```

Developer ID signing requires Apple Developer Program credentials and release
workflow secrets. Once those exist, the release workflow should sign each macOS
binary, notarize it with Apple, and keep publishing checksums for installer
verification.

### Homebrew

Add a Homebrew tap or formula after the release process is stable. The formula
should install the GitHub Release archive for the current architecture and
verify its checksum.

### Provenance

Release archives currently publish SHA-256 checksums, and the hosted installer
verifies `SHA256SUMS` before extraction. Future releases can add stronger
provenance, such as signed checksums or build attestations.

## Terminal UI

The TUI is usable and has a compact fallback for very small terminals. Future
TUI work should focus on:

- column sorting for the process table
- better resize behavior under rapid terminal size changes
- optional detail panels for selected processes, network interfaces, and
  mounted volumes
- longer-duration soak tests in small and remote terminals

## JSON Contract

JSON output is intended for scripts. Future hardening should add:

- snapshot tests for representative system and sampled JSON output
- a published JSON schema
- compatibility notes when fields are added or renamed

## Metrics

Keep the VM scope strict. Add a metric only when virtualized macOS exposes a
real value from inside the guest. Do not synthesize physical host metrics.
