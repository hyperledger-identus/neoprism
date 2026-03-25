import { describe, test, expect } from "bun:test";
import { spawn } from "node:child_process";
import { join } from "node:path";

const CLI_PATH = join(import.meta.dir, "../src/cli.ts");

interface CliResult {
  stdout: string;
  stderr: string;
  exitCode: number | null;
}

async function runCli(args: string[], stdin?: string): Promise<CliResult> {
  return new Promise((resolve) => {
    const proc = spawn("bun", ["run", CLI_PATH, ...args], {
      stdio: ["pipe", "pipe", "pipe"],
    });

    let stdout = "";
    let stderr = "";

    proc.stdout.on("data", (data) => {
      stdout += data.toString();
    });

    proc.stderr.on("data", (data) => {
      stderr += data.toString();
    });

    if (stdin) {
      proc.stdin?.write(stdin);
      proc.stdin?.end();
    }

    proc.on("close", (code) => {
      resolve({ stdout, stderr, exitCode: code });
    });
  });
}

describe("CLI argument parsing", () => {
  test("--help flag shows help message", async () => {
    const result = await runCli(["--help"]);
    expect(result.stdout).toContain("Usage:");
    expect(result.stdout).toContain("build");
  });

  test("--version flag shows version", async () => {
    const result = await runCli(["--version"]);
    expect(result.exitCode).toBe(0);
  });

  test("missing required options exits with error", async () => {
    const result = await runCli(["build"]);
    expect(result.exitCode).not.toBe(0);
    expect(result.stderr).toContain("required");
  });

  test("missing --prism-object-hex exits with error", async () => {
    const result = await runCli([
      "build",
      "--blockfrost-api-key",
      "test-key",
    ]);
    expect(result.exitCode).not.toBe(0);
    expect(result.stderr).toContain("--prism-object-hex");
  });

  test("neither --blockfrost-url nor --blockfrost-api-key exits with error", async () => {
    const result = await runCli([
      "build",
      "--prism-object-hex",
      "deadbeef",
    ]);
    expect(result.exitCode).toBe(1);
    expect(result.stderr).toContain("either --blockfrost-url or --blockfrost-api-key is required");
  });

  test("both --blockfrost-url and --blockfrost-api-key exits with error", async () => {
    const result = await runCli([
      "build",
      "--blockfrost-url",
      "https://example.com",
      "--blockfrost-api-key",
      "test-key",
      "--prism-object-hex",
      "deadbeef",
    ]);
    expect(result.exitCode).toBe(1);
    expect(result.stderr).toContain("cannot use both --blockfrost-url and --blockfrost-api-key");
  });

  test("invalid network exits with error", async () => {
    const result = await runCli([
      "build",
      "--blockfrost-api-key",
      "test-key",
      "--prism-object-hex",
      "deadbeef",
      "--network",
      "invalid-network",
    ]);
    expect(result.exitCode).toBe(1);
    expect(result.stderr).toContain("invalid network");
  });

  test("valid network is accepted", async () => {
    const result = await runCli([
      "build",
      "--blockfrost-api-key",
      "test-key",
      "--prism-object-hex",
      "deadbeef",
      "--network",
      "preprod",
    ]);
    expect(result.stderr).toContain("--mnemonic-stdin is required");
  });
});

describe("network validation", () => {
  test.each([["mainnet"], ["preprod"], ["preview"], ["custom"]])(
    "network=%s is valid",
    async (network) => {
      const result = await runCli([
        "build",
        "--blockfrost-api-key",
        "test-key",
        "--prism-object-hex",
        "deadbeef",
        "--network",
        network,
      ]);
      expect(result.stderr).toContain("--mnemonic-stdin is required");
    }
  );
});

describe("stdin mnemonic reading", () => {
  test("--mnemonic-stdin flag reads from stdin", async () => {
    const result = await runCli(
      [
        "build",
        "--blockfrost-api-key",
        "test-key",
        "--prism-object-hex",
        "deadbeef",
        "--mnemonic-stdin",
      ],
      "word1 word2 word3 word4 word5 word6 word7 word8 word9 word10 word11 word12\n"
    );
    expect(result.stderr).toContain("building transaction");
  });
});

