# Contributing to FlowFB

Thank you for your interest in contributing to **FlowFB**!

## Development Workflow

1. Fork the repository and create a feature branch (`git checkout -b feat/your-feature`).
2. Ensure all TypeScript checks pass:
   ```bash
   npm run typecheck
   ```
3. Ensure all Vitest frontend unit tests pass:
   ```bash
   npm run test
   ```
4. Ensure all Rust backend unit tests pass:
   ```bash
   cd src-tauri && cargo test
   ```
5. Verify that any subprocess spawned on Windows sets `.creation_flags(0x08000000)` (`CREATE_NO_WINDOW`) so console windows never flash.

## Code Standards
- Never log raw cookie values (`c_user`, `xs`, `datr`, `fr`) or decrypted vault contents.
- All user-facing errors must serialize cleanly through `AppError` with remediation guidance.
