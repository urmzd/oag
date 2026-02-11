import { defineConfig } from "tsdown";

export default defineConfig({
  entry: ["src/index.tsx"],
  outDir: "dist",
  format: ["esm"],
  dts: true,
  clean: true,
});