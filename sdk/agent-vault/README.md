# Agent Vault SDK

TypeScript SDK for Agent Vault: 8004 identity creation and multi-wallet PDA
management on Solana.

```ts
import { AgentVaultClient } from "agent-vault";
import { SolanaSDK } from "8004-solana";

const identity = new SolanaSDK({ cluster: "devnet" });
const client = new AgentVaultClient({ connection, identity });

const created = await client.identities.create({ uri: "ipfs://..." });
const wallets = await client.wallets.list(created.agentAsset);
const createWalletIx = await client.wallets.create(created.agentAsset, payer, {
  label: "trading",
});
```

The SDK does not use an indexer for wallet listing. It reads `vault_config`, derives
wallet PDAs from `wallet_count`, and fetches them with `getMultipleAccountsInfo`.

This package is a WIP companion SDK for the Agent Vault program.
