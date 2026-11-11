/* ArchivePlayer service worker.
 *
 * The app shell is cached so an installed app opens instantly and still renders
 * its own offline page without a connection. The catalog API and media streams
 * are always fetched from the network: a stale library or a half-cached video
 * would be worse than an honest error.
 */
const VERSION = "archiveplayer-v1";
const APP_SHELL = [
  "/offline.html",
  "/manifest.webmanifest",
  "/favicon.svg",
  "/icons/icon-192.png",
  "/icons/icon-512.png",
  "/icons/icon-maskable-512.png",
  "/icons/apple-touch-icon.png",
];

function assetUrls(css) {
  return [...css.matchAll(/url\(\s*["']?(\/assets\/[^"')]+)["']?\s*\)/g)].map(
    (match) => match[1],
  );
}

async function precache() {
  const cache = await caches.open(VERSION);
  await cache.addAll(APP_SHELL);

  // Cache the hashed entry points from the built index.html. Runtime caching
  // alone would leave the first offline visit without its JavaScript and CSS.
  const index = await fetch("/index.html", { cache: "reload" });
  if (!index.ok) throw new Error(`index.html returned ${index.status}`);
  const html = await index.clone().text();
  await cache.put("/index.html", index.clone());
  await cache.put("/", index.clone());

  const assets = new Set(
    [...html.matchAll(/(?:src|href)="(\/assets\/[^"]+)"/g)].map((match) => match[1]),
  );
  await Promise.all(
    [...assets].map(async (asset) => {
      const response = await fetch(asset, { cache: "reload" });
      if (!response.ok) return;
      await cache.put(asset, response.clone());
      if (!asset.endsWith(".css")) return;
      const nested = assetUrls(await response.text());
      await Promise.all(nested.map((url) => cache.add(url).catch(() => undefined)));
    }),
  );
}

self.addEventListener("install", (event) => {
  event.waitUntil(precache().then(() => self.skipWaiting()));
});

self.addEventListener("activate", (event) => {
  event.waitUntil(
    caches
      .keys()
      .then((keys) =>
        Promise.all(
          keys
            .filter((key) => key.startsWith("archiveplayer-") && key !== VERSION)
            .map((key) => caches.delete(key)),
        ),
      )
      .then(() => self.clients.claim()),
  );
});

self.addEventListener("message", (event) => {
  if (event.data === "SKIP_WAITING") self.skipWaiting();
});

async function navigationResponse(request) {
  const cache = await caches.open(VERSION);
  try {
    const response = await fetch(request, { cache: "no-cache" });
    if (response.ok) {
      cache.put(request, response.clone());
      cache.put("/index.html", response.clone());
    }
    return response;
  } catch {
    return (
      (await cache.match(request, { ignoreVary: true })) ||
      (await cache.match("/index.html", { ignoreVary: true })) ||
      (await cache.match("/offline.html", { ignoreVary: true })) ||
      Response.error()
    );
  }
}

async function staticResponse(request) {
  const cache = await caches.open(VERSION);
  const cached = await cache.match(request, { ignoreVary: true });
  if (cached) return cached;

  try {
    const response = await fetch(request);
    if (response.ok && response.type === "basic") {
      const length = Number(response.headers.get("content-length") || "0");
      if (!length || length < 5_000_000) {
        cache.put(request, response.clone());
      }
    }
    return response;
  } catch {
    return Response.error();
  }
}

self.addEventListener("fetch", (event) => {
  const { request } = event;
  if (request.method !== "GET") return;

  const url = new URL(request.url);
  if (url.origin !== self.location.origin) return;
  if (url.pathname.startsWith("/api/")) return;
  if (url.pathname === "/sw.js") return;
  if (request.headers.has("range")) return;
  if (request.destination === "video" || request.destination === "audio") return;

  event.respondWith(
    request.mode === "navigate" ? navigationResponse(request) : staticResponse(request),
  );
});
