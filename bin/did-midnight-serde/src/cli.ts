import { decodeContractState } from './index';

function usageAndExit() {
  console.error('Usage: did-midnight-serde <did> <network> <hex>');
  process.exit(2);
}

function main(argv: string[] = process.argv) {
  const [, , didArg, networkArg, hexArg] = argv;
  if (!didArg || !networkArg || !hexArg) usageAndExit();

  const networkId = Number.parseInt(networkArg, 10);
  if (!Number.isFinite(networkId) || Number.isNaN(networkId)) {
    console.error(`Invalid network id: '${networkArg}'. Must be an integer.`);
    usageAndExit();
  }

  try {
    const result = decodeContractState(didArg, networkId, hexArg);
    console.log(JSON.stringify(result, null, 2));
  } catch (err) {
    console.error('Error decoding contract state:', err instanceof Error ? err.message : err);
    process.exit(1);
  }
}

main();
