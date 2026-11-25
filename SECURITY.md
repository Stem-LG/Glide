# Security

Glide 0.1.x is an early local-network application. It can inject pointer and
multitouch input into the desktop, so use it only with devices and networks you trust.

## Current boundaries

- By default HTTP/WebSocket traffic is **not encrypted**. Pairing credentials, input
  and audio can be observed by a party able to inspect network traffic. Do not
  port-forward or publicly expose Glide; a pairing PIN is not a substitute for TLS.
- The optional **Secure connection (HTTPS)** encrypts traffic with a self-signed
  certificate generated on the desktop (`tls-cert.pem`, `tls-key.pem` in Glide's
  config directory). Browsers cannot verify it, so the first-visit warning must be
  accepted by hand; it protects against passive observers, not an active attacker
  on the network at that first visit.
- Desktop audio and the phone microphone are off by default, enabled per device,
  and only served to the device that currently controls input. The microphone
  stops when the phone page is hidden or disconnected.
- The server listens on all IPv4 interfaces on the selected port (default 8766).
  Configure network/firewall access yourself. Glide does not change firewall rules.
- Pairing uses a six-digit PIN, rate limiting, and randomly generated device tokens.
  The desktop stores token hashes, while the browser stores its token locally.
- Only one controller is active at a time. Forgetting/re-pairing a profile revokes
  its previous credential. The input helper validates frames and releases contacts
  after an idle timeout or disconnect.
- Membership in the optional `glide` group grants virtual-input creation rights.
  Only grant it to trusted users. Never run the network server as root.

## Reporting vulnerabilities

Use the repository's **Security → Report a vulnerability** option for sensitive
reports. Do not include pairing tokens, configuration files, or private network
captures in public issues. Include the Glide version, affected component, impact,
and a minimal reproduction when possible.

There is no long-term support branch yet; fixes target the latest preview.
