# Building a TV or native client

ArchivePlayer's server exposes a device authorization flow so a TV can connect
without typing a password. The flow is modelled on
[RFC 8628](https://www.rfc-editor.org/rfc/rfc8628) and returns a revocable,
opaque bearer token.

This document describes the stable HTTP contract. The desktop app does not expose
these endpoints; they are part of the self-hosted server.

A reference implementation is available as a native Android TV app:
[LovelessCodes/ArchivePlayer-AndroidTV](https://github.com/LovelessCodes/ArchivePlayer-AndroidTV).
It uses NSD for discovery, pairs through this device flow, stores the token in
the Android Keystore, and plays through Media3/ExoPlayer.

## Discovery

A client should not make the user type an IP address on a remote control. When the
server binds to a LAN address, it advertises this DNS-SD service:

```text
_archiveplayer._tcp.local
```

TXT records:

| Key | Meaning |
| --- | --- |
| `name` | Friendly server name, e.g. `Living Room NAS`. |
| `id` | Stable server ID; use it to de-duplicate and remember servers. |
| `api` | API generation, currently `1`. |
| `version` | Server package version, e.g. `0.5.0`. |
| `path` | Web root, currently `/`. |
| `addr` | Preferred LAN IPv4 address selected by the server. |
| `url` | Optional public HTTPS base URL from `ARCHIVEPLAYER_PUBLIC_URL`. |

On Android TV, use `NsdManager` to browse for `_archiveplayer._tcp` and resolve
the service. Build the LAN URL from `addr` when it is present, otherwise from a
resolved private IPv4 address; use `url` for the public HTTPS fallback. Ignore
IPv6 link-local addresses because they are unusable without a scope identifier.
Show the discovered name and URL before connecting. Discovery is unauthenticated,
so treat it as a hint, not proof of identity.

`addr` matters on hosts with several interfaces. A server can otherwise advertise
a Docker bridge or a link-local IPv6 address before the address a television can
actually reach. The server takes the hint from an explicit bind address, or asks
the routing table which source address it would use for an outbound connection.

The advertisement is skipped when the server binds to loopback. Docker bridge
networks do not forward multicast to the LAN; use host networking or a host-side
Avahi advertisement if discovery is required.

## API version

`GET /api/health` is public and returns the contract generation:

```json
{ "status": "ok", "version": "0.5.0", "apiVersion": 1 }
```

Refuse to pair if `apiVersion` is newer than the client supports. Missing
`apiVersion` means a pre-0.5 server; the pairing endpoints above are still
compatible.

## Endpoints

| Method | Path | Authentication | Purpose |
| --- | --- | --- | --- |
| `GET` | `/api/health` | none | Health and API version. |
| `GET` | `/api/auth/status` | none | Current user, registration state, and runtime. |
| `POST` | `/api/device/start` | none | Start a pairing request. |
| `POST` | `/api/device/token` | device code | Poll for approval and redeem the token. |
| `POST` | `/api/device/lookup` | browser cookie | Inspect a pending request by user code. |
| `POST` | `/api/device/approve` | browser cookie | Approve a pending request. |
| `GET` | `/api/devices` | browser cookie | List the signed-in user's sessions. |
| `DELETE` | `/api/devices/{id}` | browser cookie | Revoke one session. |
| `GET` | `/api/library` | bearer token or cookie | Read the library. |
| `GET` | `/api/library/continue-watching` | bearer token or cookie | Read titles with saved progress. |
| `GET` | `/api/search?q={query}&page={page}` | bearer token or cookie | Search Archive.org. |
| `GET` | `/api/playback/{identifier}?platform={platform}&file={file}` | bearer token or cookie | Get video URLs, episode data, and a playback plan. |
| `PUT` | `/api/playback/{identifier}/progress` | bearer token or cookie | Save watch progress. |
| `POST` | `/api/playback/{identifier}/completed` | bearer token or cookie | Mark an episode or title finished. |

The browser approval endpoints deliberately reject bearer tokens: approve a
device from a signed-in browser, not from the device itself.

## 1. Start pairing

The TV sends a friendly name and optional scopes. `library:read` and
`progress:write` are the defaults; a client that also adds titles asks for
`library:write`.

```http
POST /api/device/start
Content-Type: application/json

{
  "clientName": "Living room TV",
  "platform": "webos",
  "scopes": ["library:read", "progress:write"]
}
```

```json
{
  "deviceCode": "387a2f4f…",
  "userCode": "XQK8-NM99",
  "verificationUri": "https://media.example.com/#/activate",
  "verificationUriComplete": "https://media.example.com/#/activate?code=XQK8-NM99",
  "expiresIn": 600,
  "interval": 5
}
```

Show `userCode` on the TV and, if it has a display, a QR code for
`verificationUriComplete`. The device code is secret; the user code is what a
person types into the browser.

`ARCHIVEPLAYER_PUBLIC_URL` controls the host used in the verification URLs. If it
is unset, the server uses the request `Host` header. `X-Forwarded-Proto` is only
trusted when `ARCHIVEPLAYER_TRUST_PROXY=true`.

## 2. Poll for the token

Poll every `interval` seconds. Treat `slow_down` as an instruction to increase the
interval.

```http
POST /api/device/token
Content-Type: application/json

{ "deviceCode": "387a2f4f…" }
```

While waiting:

```json
{ "code": "authorization_pending", "message": "Waiting for approval." }
```

`HTTP 400` with `authorization_pending`, `access_denied`, `expired_token`, or
`invalid_grant`. `HTTP 429` with `slow_down` means the previous poll was too soon.
The pairing endpoints also have a per-IP request limit; a generic
`HTTP 429` with `code: "rate_limited"` includes a `Retry-After` header in
seconds. Honour it and back off rather than retrying immediately.

After approval, the response is `HTTP 200`:

```json
{
  "token": "<session-token>",
  "expiresAt": 1809948069,
  "user": { "id": 1, "email": "owner@example.com", "displayName": "Owner" }
}
```

Store `token` in platform-secure storage (Android Keystore / iOS Keychain /
webOS app storage). The token is shown once and stored server-side only as a
SHA-256 hash.

## 3. Call the API

```http
GET /api/library
Authorization: Bearer <token>
```

Library items carry optional `tmdbPosterUrl` (2:3) and `tmdbBackdropUrl`
(16:9) fields. Both are absolute TMDB URLs; a TV may fetch them directly, and
should keep its own fallback when either is absent. The detail response repeats
the same fields. `thumbnailUrl` remains a smaller fallback, not a wide image.

The same session accepts a browser cookie, but native clients should always use
the bearer header. Session tokens are checked against the database on every
request, so revocation takes effect immediately.

The token carries scopes:

| Scope | Allows |
| --- | --- |
| `library:read` | Library, search, metadata, playback details. |
| `library:write` | Add/remove titles and change autoplay. |
| `progress:write` | Save progress and mark completed. |
| `settings:manage` | Server settings; browser sessions only. |

Requests outside the granted scopes return `HTTP 403` with code `forbidden`.
A playback-only TV should request `library:read` and `progress:write`; the webOS
client also requests `library:write` so it can add an Archive.org title from the
search screen.

### Playback selection

Every playback response includes the raw files and a server-selected
`playbackPlan`. Ask for the plan that matches your player:

```http
GET /api/playback/57afnt64nightofthelivingdeadv2?platform=webos
Authorization: Bearer <token>
```

For a specific episode or quality, add the exact `fileName` from the response:

```http
GET /api/playback/the_twilight_zone?platform=webos&file=The.Twilight.Zone.S01E01.mp4
```

The plan looks like this:

```json
{
  "playbackPlan": {
    "platform": "webos",
    "fileName": "57 A FNT # 64 Night of the Living Dead V2.ia.mp4",
    "url": "https://archive.org/download/…/57%20A%20FNT%20%23%2064%20Night%20of%20the%20Living%20Dead%20V2.ia.mp4",
    "mimeType": "video/mp4",
    "format": "h.264 IA",
    "container": "mp4",
    "videoCodec": "h264",
    "sizeBytes": 1854950726,
    "lengthSeconds": 7199.99,
    "width": 640,
    "height": 360,
    "compatible": true,
    "reason": null
  }
}
```

`compatible: false` means the selected file is known to need a decoder the
requested platform cannot rely on; show `reason` rather than starting playback.
`playbackPlan` is `null` when the item has no video files. The plan is a
recommendation, not an access-control decision: the same URLs are returned in
`videoFileUrls`, which now also carry `format`, `mimeType`, `sizeBytes`,
`lengthSeconds`, `width`, `height`, and `videoCodec`.

`platform` accepts `webos`, `androidtv`, `web`, or `generic` (the default). A
native webOS client should send `platform=webos` and use the returned MP4; a
browser player can ignore the plan and keep using `videoFileUrls`.

`durationSeconds` is the duration the user's player saved with its progress, not
the Archive.org file length. It is `0` until the client has saved progress; use
`playbackPlan.lengthSeconds` when a duration is needed before playback starts.

## 4. Approval and revocation

In a signed-in browser, open `/#/activate?code=XQK8-NM99` (or enter the code on
that page). The page calls `POST /api/device/lookup` to show the client name,
platform, and requested scopes, then `POST /api/device/approve`.

`GET /api/devices` returns sessions for the current user:

```json
[
  {
    "id": "dc788a15eee46ce0",
    "kind": "browser",
    "label": "Chrome on Linux",
    "platform": "web",
    "scopes": ["*"],
    "createdAt": "2026-11-11T11:21:08.945Z",
    "lastSeenAt": 1794396068,
    "expiresAt": 1796988068,
    "current": true
  },
  {
    "id": "8f4f15a6fa8993a6",
    "kind": "device",
    "label": "Living room TV",
    "platform": "webos",
    "scopes": ["library:read", "progress:write"],
    "createdAt": "2026-11-11T11:21:09.021Z",
    "lastSeenAt": 1794396069,
    "expiresAt": 1809948069,
    "current": false
  }
]
```

`DELETE /api/devices/{id}` revokes a session. Revoking the current browser session
also clears its cookie.

## Remote access

mDNS only works on the same LAN. For a server reached over the internet, keep a
stable HTTPS endpoint and let the client store two addresses per server:

```text
lanUrl      // discovered on the local network
remoteUrl   // ARCHIVEPLAYER_PUBLIC_URL
```

Try `lanUrl` first and fall back to `remoteUrl` on timeout. Pairing at home is
enough: discovery can return the public URL in its TXT record, so the client can
remember it for travel without asking the user to type it on the TV.

Recommended deployment options:

- **Tailscale / WireGuard** — private access without opening ports; works through
  CGNAT. Use MagicDNS or a stable tailnet hostname.
- **TLS reverse proxy** — Caddy, nginx, or Traefik with a real certificate. Set
  `ARCHIVEPLAYER_TRUST_PROXY=true` so forwarding headers are trusted.
- **Cloudflare Tunnel** — outbound-only HTTPS with no open ports; adds a third
  party in front of the API. Video still comes directly from Archive.org.

Never expose the raw HTTP port to the internet. For a reverse proxy, rate
limiting uses the proxy address unless `ARCHIVEPLAYER_TRUST_PROXY=true`; without
that setting all clients share one bucket.

## Client notes

### Android TV

- Kotlin + Jetpack Compose for TV, or a WebView wrapper.
- Media3/ExoPlayer can stream the Archive.org MP4 URLs returned by
  `/api/playback/{identifier}`.
- Keep the token in `EncryptedSharedPreferences` backed by the Android Keystore.
- Android blocks cleartext HTTP by default. Use HTTPS, or a debug-only network
  security config while developing.
- A D-pad-first UI and 10-foot typography matter more than a phone layout.

### LG webOS

- A native webOS client should call `/api/playback/{identifier}?platform=webos`
  and play `playbackPlan.url` through `com.webos.media`. See the webOS client
  repository for the SDL2/GLES2 shell.
- A webOS web app is a Chromium runtime, so the server must allow its origin.
  Set `ARCHIVEPLAYER_CORS_ORIGINS` to the packaged app origin (or `null` for a
  `file://` package) and restart the server.
- Store the bearer token in the app's local storage; webOS packages do not have a
  system keychain equivalent.
- Use the Magic Remote back key and arrow keys for navigation. Avoid hover-only
  controls.
- For a zero-CORS option, host the TV UI from the ArchivePlayer server itself
  (same origin) and open that URL in the TV browser.

CORS is off by default. It only affects cross-origin browser runtimes; native
Android requests and same-origin pages work without it. The server does not enable
`Access-Control-Allow-Credentials`: cross-origin clients must use bearer tokens,
not cookies.

### Testing without a TV

```bash
# Start a pairing request.
curl -s https://media.example.com/api/device/start \
  -H 'Content-Type: application/json' \
  -d '{"clientName":"Test TV","platform":"webos"}'

# Poll with the returned deviceCode.
curl -s https://media.example.com/api/device/token \
  -H 'Content-Type: application/json' \
  -d '{"deviceCode":"…"}'
```

Approve the returned user code at `/#/activate`, then poll again. Use the token:

```bash
curl -s https://media.example.com/api/library \
  -H "Authorization: Bearer $TOKEN"
```
