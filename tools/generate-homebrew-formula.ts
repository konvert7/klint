import { mkdirSync, writeFileSync } from "node:fs";
import { dirname, resolve } from "node:path";

interface FormulaOptions {
  version: string;
  darwinArm64Sha: string;
  darwinX64Sha: string;
  linuxArm64Sha: string;
  linuxX64Sha: string;
}

export function homebrewFormula(options: FormulaOptions): string {
  const version = options.version.replace(/^v/, "");

  return `# typed: false
# frozen_string_literal: true

class Klint < Formula
  desc "Architecture-as-Code linter: layer boundaries, imports and forbidden patterns"
  homepage "https://github.com/konvert7/klint"
  version "${version}"
  license "MIT"

  if OS.mac? && Hardware::CPU.arm?
    url "https://github.com/konvert7/klint/releases/download/v${version}/klint-${version}-darwin-arm64.tar.gz"
    sha256 "${options.darwinArm64Sha}"
  elsif OS.mac?
    url "https://github.com/konvert7/klint/releases/download/v${version}/klint-${version}-darwin-x64.tar.gz"
    sha256 "${options.darwinX64Sha}"
  elsif Hardware::CPU.arm?
    url "https://github.com/konvert7/klint/releases/download/v${version}/klint-${version}-linux-arm64.tar.gz"
    sha256 "${options.linuxArm64Sha}"
  else
    url "https://github.com/konvert7/klint/releases/download/v${version}/klint-${version}-linux-x64.tar.gz"
    sha256 "${options.linuxX64Sha}"
  end

  def install
    binary = File.exist?("klint") ? "klint" : Dir["klint-*/klint"].first
    odie "klint binary not found" unless binary
    bin.install binary => "klint"
  end

  test do
    assert_match version.to_s, shell_output("#{bin}/klint --version")
  end
end
`;
}

if (import.meta.main) {
  const args = process.argv.slice(2);
  const output = requireValue(args, "--output");
  const formula = homebrewFormula({
    version: requireValue(args, "--version"),
    darwinArm64Sha: requireValue(args, "--darwin-arm64-sha"),
    darwinX64Sha: requireValue(args, "--darwin-x64-sha"),
    linuxArm64Sha: requireValue(args, "--linux-arm64-sha"),
    linuxX64Sha: requireValue(args, "--linux-x64-sha"),
  });

  const path = resolve(output);
  mkdirSync(dirname(path), { recursive: true });
  writeFileSync(path, formula);
}

function requireValue(args: string[], name: string): string {
  const index = args.indexOf(name);
  const value = index >= 0 ? args[index + 1] : undefined;
  if (!value) {
    throw new Error(`${name} is required`);
  }
  return value;
}
