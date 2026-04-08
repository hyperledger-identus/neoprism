import { describe, test, expect } from "bun:test";
import { normalizeHex, hexToBytes } from "../src/hex";

describe("normalizeHex", () => {
  test("returns valid hex unchanged", () => {
    expect(normalizeHex("deadbeef")).toBe("deadbeef");
  });

  test("strips 0x prefix", () => {
    expect(normalizeHex("0xdeadbeef")).toBe("deadbeef");
  });

  test("accepts uppercase hex", () => {
    expect(normalizeHex("DEADBEEF")).toBe("DEADBEEF");
  });

  test("accepts mixed case hex", () => {
    expect(normalizeHex("DeAdBeEf")).toBe("DeAdBeEf");
  });

  test("accepts empty string", () => {
    expect(normalizeHex("")).toBe("");
  });

  test("accepts 0x alone as empty hex", () => {
    expect(normalizeHex("0x")).toBe("");
  });

  test("throws on non-hex characters", () => {
    expect(() => normalizeHex("notvalidhex!@#")).toThrow("contains non-hex characters");
  });

  test("throws on odd-length hex", () => {
    expect(() => normalizeHex("abc")).toThrow("odd length");
  });

  test("throws on odd-length hex with 0x prefix", () => {
    expect(() => normalizeHex("0xabc")).toThrow("odd length");
  });
});

describe("hexToBytes", () => {
  test("converts hex to bytes", () => {
    expect(hexToBytes("deadbeef")).toEqual(new Uint8Array([0xde, 0xad, 0xbe, 0xef]));
  });

  test("converts empty string to empty array", () => {
    expect(hexToBytes("")).toEqual(new Uint8Array([]));
  });

  test("converts single byte", () => {
    expect(hexToBytes("ff")).toEqual(new Uint8Array([255]));
  });
});
