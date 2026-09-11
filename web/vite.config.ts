// SPDX-FileCopyrightText: Trident IoT, LLC <https://www.tridentiot.com>
// SPDX-License-Identifier: MIT
import { defineConfig } from "vite";

export default defineConfig({
  // Relative paths so the bundle works from any subdirectory, such as a
  // GitHub Pages project site.
  base: "./",
  build: { target: "es2022" },
});
