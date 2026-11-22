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
  homepage "https://klint.dev"
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

function requireValue(args: string[], name: string): string {
  const index = args.indexOf(name);
  const value = index >= 0 ? args[index + 1] : undefined;
  if (!value) {
    throw new Error(`${name} is required`);
  }

  return value;
}

export function formulaFromArgs(args: string[]): { output: string; formula: string } {
  return {
    output: requireValue(args, "--output"),
    formula: homebrewFormula({
      version: requireValue(args, "--version"),
      darwinArm64Sha: requireValue(args, "--darwin-arm64-sha"),
      darwinX64Sha: requireValue(args, "--darwin-x64-sha"),
      linuxArm64Sha: requireValue(args, "--linux-arm64-sha"),
      linuxX64Sha: requireValue(args, "--linux-x64-sha"),
    }),
  };
}
