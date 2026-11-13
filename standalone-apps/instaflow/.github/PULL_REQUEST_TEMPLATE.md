## Summary of Changes

- [ ] Describe the changes introduced in this Pull Request.

## Verification Checklist

- [ ] `pnpm typecheck` and `pnpm lint` pass with zero errors
- [ ] `pnpm test` passes all Vitest unit tests
- [ ] `cargo test` and `cargo clippy -- -D warnings` pass in `src-tauri`
- [ ] All child processes on Windows use `CREATE_NO_WINDOW` (`0x08000000`) and structured argument slices
- [ ] Sensitive session cookies (`sessionid`, `csrftoken`) and user paths are scrubbed from diagnostic logs
