import * as esbuild from "esbuild";
import { readFileSync } from "node:fs";
import { dirname, join } from "node:path";
import { fileURLToPath } from "node:url";

const here = dirname(fileURLToPath(import.meta.url));
const pkg = JSON.parse(readFileSync(join(here, "package.json"), "utf8"));
const reactVersion = pkg.dependencies.react;

await esbuild.build({
  entryPoints: [join(here, "global-entry.js")],
  bundle: true,
  outfile: join(here, "react-runtime.bundle.js"),
  format: "iife",
  globalName: "KV8ReactRuntime",
  platform: "browser",
  target: "es2015",
  minify: true,
  legalComments: "none",
  define: {
    "process.env.NODE_ENV": '"production"',
  },
  supported: {
    arrow: false,
    "const-and-let": false,
    "for-of": true,
    "for-await": false,
    "dynamic-import": false,
    "async-generator": false,
    "async-await": false,
    "optional-chain": true,
    "nullish-coalescing": true,
    bigint: false,
    "template-literal": true,
    "object-extensions": false,
    "exponent-operator": false,
    destructuring: false,
    "default-argument": false,
    "rest-argument": false,
  },
  banner: {
    js: `/* React ${reactVersion} — esbuild ESM bundle for Kv8 (regenerate: npm run build) */`,
  },
});

console.log("wrote react-runtime.bundle.js");
