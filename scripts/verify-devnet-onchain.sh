#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$ROOT_DIR"

NODE_NO_WARNINGS=1 node <<'NODE'
const crypto = require('crypto');
const fs = require('fs');
const os = require('os');
const path = require('path');
const { execFileSync } = require('child_process');

const cluster = process.env.AGENT_VAULT_RPC_URL || 'devnet';
const manifest = JSON.parse(fs.readFileSync('docs/RELEASE_MANIFEST.devnet.json', 'utf8'));
const programId = manifest.program.id;

function solana(args, options = {}) {
  return execFileSync('solana', [...args, '-u', cluster], {
    encoding: options.encoding || 'utf8',
    stdio: options.stdio || ['ignore', 'pipe', 'pipe'],
  });
}

function assertEqual(name, actual, expected) {
  if (actual !== expected) {
    throw new Error(`${name} mismatch: expected ${expected}, got ${actual}`);
  }
}

function assertBytes(name, actual, expectedBase58) {
  const expected = base58Decode(expectedBase58);
  if (!Buffer.from(actual).equals(expected)) {
    throw new Error(`${name} mismatch: expected ${expectedBase58}, got ${base58Encode(actual)}`);
  }
}

function readU64LE(buffer, offset) {
  return Number(buffer.readBigUInt64LE(offset));
}

const ALPHABET = '123456789ABCDEFGHJKLMNPQRSTUVWXYZabcdefghijkmnopqrstuvwxyz';
const ALPHABET_MAP = new Map([...ALPHABET].map((char, index) => [char, index]));

function base58Decode(value) {
  let bytes = [0];
  for (const char of value) {
    const carryStart = ALPHABET_MAP.get(char);
    if (carryStart === undefined) throw new Error(`invalid base58 character ${char}`);
    let carry = carryStart;
    for (let i = 0; i < bytes.length; i += 1) {
      carry += bytes[i] * 58;
      bytes[i] = carry & 0xff;
      carry >>= 8;
    }
    while (carry > 0) {
      bytes.push(carry & 0xff);
      carry >>= 8;
    }
  }
  for (const char of value) {
    if (char !== '1') break;
    bytes.push(0);
  }
  return Buffer.from(bytes.reverse());
}

function base58Encode(buffer) {
  let digits = [0];
  for (const byte of buffer) {
    let carry = byte;
    for (let i = 0; i < digits.length; i += 1) {
      carry += digits[i] << 8;
      digits[i] = carry % 58;
      carry = Math.floor(carry / 58);
    }
    while (carry > 0) {
      digits.push(carry % 58);
      carry = Math.floor(carry / 58);
    }
  }
  for (const byte of buffer) {
    if (byte !== 0) break;
    digits.push(0);
  }
  return digits.reverse().map((digit) => ALPHABET[digit]).join('');
}

const program = JSON.parse(solana(['program', 'show', programId, '--output', 'json']));
assertEqual('programId', program.programId, programId);
assertEqual('program.owner', program.owner, 'BPFLoaderUpgradeab1e11111111111111111111111');
assertEqual(
  'programdataAddress',
  program.programdataAddress,
  manifest.deploymentVerification.programDataAddress,
);
assertEqual('upgradeAuthority', program.authority, manifest.deploymentVerification.upgradeAuthority);
assertEqual('programDataSizeBytes', program.dataLen, manifest.deploymentVerification.programDataSizeBytes);

const tmpDir = fs.mkdtempSync(path.join(os.tmpdir(), 'agent-vault-onchain-'));
const dumpedElf = path.join(tmpDir, 'agent_vault.so');
try {
  solana(['program', 'dump', programId, dumpedElf], { stdio: ['ignore', 'ignore', 'pipe'] });
  const elf = fs.readFileSync(dumpedElf);
  assertEqual('dumped ELF size', elf.length, manifest.program.sbfElfSizeBytes);
  assertEqual(
    'dumped ELF sha256',
    crypto.createHash('sha256').update(elf).digest('hex'),
    manifest.deploymentVerification.programDataSha256,
  );
} finally {
  fs.rmSync(tmpDir, { recursive: true, force: true });
}

const global = JSON.parse(
  solana(['account', manifest.program.globalConfigPda, '--output', 'json']),
).account;
assertEqual('global_config.owner', global.owner, programId);
assertEqual('global_config.space', global.space, 160);
const globalData = Buffer.from(global.data[0], 'base64');
assertEqual('global_config.data_len', globalData.length, 160);
assertEqual('global_config.discriminator', globalData.subarray(0, 8).toString('ascii'), 'AVGLBCFG');
assertEqual('global_config.version', globalData[8], 0);
assertEqual('global_config.bump', globalData[9], manifest.program.globalConfigBump);
assertBytes('global_config.initializer', globalData.subarray(10, 42), manifest.expectedGlobalConfig.initializer);
assertBytes('global_config.registryProgram', globalData.subarray(42, 74), manifest.expectedGlobalConfig.registryProgram);
assertBytes('global_config.collection', globalData.subarray(74, 106), manifest.expectedGlobalConfig.collection);
assertBytes('global_config.feeTreasury', globalData.subarray(106, 138), manifest.expectedGlobalConfig.feeTreasury);
assertEqual(
  'global_config.vaultActivationFeeLamports',
  readU64LE(globalData, 138),
  manifest.expectedGlobalConfig.vaultActivationFeeLamports,
);
if (!globalData.subarray(146).every((byte) => byte === 0)) {
  throw new Error('global_config reserved bytes are non-zero');
}

console.log(`devnet onchain deployment verified on ${cluster}: ${programId}`);
NODE
