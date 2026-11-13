## Summary

Briefly describe what this Pull Request adds, fixes, or refactors.

## Related Issues

Closes #

## Type of Change

- [ ] `feat`: New feature
- [ ] `fix`: Bug fix
- [ ] `refactor`: Architecture / code quality improvement
- [ ] `docs`: Documentation update
- [ ] `ci`: GitHub Actions / build tooling
- [ ] `security`: Security hardening

## Verification Checklist

- [ ] `npm run typecheck` passes with zero errors
- [ ] `npm run lint` passes
- [ ] `npm run test` (Vitest) passes
- [ ] `cargo fmt --all -- --check` passes
- [ ] `cargo clippy --all-targets -- -D warnings` passes
- [ ] `cargo test` passes
- [ ] No console windows flash on Windows (`CREATE_NO_WINDOW`)
- [ ] Database migrations (if any) preserve existing history and queue data
