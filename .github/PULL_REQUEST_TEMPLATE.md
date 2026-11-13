## Summary
<!-- Describe what changed and why -->

## Verification Checklist
- [ ] `npm run typecheck && npm run test && npm run build` passes cleanly
- [ ] `cargo test --lib -- --skip test_live_multi_platform_inspect_and_download` passes cleanly
- [ ] All Windows child process spawns enforce `CREATE_NO_WINDOW (0x08000000)`
- [ ] Sensitive cookie/session values remain redacted in logs
