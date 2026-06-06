import test from "node:test";
import assert from "node:assert";
import fs from "node:fs";
import os from "node:os";
import path from "node:path";
import { spawnSync } from "node:child_process";
import { fileURLToPath } from "node:url";

const __dirname = path.dirname(fileURLToPath(import.meta.url));
const script = path.join(__dirname, "..", "build-packages.mjs");

test("generates per-OS packages and injects version into main package", () => {
  const tmp = fs.mkdtempSync(path.join(os.tmpdir(), "diffdeck-build-"));
  try {
    const binDir = path.join(tmp, "bins");
    const outDir = path.join(tmp, "out");
    fs.mkdirSync(binDir, { recursive: true });

    // フェイクのクロスビルド成果物（一部ターゲットのみ）。
    fs.writeFileSync(path.join(binDir, "diffdeck-aarch64-apple-darwin"), "BIN-darwin-arm64");
    fs.writeFileSync(path.join(binDir, "diffdeck-x86_64-unknown-linux-gnu"), "BIN-linux-x64");
    fs.writeFileSync(path.join(binDir, "diffdeck-x86_64-pc-windows-msvc.exe"), "BIN-win-x64");

    const res = spawnSync(process.execPath, [script, "1.2.3", binDir, outDir], {
      encoding: "utf8",
    });
    assert.strictEqual(res.status, 0, `stderr: ${res.stderr}`);

    // OS 別パッケージ: darwin-arm64
    const macPkgDir = path.join(outDir, "@diffdeck", "cli-darwin-arm64");
    const macPkg = JSON.parse(fs.readFileSync(path.join(macPkgDir, "package.json"), "utf8"));
    assert.strictEqual(macPkg.name, "@diffdeck/cli-darwin-arm64");
    assert.strictEqual(macPkg.version, "1.2.3");
    assert.deepStrictEqual(macPkg.os, ["darwin"]);
    assert.deepStrictEqual(macPkg.cpu, ["arm64"]);
    assert.strictEqual(
      fs.readFileSync(path.join(macPkgDir, "bin", "diffdeck"), "utf8"),
      "BIN-darwin-arm64"
    );

    // win32 はバイナリ名に .exe が付く
    const winBin = path.join(outDir, "@diffdeck", "cli-win32-x64", "bin", "diffdeck.exe");
    assert.strictEqual(fs.readFileSync(winBin, "utf8"), "BIN-win-x64");

    // 未ビルドのターゲット（darwin-x64）はスキップされる
    assert.ok(!fs.existsSync(path.join(outDir, "@diffdeck", "cli-darwin-x64")));

    // main パッケージ: version と optionalDependencies が注入される
    const mainPkg = JSON.parse(
      fs.readFileSync(path.join(outDir, "diffdeck", "package.json"), "utf8")
    );
    assert.strictEqual(mainPkg.version, "1.2.3");
    assert.deepStrictEqual(mainPkg.optionalDependencies, {
      "@diffdeck/cli-darwin-arm64": "1.2.3",
      "@diffdeck/cli-linux-x64": "1.2.3",
      "@diffdeck/cli-win32-x64": "1.2.3",
    });
    // 未ビルドの darwin-x64 は optionalDependencies に入らない
    assert.ok(!("@diffdeck/cli-darwin-x64" in mainPkg.optionalDependencies));
    // engines などの他フィールドが注入で失われないこと（Task 7 で追加した engines を保持）
    assert.deepStrictEqual(mainPkg.engines, { node: ">=18" });

    // launcher も同梱される
    assert.ok(fs.existsSync(path.join(outDir, "diffdeck", "bin", "diffdeck.js")));
    assert.ok(fs.existsSync(path.join(outDir, "diffdeck", "lib", "resolve.js")));
  } finally {
    fs.rmSync(tmp, { recursive: true, force: true });
  }
});
