# Security Policy

## Supported Versions

| Version | Supported          |
| ------- | ------------------ |
| 0.1.x   | :white_check_mark: |
| < 0.1.0 | :x:                |

## Security Architecture & Threat Model

Easy Edits Pro is a desktop application running locally on Linux. To deliver high-performance video preview without WebKit memory bloat, it runs a lightweight loopback streaming server alongside the Tauri backend.

Key security controls implemented:
- **Media Streaming Server Isolation**: Bound exclusively to `127.0.0.1`.
- **Per-Session Authentication Tokens**: Every request to the streaming server must provide a cryptographically random session token generated at launch.
- **Strict Host & Origin Validation**: Only requests containing a valid `Host` header (`127.0.0.1:<port>` or `localhost:<port>`) and approved local origins (`tauri://localhost`, `http://localhost:1420`) are accepted. Cross-origin requests from external web browsers are rejected with `403 Forbidden` and `Vary: Origin`.
- **Media Path Scoping & Canonicalization**: The media server only serves files explicitly imported or registered by the user during the active session. Directory traversal attempts (`..`, symlink escapes) outside registered paths are blocked.
- **SSRF & Network Import Hardening**: Remote asset imports block loopback (`127.0.0.0/8`), private RFC1918 subnets (`10.0.0.0/8`, `172.16.0.0/12`, `192.168.0.0/16`), and link-local (`169.254.0.0/16`) addresses. Media downloads are strictly capped at 512 MiB with sanitized file extensions.

## Reporting a Vulnerability

If you discover a security vulnerability or security bug in Easy Edits Pro:

1. **Do not open a public issue.**
2. Use **GitHub Private Vulnerability Reporting** via the [Security Advisories tab](https://github.com/zvirox/easy-edits-pro/security/advisories/new) on GitHub.
3. Alternatively, report directly to the lead maintainer via email: `zvirox.in@gmail.com` with the subject `[SECURITY] Easy Edits Pro Vulnerability`.

Please include:
- A description of the vulnerability and potential impact.
- Step-by-step reproduction instructions or a minimal Proof of Concept (PoC).
- Your operating system, desktop environment, and Easy Edits Pro version.

We strive to acknowledge receipt of security reports within 48 hours and provide patches or mitigation advice promptly before public disclosure.
