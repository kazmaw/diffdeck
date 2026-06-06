#!/usr/bin/env node
// 用途: クロスビルド成果物から OS 別 npm パッケージと main パッケージを生成する。
// usage: build-packages.mjs <version> <binDir> <outDir>
//   <binDir>: diffdeck-<rust-target>(.exe) が並ぶディレクトリ
//   <outDir>: 生成先（<outDir>/@diffdeck/cli-* と <outDir>/diffdeck を作る）
import fs from "node:fs";
import path from "node:path";
import { fileURLToPath } from "node:url";

const __dirname = path.dirname(fileURLToPath(import.meta.url));

// rust target -> node の { platform, arch }
const TARGETS = {
  "aarch64-apple-darwin": { platform: "darwin", arch: "arm64" },
  "x86_64-apple-darwin": { platform: "darwin", arch: "x64" },
  "x86_64-unknown-linux-gnu": { platform: "linux", arch: "x64" },
  "x86_64-pc-windows-msvc": { platform: "win32", arch: "x64" },
  // 任意: CI のマトリクスに足せば自動で取り込まれる
  "aarch64-unknown-linux-gnu": { platform: "linux", arch: "arm64" },
};

function main() {
  const [version, binDir, outDir] = process.argv.slice(2);
  if (!version || !binDir || !outDir) {
    console.error("usage: build-packages.mjs <version> <binDir> <outDir>");
    process.exit(1);
  }

  const optionalDependencies = {};

  for (const [target, { platform, arch }] of Object.entries(TARGETS)) {
    const ext = platform === "win32" ? ".exe" : "";
    const src = path.join(binDir, `diffdeck-${target}${ext}`);
    if (!fs.existsSync(src)) continue; // このリリースでビルドしていないターゲットは飛ばす

    const pkgName = `@diffdeck/cli-${platform}-${arch}`;
    const pkgDir = path.join(outDir, pkgName);
    const binOutDir = path.join(pkgDir, "bin");
    fs.mkdirSync(binOutDir, { recursive: true });

    const binFile = `diffdeck${ext}`;
    fs.copyFileSync(src, path.join(binOutDir, binFile));
    fs.chmodSync(path.join(binOutDir, binFile), 0o755);

    fs.writeFileSync(
      path.join(pkgDir, "package.json"),
      JSON.stringify(
        {
          name: pkgName,
          version,
          description: `diffdeck prebuilt binary (${platform}-${arch})`,
          license: "MIT",
          os: [platform],
          cpu: [arch],
          files: ["bin/"],
        },
        null,
        2
      ) + "\n"
    );

    optionalDependencies[pkgName] = version;
  }

  // main パッケージを out にコピーし version / optionalDependencies を注入
  const mainSrc = path.join(__dirname, "diffdeck");
  const mainOut = path.join(outDir, "diffdeck");
  fs.mkdirSync(path.join(mainOut, "bin"), { recursive: true });
  fs.mkdirSync(path.join(mainOut, "lib"), { recursive: true });
  fs.copyFileSync(
    path.join(mainSrc, "bin", "diffdeck.js"),
    path.join(mainOut, "bin", "diffdeck.js")
  );
  fs.copyFileSync(
    path.join(mainSrc, "lib", "resolve.js"),
    path.join(mainOut, "lib", "resolve.js")
  );

  const mainPkg = JSON.parse(fs.readFileSync(path.join(mainSrc, "package.json"), "utf8"));
  mainPkg.version = version;
  mainPkg.optionalDependencies = optionalDependencies;
  fs.writeFileSync(
    path.join(mainOut, "package.json"),
    JSON.stringify(mainPkg, null, 2) + "\n"
  );

  const n = Object.keys(optionalDependencies).length;
  console.log(`built ${n} platform package(s) + main package into ${outDir}`);
}

main();
