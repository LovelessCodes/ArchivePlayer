/**
 * Register the production service worker for the browser build.
 *
 * The desktop app loads from Tauri's custom protocol and does not need an
 * offline cache. Service workers also require a secure context, so a server
 * reached over plain HTTP on the LAN simply skips registration.
 */
export function registerServiceWorker() {
  if (!import.meta.env.PROD) return;
  if (typeof window === "undefined" || "__TAURI_INTERNALS__" in window) return;
  if (!("serviceWorker" in navigator) || !window.isSecureContext) return;

  window.addEventListener("load", () => {
    void navigator.serviceWorker
      .register("/sw.js", { scope: "/" })
      .catch((error: unknown) => {
        console.info("ArchivePlayer could not enable offline mode.", error);
      });
  });
}
