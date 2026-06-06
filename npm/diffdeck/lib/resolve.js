"use strict";

/** node の platform/arch から OS 別パッケージ名を作る。 */
function packageName(platform, arch) {
  return `@diffdeck/cli-${platform}-${arch}`;
}

/** プラットフォームに応じたバイナリ名。 */
function binName(platform) {
  return platform === "win32" ? "diffdeck.exe" : "diffdeck";
}

module.exports = { packageName, binName };
