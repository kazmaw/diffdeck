#!/usr/bin/env node
"use strict";

const { spawnSync } = require("node:child_process");
const { packageName, binName } = require("../lib/resolve");

const platform = process.platform; // darwin | linux | win32 | ...
const arch = process.arch; // arm64 | x64 | ...
const pkg = packageName(platform, arch);
const bin = binName(platform);

let binPath;
try {
  // OS 別パッケージは optionalDependencies で入る。bin/<name> を解決。
  binPath = require.resolve(`${pkg}/bin/${bin}`);
} catch (_e) {
  process.stderr.write(
    `diffdeck: no prebuilt binary for ${platform}-${arch}.\n` +
      `Install from source instead:\n` +
      `  cargo install diffdeck\n`
  );
  process.exit(1);
}

// stdio を継承して TUI に tty を渡す。引数はそのまま透過。
const result = spawnSync(binPath, process.argv.slice(2), { stdio: "inherit" });
if (result.error) {
  process.stderr.write(`diffdeck: failed to launch binary: ${result.error.message}\n`);
  process.exit(1);
}
// シグナル終了も考慮しつつ終了コードを透過。
process.exit(result.status === null ? 1 : result.status);
