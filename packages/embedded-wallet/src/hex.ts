/**
 * Strips an optional "0x" prefix, validates that the string contains only
 * hex characters and has even length, then returns the clean hex string.
 *
 * @throws {Error} if the input contains non-hex characters or has odd length
 */
export function normalizeHex(input: string): string {
  const hex = input.startsWith("0x") ? input.slice(2) : input;

  if (!/^[0-9a-fA-F]*$/.test(hex)) {
    throw new Error("Invalid hex string: contains non-hex characters");
  }

  if (hex.length % 2 !== 0) {
    throw new Error(`Invalid hex string: odd length (${hex.length} characters)`);
  }

  return hex;
}

/** Converts a hex string to Uint8Array. */
export function hexToBytes(hex: string): Uint8Array {
  const bytes = new Uint8Array(hex.length / 2);
  for (let i = 0; i < hex.length; i += 2) {
    bytes[i / 2] = Number.parseInt(hex.slice(i, i + 2), 16);
  }
  return bytes;
}
