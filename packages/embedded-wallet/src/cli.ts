import { Command } from "commander";
import { BuildOptions, isValidNetwork, Network, VALID_NETWORKS } from "./types";
import { buildTransaction } from "./transaction";

const VERSION = "0.0.1";

async function readMnemonicFromStdin(): Promise<string> {
  const text = await Bun.stdin.text();
  return text.trim();
}

async function buildCommand(options: BuildOptions): Promise<void> {
  let mnemonic: string[];

  if (options.mnemonicStdin) {
    try {
      const mnemonicText = await readMnemonicFromStdin();
      if (!mnemonicText) {
        console.error("error: failed to read mnemonic from stdin");
        process.exit(1);
      }
      mnemonic = mnemonicText.split(/\s+/);
      if (![12, 15, 18, 21, 24].includes(mnemonic.length)) {
        console.error(`error: invalid mnemonic length ${mnemonic.length}, expected 12, 15, 18, 21, or 24 words`);
        process.exit(1);
      }
    } catch (e) {
      console.error(`error: failed to read mnemonic from stdin: ${e}`);
      process.exit(1);
    }
  } else {
    console.error("error: --mnemonic-stdin is required");
    process.exit(1);
  }

  // Validate hex string
  const hex = options.prismObjectHex.startsWith("0x")
    ? options.prismObjectHex.slice(2)
    : options.prismObjectHex;
  if (!/^[0-9a-fA-F]*$/.test(hex)) {
    console.error("error: --prism-object-hex contains invalid hex characters");
    process.exit(1);
  }

  console.error(`info: building transaction for network=${options.network}`);

  try {
    const result = await buildTransaction({
      mnemonic,
      network: options.network,
      blockfrostUrl: options.blockfrostUrl,
      blockfrostApiKey: options.blockfrostApiKey,
      prismObjectHex: options.prismObjectHex,
    });

    console.log(result.cbor);
    process.exit(0);
  } catch (e) {
    console.error(`error: ${e}`);
    process.exit(1);
  }
}

const program = new Command();

program
  .name("embedded-wallet")
  .version(VERSION)
  .description("Cardano transaction builder for PRISM DID operations");

program
  .command("build")
  .description("Build a Cardano transaction from a PrismObject")
  .option("--blockfrost-url <url>", "Blockfrost API URL (for private instances)")
  .option("--blockfrost-api-key <key>", "Blockfrost API key (for public Blockfrost)")
  .option("--mnemonic-stdin", "Read mnemonic from stdin", false)
  .requiredOption("--prism-object-hex <hex>", "Hex-encoded PrismObject bytes (protobuf serialized)")
  .option(
    "--network <network>",
    `Network: ${VALID_NETWORKS.join(", ")}. Use 'custom' for custom testnets.`,
    "preview"
  )
  .action(async (rawOptions) => {
    // Validate mutual exclusion: exactly one of --blockfrost-url or --blockfrost-api-key must be provided
    if (!rawOptions.blockfrostUrl && !rawOptions.blockfrostApiKey) {
      console.error("error: either --blockfrost-url or --blockfrost-api-key is required");
      process.exit(1);
    }
    if (rawOptions.blockfrostUrl && rawOptions.blockfrostApiKey) {
      console.error("error: cannot use both --blockfrost-url and --blockfrost-api-key");
      process.exit(1);
    }

    if (!isValidNetwork(rawOptions.network)) {
      console.error(
        `error: invalid network "${rawOptions.network}", must be one of: ${VALID_NETWORKS.join(", ")}`
      );
      process.exit(1);
    }

    const options: BuildOptions = {
      blockfrostUrl: rawOptions.blockfrostUrl,
      blockfrostApiKey: rawOptions.blockfrostApiKey,
      mnemonicStdin: rawOptions.mnemonicStdin,
      prismObjectHex: rawOptions.prismObjectHex,
      network: rawOptions.network as Network,
    };

    await buildCommand(options);
  });

program.parse();
