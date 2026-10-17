import {
  NETWORK_CONFIGS,
  getActiveNetwork,
  getNetworkConfig,
} from "../src/network";
import type { MuxContractIds } from "../src/types";
import { DEFAULT_ADDRESSES } from "../src/addresses-config";

// Type-level assertion: MuxContractIds and DEFAULT_ADDRESSES values
// must share exactly the same set of keys so the two public types
// cannot drift.
type Equals<A, B> =
  (<T>() => T extends A ? 1 : 2) extends (<T>() => T extends B ? 1 : 2)
    ? true
    : false;
type AddressKeys = keyof (typeof DEFAULT_ADDRESSES)["localnet"];
type ContractIdsKeys = keyof MuxContractIds;
type _AssertKeysMatch = Equals<AddressKeys, ContractIdsKeys>;
// If this line fails to compile, MuxContractIds and DEFAULT_ADDRESSES
// have diverged — add or remove the mismatched key in both places.
const _keysMatch: _AssertKeysMatch = true;

describe("NETWORK_CONFIGS", () => {
  it("defines localnet config", () => {
    expect(NETWORK_CONFIGS.localnet).toBeDefined();
    expect(NETWORK_CONFIGS.localnet.rpcUrl).toBeTruthy();
  });

  it("defines testnet config", () => {
    expect(NETWORK_CONFIGS.testnet).toBeDefined();
    expect(NETWORK_CONFIGS.testnet.rpcUrl).toContain("testnet");
  });

  it("defines mainnet config", () => {
    expect(NETWORK_CONFIGS.mainnet).toBeDefined();
    expect(NETWORK_CONFIGS.mainnet.rpcUrl).toContain("mainnet");
  });

  it("testnet and mainnet have different passphrases", () => {
    expect(NETWORK_CONFIGS.testnet.networkPassphrase).not.toEqual(
      NETWORK_CONFIGS.mainnet.networkPassphrase
    );
  });

  it("contract IDs are strings", () => {
    const { contracts } = NETWORK_CONFIGS.testnet;
    expect(typeof contracts.muxAccount).toBe("string");
    expect(typeof contracts.muxBatcher).toBe("string");
    expect(typeof contracts.muxPermissions).toBe("string");
  });

  it("all networks have RPC URLs", () => {
    Object.values(NETWORK_CONFIGS).forEach((config) => {
      expect(config.rpcUrl).toBeTruthy();
    });
  });

  it("all networks have network passphrases", () => {
    Object.values(NETWORK_CONFIGS).forEach((config) => {
      expect(config.networkPassphrase).toBeTruthy();
    });
  });

  it("MuxContractIds keys match DEFAULT_ADDRESSES keys", () => {
    const defaultKeys = Object.keys(DEFAULT_ADDRESSES.localnet).sort();
    const contractIdKeys: Array<keyof MuxContractIds> = [
      "muxAccount",
      "muxBatcher",
      "muxPermissions",
    ];
    const sortedKeys = [...contractIdKeys].sort();
    expect(defaultKeys).toEqual(sortedKeys);
  });
});

describe("Network selection", () => {
  it("getActiveNetwork returns default network", () => {
    const originalEnv = process.env.SOROBAN_NETWORK;
    delete process.env.SOROBAN_NETWORK;

    const network = getActiveNetwork();
    expect(network).toBe("localnet");

    if (originalEnv) {
      process.env.SOROBAN_NETWORK = originalEnv;
    }
  });

  it("getActiveNetwork respects SOROBAN_NETWORK env var", () => {
    const originalEnv = process.env.SOROBAN_NETWORK;
    process.env.SOROBAN_NETWORK = "testnet";

    const network = getActiveNetwork();
    expect(network).toBe("testnet");

    if (originalEnv) {
      process.env.SOROBAN_NETWORK = originalEnv;
    } else {
      delete process.env.SOROBAN_NETWORK;
    }
  });

  it("getNetworkConfig returns config for active network", () => {
    const originalEnv = process.env.SOROBAN_NETWORK;
    process.env.SOROBAN_NETWORK = "mainnet";

    const config = getNetworkConfig();
    expect(config.rpcUrl).toContain("mainnet");

    if (originalEnv) {
      process.env.SOROBAN_NETWORK = originalEnv;
    } else {
      delete process.env.SOROBAN_NETWORK;
    }
  });

  it("getNetworkConfig throws for unknown network", () => {
    const originalEnv = process.env.SOROBAN_NETWORK;
    process.env.SOROBAN_NETWORK = "unknown";

    expect(() => {
      getNetworkConfig();
    }).toThrow("not found");

    if (originalEnv) {
      process.env.SOROBAN_NETWORK = originalEnv;
    } else {
      delete process.env.SOROBAN_NETWORK;
    }
  });
});
