## What changes?

<!-- Short and concrete. Link an issue if there is one. -->

## How was it checked?

- [ ] `npm run check`
- [ ] `cargo fmt --all --check`, `cargo clippy --workspace --all-targets -- -D warnings`
- [ ] `cargo test --workspace`
- [ ] Tried in demo mode (`npm run dev`)
- [ ] Tried on Windows (only needed for shell, drag & drop, OneDrive, network, installer)

## Security

- [ ] New path inputs are validated in the backend with `validate_path`
- [ ] No new permissions in `capabilities`, no `{@html}`, nothing is opened automatically
- [ ] Not applicable

## Notes for review

<!-- Screenshots/GIFs, open questions, known limits. -->
