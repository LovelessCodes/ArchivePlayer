#!/usr/bin/env node
/**
 * Generate the screenshots used by the README.
 *
 * The UI runs against the browser mock backend, so no Rust server or network
 * connection is required. Screenshots are committed assets; regenerate them
 * when a screen changes:
 *
 *   pnpm screenshots
 *
 * Set SCREENSHOT_BASE_URL to reuse an already-running Vite dev server. Set
 * SCREENSHOT_OUT to write somewhere other than docs/screenshots.
 */
import { spawn } from "node:child_process";
import fs from "node:fs";
import path from "node:path";
import process from "node:process";
import { chromium, devices } from "playwright";

const root = path.resolve(import.meta.dirname, "..");
const outDir = path.resolve(root, process.env.SCREENSHOT_OUT || "docs/screenshots");
const externalBase = process.env.SCREENSHOT_BASE_URL;
const port = Number(process.env.SCREENSHOT_PORT || 4321);
const base = externalBase || `http://127.0.0.1:${port}`;

function log(message) {
  process.stdout.write(`screenshots: ${message}\n`);
}

async function waitForServer(url, timeoutMs = 30_000) {
  const deadline = Date.now() + timeoutMs;
  while (Date.now() < deadline) {
    try {
      const response = await fetch(url);
      if (response.ok) return;
    } catch {
      // Vite is still starting.
    }
    await new Promise((resolve) => setTimeout(resolve, 250));
  }
  throw new Error(`Vite did not start at ${url}`);
}

function startVite() {
  if (externalBase) return null;
  const command = process.platform === "win32" ? "pnpm.cmd" : "pnpm";
  const child = spawn(
    command,
    ["exec", "vite", "--host", "127.0.0.1", "--port", String(port), "--strictPort"],
    {
      cwd: root,
      env: { ...process.env, VITE_USE_MOCK: "true", BROWSER: "none" },
      stdio: ["ignore", "pipe", "pipe"],
    },
  );

  let output = "";
  child.stopRequested = false;
  child.stdout.on("data", (chunk) => {
    output = (output + chunk).slice(-4000);
  });
  child.stderr.on("data", (chunk) => {
    output = (output + chunk).slice(-4000);
  });
  child.on("exit", (code) => {
    if (!child.stopRequested && code && code !== 0) {
      log(`Vite exited with code ${code}\n${output}`);
    }
  });
  return child;
}

async function settle(page) {
  await page.evaluate(async () => {
    await document.fonts.ready;
    await Promise.all(
      [...document.images]
        .filter((image) => !image.complete)
        .map(
          (image) =>
            new Promise((resolve) => {
              image.addEventListener("load", resolve, { once: true });
              image.addEventListener("error", resolve, { once: true });
            }),
        ),
    );
  });
  await page.waitForTimeout(200);
}

async function capture(page, name) {
  await settle(page);
  const file = path.join(outDir, `${name}.webp`);
  await page.screenshot({
    path: file,
    type: "webp",
    quality: 92,
    animations: "disabled",
    caret: "hide",
  });
  log(`wrote ${path.relative(root, file)}`);
}

async function signUp(page, displayName) {
  await page.goto(`${base}/#/auth`, { waitUntil: "domcontentloaded" });
  await page.getByRole("heading", { name: "Create your profile" }).waitFor();
  await page.getByLabel("Display name").fill(displayName);
  await page.getByLabel("Email").fill("demo@archiveplayer.local");
  await page.getByLabel("Password", { exact: true }).fill("archiveplayer-demo");
  await page.getByLabel(/^Confirm/).fill("archiveplayer-demo");
  await page.getByRole("button", { name: "Create profile" }).click();
  await page.getByRole("heading", { name: `Welcome back, ${displayName}` }).waitFor();
}

async function desktopShots(browser) {
  const context = await browser.newContext({
    viewport: { width: 1440, height: 960 },
    deviceScaleFactor: 2,
    colorScheme: "dark",
    locale: "en-US",
    reducedMotion: "reduce",
  });
  const page = await context.newPage();

  await page.goto(`${base}/#/auth`, { waitUntil: "domcontentloaded" });
  await page.getByRole("heading", { name: "Create your profile" }).waitFor();
  await capture(page, "desktop-auth");

  await signUp(page, "Demo Viewer");
  await page.getByText("Continue watching").waitFor();
  await capture(page, "desktop-library");

  await page.goto(`${base}/#/add`, { waitUntil: "domcontentloaded" });
  await page.getByRole("heading", { name: /Add something to your library/ }).waitFor();
  await page.getByPlaceholder(/Search Archive.org/).fill("19");
  await page.getByRole("button", { name: "Search", exact: true }).click();
  await page.getByText(/Results for/).waitFor();
  await capture(page, "desktop-discover");

  await page.goto(`${base}/#/settings`, { waitUntil: "domcontentloaded" });
  await page.getByRole("heading", { name: "Settings", exact: true }).waitFor();
  await page.getByText("TMDB metadata").waitFor();
  await capture(page, "desktop-settings");

  await context.close();
}

async function mobileShots(browser) {
  const context = await browser.newContext({
    ...devices["iPhone 14"],
    colorScheme: "dark",
    locale: "en-US",
    reducedMotion: "reduce",
  });
  const page = await context.newPage();

  await signUp(page, "Demo Viewer");
  await page.getByText("Continue watching").waitFor();
  await capture(page, "mobile-library");

  await page.goto(`${base}/#/add`, { waitUntil: "domcontentloaded" });
  await page.getByRole("heading", { name: /Add something to your library/ }).waitFor();
  await page.getByPlaceholder(/Search Archive.org/).fill("19");
  await page.getByRole("button", { name: "Search", exact: true }).click();
  await page.getByText(/Results for/).waitFor();
  await capture(page, "mobile-discover");

  await page.goto(`${base}/#/settings`, { waitUntil: "domcontentloaded" });
  await page.getByRole("heading", { name: "Settings", exact: true }).waitFor();
  await capture(page, "mobile-settings");

  await context.close();
}

async function main() {
  fs.mkdirSync(outDir, { recursive: true });
  const vite = startVite();
  let browser;

  try {
    await waitForServer(base);
    browser = await chromium.launch();
    await desktopShots(browser);
    await mobileShots(browser);
    log(`done — ${fs.readdirSync(outDir).filter((file) => file.endsWith(".webp")).length} images in ${path.relative(root, outDir)}`);
  } finally {
    await browser?.close();
    if (vite && !vite.killed) {
      vite.stopRequested = true;
      vite.kill("SIGTERM");
    }
  }
}

main().catch((error) => {
  console.error(error);
  process.exitCode = 1;
});
