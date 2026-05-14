# Security Policy

## Status

Agent Vault is a work in progress devnet candidate. It has not received an
external production audit and there is no mainnet release. Do not use this
program with valuable assets until a mainnet release, published release manifest,
and production security review are available.

## Reporting Vulnerabilities

Please report suspected vulnerabilities privately to Quantu Labs before opening
a public issue. Include:

- affected commit or release;
- exact program ID or cluster, if relevant;
- reproduction steps;
- expected impact;
- suggested fix, if known.

Do not include private keys, seed phrases, wallet backups, or production secrets
in any report.

## Supported Versions

Only the latest `main` branch and the published devnet candidate manifest are
currently supported for security review.

## Developer Safety Checklist

- Do not use Agent Vault with valuable assets while it is a devnet candidate.
- Keep SDK writes gated by deployment verification.
- Keep `allowUnverifiedDeployment` limited to localnet or devnet deployments you control.
- In docs and integrations, distinguish `agentAsset` from wallet PDA addresses and wallet indexes.
- Never request or store private keys, seed phrases, wallet backups, or production secrets.

## Release Expectations

Production releases should keep SDK writes fail-closed against a published
manifest that verifies the program account, ProgramData address, deployed byte
hash, byte size, upgrade authority policy, and global config fields.
