import { defineConfig } from "vite"

export default defineConfig({
  // The window ships inside a WKWebView on macOS 14+, so there is no older
  // engine to down-level for.
  build: { target: "safari17", cssMinify: "lightningcss" },
})
