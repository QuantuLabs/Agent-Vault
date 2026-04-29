# Agent Vault SDK Spec

## Package

The npm package name is `agent-vault`.

Description:

```text
TypeScript SDK for Agent Vault: 8004 identity creation and multi-wallet PDA management on Solana.
```

The SDK is standalone and imports the existing public `8004-solana` package for
8004 identity creation and registry PDA compatibility.

## Product Surface

The root client exposes two namespaces:

```ts
client.identities
client.wallets
```

`client.identities` is the 8004-aware surface. It delegates agent identity
creation to `8004-solana` and exposes PDA helpers for the 8004 AgentAccount.

Target API:

```ts
client.identities.create({ uri, atomEnabled, collectionPointer })
client.identities.getAgentAccountPda(agentAsset)
client.identities.requireIdentitySdk()
```

`client.wallets` is the Agent Vault surface. It builds instructions, derives
PDAs, lists wallets without an indexer, and verifies deployment metadata.

Target API:

```ts
client.wallets.getVault(agentAsset)
client.wallets.get(agentAsset, index)
client.wallets.list(agentAsset, { startIndex, limit, includeClosed })
client.wallets.listAll(agentAsset)
client.wallets.create(agentAsset, holder, { label })
client.wallets.buildInitVault(agentAsset, holder)
client.wallets.buildCreate(agentAsset, holder, { index, label })
client.wallets.buildUpdateLabel(agentAsset, holder, index, label)
client.wallets.buildDepositSol(agentAsset, index, funder, amount)
client.wallets.buildWithdrawSol(agentAsset, holder, index, amount, destination)
client.wallets.buildTransferSol(agentAsset, holder, fromIndex, toIndex, amount)
client.wallets.buildCreateAta(agentAsset, holder, index, mint, tokenProgram)
client.wallets.buildTransferSpl(agentAsset, holder, index, params)
client.wallets.buildWrapSol(agentAsset, holder, index, amount)
client.wallets.buildUnwrapSol(agentAsset, holder, index)
client.wallets.buildCloseAta(agentAsset, holder, index, mint, tokenProgram, rentReceiver)
client.wallets.buildExecuteCpiChecked(agentAsset, holder, index, params)
client.wallets.buildReopenForRecovery(agentAsset, holder, index, { label })
client.wallets.buildClose(agentAsset, holder, index, rentReceiver)
client.wallets.verifyDeployment()
```

## RPC Rules

Wallet listing must not call `getProgramAccounts`.

Default listing path:

```text
1 getAccountInfo(vault_config)
+ ceil(limit / 100) getMultipleAccountsInfo(wallet_pdas)
```

Token discovery is explicit and lazy. Default wallet listing does not scan token
accounts.

## Deployment Verification

The SDK accepts a release manifest with:

```ts
schema: "agent-vault.release-manifest.v0"
name: "Agent Vault"
```

Mainnet builders fail closed until canonical deployment verification is
implemented and passes. Devnet/localnet may warn for candidate manifests.

## Identity Creation

Identity creation is delegated to `8004-solana`:

```ts
const identity = new SolanaSDK({ cluster: "devnet", signer });
const client = new AgentVaultClient({ connection, identity });
await client.identities.create({ uri: "ipfs://..." });
```

The returned value normalizes the 8004 response into:

```ts
{
  agentAsset: PublicKey
  result: unknown
}
```

If no identity SDK is configured, identity creation fails with an actionable
error instead of silently building partial transactions.
