"use strict";
const test = require("node:test");
const assert = require("node:assert");
const fs = require("node:fs");
const os = require("node:os");
const path = require("node:path");
const { spawnSync } = require("node:child_process");
const { packageName, binName } = require("../lib/resolve");

// Windows ではシェルスクリプトのフェイクバイナリが使えないためスキップ。
const skip = process.platform === "win32";

test("launcher resolves the platform package and forwards args + exit code", { skip }, () => {
  const tmp = fs.mkdtempSync(path.join(os.tmpdir(), "diffdeck-launch-"));
  try {
    // フェイクの node_modules を構築する。
    const launcherDir = path.join(tmp, "node_modules", "diffdeck");
    fs.mkdirSync(path.join(launcherDir, "bin"), { recursive: true });
    fs.mkdirSync(path.join(launcherDir, "lib"), { recursive: true });
    fs.copyFileSync(
      path.join(__dirname, "..", "bin", "diffdeck.js"),
      path.join(launcherDir, "bin", "diffdeck.js")
    );
    fs.copyFileSync(
      path.join(__dirname, "..", "lib", "resolve.js"),
      path.join(launcherDir, "lib", "resolve.js")
    );

    // 現在のプラットフォーム向けフェイクバイナリ。引数を echo し exit 7 で終わる。
    const pkg = packageName(process.platform, process.arch);
    const bin = binName(process.platform);
    const pkgBinDir = path.join(tmp, "node_modules", pkg, "bin");
    fs.mkdirSync(pkgBinDir, { recursive: true });
    const fakeBin = path.join(pkgBinDir, bin);
    fs.writeFileSync(fakeBin, `#!/bin/sh\necho "args:$@"\nexit 7\n`);
    fs.chmodSync(fakeBin, 0o755);

    const res = spawnSync(
      process.execPath,
      [path.join(launcherDir, "bin", "diffdeck.js"), "staged", "--merge-base"],
      { encoding: "utf8" }
    );

    assert.strictEqual(res.status, 7, `stderr: ${res.stderr}`);
    assert.match(res.stdout, /args:staged --merge-base/);
  } finally {
    fs.rmSync(tmp, { recursive: true, force: true });
  }
});

test("launcher errors clearly when no platform package is present", () => {
  const tmp = fs.mkdtempSync(path.join(os.tmpdir(), "diffdeck-missing-"));
  try {
    const launcherDir = path.join(tmp, "node_modules", "diffdeck");
    fs.mkdirSync(path.join(launcherDir, "bin"), { recursive: true });
    fs.mkdirSync(path.join(launcherDir, "lib"), { recursive: true });
    fs.copyFileSync(
      path.join(__dirname, "..", "bin", "diffdeck.js"),
      path.join(launcherDir, "bin", "diffdeck.js")
    );
    fs.copyFileSync(
      path.join(__dirname, "..", "lib", "resolve.js"),
      path.join(launcherDir, "lib", "resolve.js")
    );
    // 故意に OS 別パッケージを作らない。

    const res = spawnSync(
      process.execPath,
      [path.join(launcherDir, "bin", "diffdeck.js")],
      { encoding: "utf8" }
    );
    assert.strictEqual(res.status, 1);
    assert.match(res.stderr, /no prebuilt binary/);
    assert.match(res.stderr, /cargo install diffdeck/);
  } finally {
    fs.rmSync(tmp, { recursive: true, force: true });
  }
});