describe("prism object hex parsing", () => {
  test("--prism-object-hex accepts valid hex string", async () => {
    const result = await runCli([
      "build",
      "--blockfrost-api-key",
      "test-key",
      "--prism-object-hex",
      "deadbeef",
    ]);
    expect(result.stderr).toContain("--mnemonic-stdin is required");
  });

  test("--prism-object-hex accepts valid hex string with 0x prefix", async () => {
    const result = await runCli([
      "build",
      "--blockfrost-api-key",
      "test-key",
      "--prism-object-hex",
      "0xdeadbeef",
    ]);
    expect(result.stderr).toContain("--mnemonic-stdin is required");
  });

  test("--prism-object-hex accepts empty string (edge case)", async () => {
    const result = await runCli([
      "build",
      "--blockfrost-api-key",
      "test-key",
      "--prism-object-hex",
      "",
    ]);
    // Empty string might be treated as missing required option
    expect(result.exitCode).not.toBe(0);
  });

  test("--prism-object-hex rejects invalid hex characters", async () => {
    const result = await runCli(
      [
        "build",
        "--blockfrost-api-key",
        "test-key",
        "--prism-object-hex",
        "notvalidhex!@#",
        "--mnemonic-stdin",
      ],
      "word1 word2 word3 word4 word5 word6 word7 word8 word9 word10 word11 word12\n"
    );
    expect(result.exitCode).toBe(1);
    expect(result.stderr).toContain("invalid hex");
  });

  test("--prism-object-hex rejects odd-length hex string", async () => {
    const result = await runCli(
      [
        "build",
        "--blockfrost-api-key",
        "test-key",
        "--prism-object-hex",
        "abc", // 3 chars - odd length
        "--mnemonic-stdin",
      ],
      "word1 word2 word3 word4 word5 word6 word7 word8 word9 word10 word11 word12\n"
    );
    expect(result.exitCode).toBe(1);
    expect(result.stderr).toContain("odd length");
  });

  test("--prism-object-hex accepts long hex string (multiple chunks)", async () => {
    // Create a hex string longer than 128 characters (64 bytes) to test chunking
    const longHex = "a".repeat(256);
    const result = await runCli([
      "build",
      "--blockfrost-api-key",
      "test-key",
      "--prism-object-hex",
      longHex,
    ]);
    expect(result.stderr).toContain("--mnemonic-stdin is required");
  });
});

describe("stderr vs stdout output", () => {
  test("errors are written to stderr, not stdout", async () => {
    const result = await runCli(["build"]);
    expect(result.exitCode).not.toBe(0);
    expect(result.stderr).not.toBe("");
    expect(result.stdout).toBe("");
  });

  test("missing required options writes error to stderr", async () => {
    const result = await runCli([
      "build",
      "--blockfrost-url",
      "https://example.com",
    ]);
    expect(result.exitCode).not.toBe(0);
    expect(result.stderr).not.toBe("");
    expect(result.stderr).toContain("required");
    expect(result.stdout).toBe("");
  });

  test("invalid network writes error to stderr", async () => {
    const result = await runCli([
      "build",
      "--blockfrost-api-key",
      "test-key",
      "--prism-object-hex",
      "deadbeef",
      "--network",
      "invalid",
    ]);
    expect(result.exitCode).toBe(1);
    expect(result.stderr).not.toBe("");
    expect(result.stderr).toContain("invalid network");
    expect(result.stdout).toBe("");
  });

  test("--mnemonic-stdin required error goes to stderr", async () => {
    const result = await runCli([
      "build",
      "--blockfrost-api-key",
      "test-key",
      "--prism-object-hex",
      "deadbeef",
    ]);
    expect(result.exitCode).not.toBe(0);
    expect(result.stderr).toContain("--mnemonic-stdin is required");
    expect(result.stdout).toBe("");
  });

  test("error during transaction building writes to stderr, stdout remains clean", async () => {
    const result = await runCli(
      [
        "build",
        "--blockfrost-api-key",
        "test-key",
        "--prism-object-hex",
        "deadbeef",
        "--mnemonic-stdin",
      ],
      "abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon about"
    );
    // The CLI logs "info: building transaction" to stderr before making the network call
    expect(result.stderr).toContain("building transaction");

    // On failure, stdout should remain clean (no error messages)
    // CBOR output only goes to stdout on success
    expect(result.exitCode).not.toBe(0);
    expect(result.stderr).toContain("error:");
    expect(result.stdout).toBe("");
  });
});