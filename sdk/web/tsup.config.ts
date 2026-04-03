import { defineConfig } from "tsup";

export default defineConfig({
  entry: ["src/index.ts"],
  format: ["esm", "cjs", "iife"],
  globalName: "RiftSDK",
  dts: true,
  clean: true,
  minify: true,
  outDir: "dist",
  // The IIFE build creates dist/index.global.js — used by the server's /sdk/rift.js endpoint
  esbuildOptions(options, context) {
    if (context.format === "iife") {
      // Expose Rift on window for script tag usage
      options.footer = {
        js: "if(typeof window!=='undefined'){window.Rift=RiftSDK.Rift||RiftSDK.default||RiftSDK;}",
      };
    }
  },
});
