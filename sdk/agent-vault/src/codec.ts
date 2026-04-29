import { PublicKey } from "@solana/web3.js";
import { LABEL_LENGTH } from "./constants.js";
import type { PublicKeyish, U64Input } from "./types.js";

export function toPublicKey(value: PublicKeyish): PublicKey {
  return value instanceof PublicKey ? value : new PublicKey(value);
}

export function u16Le(value: number): Buffer {
  if (!Number.isInteger(value) || value < 0 || value > 0xffff) {
    throw new RangeError("u16 value out of range");
  }
  const out = Buffer.alloc(2);
  out.writeUInt16LE(value, 0);
  return out;
}

export function u32Le(value: number): Buffer {
  if (!Number.isInteger(value) || value < 0 || value > 0xffffffff) {
    throw new RangeError("u32 value out of range");
  }
  const out = Buffer.alloc(4);
  out.writeUInt32LE(value, 0);
  return out;
}

export function u64Le(value: U64Input): Buffer {
  const bigint = typeof value === "bigint" ? value : BigInt(value);
  if (bigint < 0n || bigint > 0xffff_ffff_ffff_ffffn) {
    throw new RangeError("u64 value out of range");
  }
  const out = Buffer.alloc(8);
  out.writeBigUInt64LE(bigint, 0);
  return out;
}

export function encodeLabel(label: string | Uint8Array = ""): Buffer {
  const bytes = typeof label === "string" ? Buffer.from(label, "utf8") : Buffer.from(label);
  if (bytes.length > LABEL_LENGTH) {
    throw new RangeError(`label must fit in ${LABEL_LENGTH} bytes`);
  }
  const out = Buffer.alloc(LABEL_LENGTH);
  bytes.copy(out);
  return out;
}

export function decodeLabel(bytes: Uint8Array): string {
  const nul = bytes.indexOf(0);
  const end = nul === -1 ? bytes.length : nul;
  return Buffer.from(bytes.slice(0, end)).toString("utf8");
}

export function concatData(tag: number, ...parts: Uint8Array[]): Buffer {
  return Buffer.concat([Buffer.from([tag]), ...parts.map((part) => Buffer.from(part))]);
}
