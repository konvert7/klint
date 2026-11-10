import { describe, expect, test } from "bun:test";
import { homebrewFormula } from "../tools/generate-homebrew-formula";

describe("Homebrew formula generation", () => {
  test("renders Darwin and Linux release asset URLs and checksums", () => {
    const formula = homebrewFormula({
      version: "v0.1.2",
      darwinArm64Sha: "a".repeat(64),
      darwinX64Sha: "b".repeat(64),
      linuxArm64Sha: "c".repeat(64),
      linuxX64Sha: "d".repeat(64),
    });

    expect(formula).toContain("class Klint < Formula");
    expect(formula).toContain('version "0.1.2"');
    expect(formula).toContain("if OS.mac? && Hardware::CPU.arm?");
    expect(formula).toContain("elsif OS.mac?");
    expect(formula).toContain("elsif Hardware::CPU.arm?");
    expect(formula).not.toContain("on_macos do");
    expect(formula).not.toContain("on_arm do");
    expect(formula).not.toContain("on_intel do");
    expect(formula).toContain(
      "https://github.com/konvert7/klint/releases/download/v0.1.2/klint-0.1.2-darwin-arm64.tar.gz"
    );
    expect(formula).toContain(
      "https://github.com/konvert7/klint/releases/download/v0.1.2/klint-0.1.2-darwin-x64.tar.gz"
    );
    expect(formula).toContain(`sha256 "${"a".repeat(64)}"`);
    expect(formula).toContain(`sha256 "${"b".repeat(64)}"`);
    expect(formula).toContain(
      "https://github.com/konvert7/klint/releases/download/v0.1.2/klint-0.1.2-linux-arm64.tar.gz"
    );
    expect(formula).toContain(
      "https://github.com/konvert7/klint/releases/download/v0.1.2/klint-0.1.2-linux-x64.tar.gz"
    );
    expect(formula).toContain(`sha256 "${"c".repeat(64)}"`);
    expect(formula).toContain(`sha256 "${"d".repeat(64)}"`);
  });

  test("installs extracted klint binary and smokes version output", () => {
    const formula = homebrewFormula({
      version: "0.1.2",
      darwinArm64Sha: "a".repeat(64),
      darwinX64Sha: "b".repeat(64),
      linuxArm64Sha: "c".repeat(64),
      linuxX64Sha: "d".repeat(64),
    });

    expect(formula).toContain(
      'binary = File.exist?("klint") ? "klint" : Dir["klint-*/klint"].first'
    );
    expect(formula).toContain('bin.install binary => "klint"');
    expect(formula).toContain(
      'assert_match version.to_s, shell_output("#{bin}/klint --version")'
    );
  });
});
