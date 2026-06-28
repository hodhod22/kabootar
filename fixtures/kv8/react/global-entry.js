/**
 * ESM entry — esbuild IIFE with `globalName` exposes `KV8ReactRuntime` at top level for Kv8.
 */
import * as React from "react";
import { createRoot, hydrateRoot } from "react-dom/client";
import { version as reactDomVersion } from "react-dom";

export default {
  React: React,
  ReactDOM: {
    createRoot: createRoot,
    hydrateRoot: hydrateRoot,
    version: reactDomVersion,
  },
};
