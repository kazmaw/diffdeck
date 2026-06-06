"use strict";
const test = require("node:test");
const assert = require("node:assert");
const { packageName, binName } = require("../lib/resolve");

test("packageName maps node platform/arch to scoped package", () => {
  assert.strictEqual(packageName("darwin", "arm64"), "@diffdeck/cli-darwin-arm64");
  assert.strictEqual(packageName("linux", "x64"), "@diffdeck/cli-linux-x64");
  assert.strictEqual(packageName("win32", "x64"), "@diffdeck/cli-win32-x64");
});

test("binName appends .exe only on win32", () => {
  assert.strictEqual(binName("win32"), "diffdeck.exe");
  assert.strictEqual(binName("linux"), "diffdeck");
  assert.strictEqual(binName("darwin"), "diffdeck");
});
