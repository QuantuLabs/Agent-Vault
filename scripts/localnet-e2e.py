#!/usr/bin/env python3
import base64
import json
import os
import shutil
import signal
import subprocess
import sys
import time
import urllib.request
from pathlib import Path

from solders.hash import Hash
from solders.instruction import AccountMeta, Instruction
from solders.keypair import Keypair
from solders.message import Message
from solders.pubkey import Pubkey
from solders.transaction import Transaction


ROOT = Path(__file__).resolve().parents[1]
TMP = ROOT / ".tmp" / "localnet-e2e"
FIXTURES = TMP / "fixtures"
RPC_URL = "http://127.0.0.1:8899"
RESTORE_AGENT_VAULT_SO = TMP / "restore-agent_vault.so"
RESTORE_MOCK_AMM_SO = TMP / "restore-mock_amm.so"

PROGRAM_ID = Pubkey.from_string("36u7KMBuxjExvU6V2nfTX5SnNdYMGUupFiYouLzrgpfW")
MOCK_AMM_PROGRAM = Pubkey(bytes([7]) * 32)
SYSTEM_PROGRAM = Pubkey.from_string("11111111111111111111111111111111")
CLOCK_SYSVAR = Pubkey.from_string("SysvarC1ock11111111111111111111111111111111")
RENT_SYSVAR = Pubkey.from_string("SysvarRent111111111111111111111111111111111")
TOKEN_PROGRAM = Pubkey.from_string("TokenkegQfeZyiNwAJbNbGKPFXCWuBvf9Ss623VQ5DA")
ASSOCIATED_TOKEN_PROGRAM = Pubkey.from_string("ATokenGPvbdGVxr1b2hvZbsiqW5xWH25efTNsLJA8knL")
METAPLEX_CORE_PROGRAM = Pubkey.from_string("CoREENxT6tW1HoK8ypY1SxRMZTcVPm7R94rH4PZNhX7d")
NATIVE_MINT = Pubkey.from_string("So11111111111111111111111111111111111111112")

INITIALIZER = Keypair.from_seed(bytes([1]) * 32)
FEE_TREASURY = Keypair.from_seed(bytes([2]) * 32).pubkey()
COLLECTION = Keypair.from_seed(bytes([3]) * 32).pubkey()
AGENT_ASSET = Keypair.from_seed(bytes([4]) * 32).pubkey()
RECIPIENT = Keypair.from_seed(bytes([5]) * 32).pubkey()
REGISTRY_PROGRAM = Keypair.from_seed(bytes([9]) * 32).pubkey()

MINT_CREATE_ATA = Keypair.from_seed(bytes([6]) * 32).pubkey()
MINT_TRANSFER = Keypair.from_seed(bytes([10]) * 32).pubkey()
MINT_SWAP_IN = Keypair.from_seed(bytes([11]) * 32).pubkey()
MINT_SWAP_OUT = Keypair.from_seed(bytes([12]) * 32).pubkey()
TRANSFER_DEST = Keypair.from_seed(bytes([13]) * 32).pubkey()
POOL_INPUT = Keypair.from_seed(bytes([14]) * 32).pubkey()
SWAP_USER_OUTPUT = Keypair.from_seed(bytes([15]) * 32).pubkey()
POOL_AUTHORITY = Keypair.from_seed(bytes([16]) * 32).pubkey()

ACTIVATION_FEE = 500_000
LABEL_LEN = 16
TOKEN_MINT_LEN = 82
TOKEN_ACCOUNT_LEN = 165
CORE_ASSET_MIN_LEN = 66
CORE_ASSET_V1_KEY = 1
CORE_ASSET_OWNER_OFFSET = 1
CORE_ASSET_COLLECTION_TAG_OFFSET = 33
CORE_ASSET_COLLECTION_OFFSET = 34
CORE_ASSET_COLLECTION_TAG = 2
AGENT_ACCOUNT_MIN_LEN = 137
AGENT_ACCOUNT_DISCRIMINATOR = bytes([241, 119, 69, 140, 233, 9, 112, 50])
AGENT_ACCOUNT_COLLECTION_OFFSET = 8
AGENT_ACCOUNT_CREATOR_OFFSET = 40
AGENT_ACCOUNT_OWNER_OFFSET = 72
AGENT_ACCOUNT_ASSET_OFFSET = 104
AGENT_ACCOUNT_BUMP_OFFSET = 136

TAG_INITIALIZE_GLOBAL_CONFIG = 0
TAG_INIT_VAULT_CONFIG = 1
TAG_CREATE_WALLET = 2
TAG_UPDATE_WALLET_LABEL = 3
TAG_DEPOSIT_SOL = 4
TAG_WITHDRAW_SOL = 5
TAG_TRANSFER_SOL = 6
TAG_CLOSE_WALLET = 7
TAG_CREATE_WALLET_ATA = 32
TAG_TRANSFER_SPL = 33
TAG_WRAP_SOL = 34
TAG_UNWRAP_SOL = 35
TAG_CLOSE_WALLET_ATA = 36
TAG_EXECUTE_CPI_CHECKED = 64


def run(cmd):
    print("+ " + " ".join(cmd), flush=True)
    env = os.environ.copy()
    env["NO_DNA"] = "1"
    subprocess.run(cmd, cwd=ROOT, env=env, check=True)


def rpc(method, params=None, timeout=20):
    payload = json.dumps({"jsonrpc": "2.0", "id": 1, "method": method, "params": params or []})
    request = urllib.request.Request(
        RPC_URL,
        data=payload.encode(),
        headers={"content-type": "application/json"},
        method="POST",
    )
    with urllib.request.urlopen(request, timeout=timeout) as response:
        out = json.loads(response.read().decode())
    if "error" in out:
        raise RuntimeError(f"RPC {method} failed: {out['error']}")
    return out["result"]


def wait_for_rpc():
    deadline = time.time() + 30
    while time.time() < deadline:
        try:
            rpc("getHealth", timeout=2)
            return
        except Exception:
            time.sleep(0.25)
    raise TimeoutError("local validator RPC did not become ready")


def request_airdrop(pubkey, lamports):
    signature = rpc("requestAirdrop", [str(pubkey), lamports])
    deadline = time.time() + 20
    while time.time() < deadline:
        status = rpc("getSignatureStatuses", [[signature]])["value"][0]
        if status and status.get("confirmationStatus") in ("confirmed", "finalized"):
            return signature
        time.sleep(0.25)
    raise TimeoutError(f"airdrop did not confirm: {signature}")


def get_balance(pubkey):
    return rpc("getBalance", [str(pubkey), {"commitment": "confirmed"}])["value"]


def get_account(pubkey):
    value = rpc("getAccountInfo", [str(pubkey), {"encoding": "base64", "commitment": "confirmed"}])[
        "value"
    ]
    if value is None:
        return None
    data = base64.b64decode(value["data"][0])
    return {"lamports": value["lamports"], "owner": value["owner"], "data": data}


def send_ixs(ixs, signers=None):
    signers = signers or [INITIALIZER]
    latest = rpc("getLatestBlockhash")["value"]["blockhash"]
    blockhash = Hash.from_string(latest)
    message = Message.new_with_blockhash(ixs, INITIALIZER.pubkey(), blockhash)
    tx = Transaction(signers, message, blockhash)
    encoded = base64.b64encode(bytes(tx)).decode()
    signature = rpc(
        "sendTransaction",
        [encoded, {"encoding": "base64", "skipPreflight": False, "preflightCommitment": "confirmed"}],
        timeout=30,
    )
    deadline = time.time() + 30
    while time.time() < deadline:
        status = rpc("getSignatureStatuses", [[signature]])["value"][0]
        if status:
            if status.get("err") is not None:
                raise RuntimeError(f"transaction failed {signature}: {status['err']}")
            if status.get("confirmationStatus") in ("confirmed", "finalized"):
                return signature
        time.sleep(0.25)
    raise TimeoutError(f"transaction did not confirm: {signature}")


def pda(seeds, program_id=PROGRAM_ID):
    return Pubkey.find_program_address(seeds, program_id)[0]


def global_config_pda():
    return pda([b"global_config"])


def vault_config_pda(agent_asset):
    return pda([b"vault_config", bytes(agent_asset)])


def wallet_pda(agent_asset, index):
    return pda([b"agent_vault", bytes(agent_asset), index.to_bytes(2, "little")])


def ata(owner, mint, token_program=TOKEN_PROGRAM):
    return Pubkey.find_program_address(
        [bytes(owner), bytes(token_program), bytes(mint)],
        ASSOCIATED_TOKEN_PROGRAM,
    )[0]


def registry_agent_pda(agent_asset):
    return Pubkey.find_program_address([b"agent", bytes(agent_asset)], REGISTRY_PROGRAM)


def am(pubkey, signer=False, writable=False):
    return AccountMeta(pubkey, signer, writable)


def u16(value):
    return value.to_bytes(2, "little")


def u64(value):
    return value.to_bytes(8, "little")


def ix_initialize_global_config():
    data = bytes([TAG_INITIALIZE_GLOBAL_CONFIG])
    data += bytes(REGISTRY_PROGRAM)
    data += bytes(COLLECTION)
    data += bytes(FEE_TREASURY)
    data += u64(ACTIVATION_FEE)
    return Instruction(
        PROGRAM_ID,
        data,
        [am(INITIALIZER.pubkey(), True, True), am(global_config_pda(), False, True), am(SYSTEM_PROGRAM)],
    )


def ix_init_vault_config(agent_account):
    return Instruction(
        PROGRAM_ID,
        bytes([TAG_INIT_VAULT_CONFIG]),
        [
            am(INITIALIZER.pubkey(), True, True),
            am(global_config_pda()),
            am(vault_config_pda(AGENT_ASSET), writable=True),
            am(AGENT_ASSET),
            am(agent_account),
            am(FEE_TREASURY, writable=True),
            am(CLOCK_SYSVAR),
            am(SYSTEM_PROGRAM),
        ],
    )


def ix_create_wallet(index, label):
    data = bytes([TAG_CREATE_WALLET]) + label.encode()[:LABEL_LEN].ljust(LABEL_LEN, b"\0")
    return Instruction(
        PROGRAM_ID,
        data,
        [
            am(INITIALIZER.pubkey(), True, True),
            am(vault_config_pda(AGENT_ASSET), writable=True),
            am(wallet_pda(AGENT_ASSET, index), writable=True),
            am(AGENT_ASSET),
            am(SYSTEM_PROGRAM),
        ],
    )


def ix_update_wallet_label(index, label):
    data = bytes([TAG_UPDATE_WALLET_LABEL]) + u16(index) + label.encode()[:LABEL_LEN].ljust(LABEL_LEN, b"\0")
    return Instruction(
        PROGRAM_ID,
        data,
        [
            am(INITIALIZER.pubkey(), True),
            am(global_config_pda()),
            am(vault_config_pda(AGENT_ASSET)),
            am(wallet_pda(AGENT_ASSET, index), writable=True),
            am(AGENT_ASSET),
        ],
    )


def ix_deposit_sol(index, amount):
    return Instruction(
        PROGRAM_ID,
        bytes([TAG_DEPOSIT_SOL]) + u64(amount),
        [
            am(INITIALIZER.pubkey(), True, True),
            am(wallet_pda(AGENT_ASSET, index), writable=True),
            am(AGENT_ASSET),
            am(SYSTEM_PROGRAM),
        ],
    )


def ix_withdraw_sol(index, destination, amount):
    return Instruction(
        PROGRAM_ID,
        bytes([TAG_WITHDRAW_SOL]) + u16(index) + u64(amount),
        [
            am(INITIALIZER.pubkey(), True, True),
            am(wallet_pda(AGENT_ASSET, index), writable=True),
            am(destination, writable=True),
            am(AGENT_ASSET),
            am(RENT_SYSVAR),
        ],
    )


def ix_transfer_sol(from_index, to_index, amount):
    return Instruction(
        PROGRAM_ID,
        bytes([TAG_TRANSFER_SOL]) + u16(from_index) + u16(to_index) + u64(amount),
        [
            am(INITIALIZER.pubkey(), True),
            am(wallet_pda(AGENT_ASSET, from_index), writable=True),
            am(wallet_pda(AGENT_ASSET, to_index), writable=True),
            am(AGENT_ASSET),
            am(RENT_SYSVAR),
        ],
    )


def ix_close_wallet(index, rent_receiver):
    return Instruction(
        PROGRAM_ID,
        bytes([TAG_CLOSE_WALLET]),
        [
            am(INITIALIZER.pubkey(), True, True),
            am(global_config_pda()),
            am(vault_config_pda(AGENT_ASSET)),
            am(wallet_pda(AGENT_ASSET, index), writable=True),
            am(rent_receiver, writable=True),
            am(AGENT_ASSET),
            am(RENT_SYSVAR),
        ],
    )


def ix_create_wallet_ata(index, mint, token_program=TOKEN_PROGRAM, kind=0):
    wallet = wallet_pda(AGENT_ASSET, index)
    return Instruction(
        PROGRAM_ID,
        bytes([TAG_CREATE_WALLET_ATA]) + u16(index) + bytes([kind]),
        [
            am(INITIALIZER.pubkey(), True, True),
            am(global_config_pda()),
            am(vault_config_pda(AGENT_ASSET)),
            am(wallet),
            am(AGENT_ASSET),
            am(mint),
            am(ata(wallet, mint, token_program), writable=True),
            am(ASSOCIATED_TOKEN_PROGRAM),
            am(token_program),
            am(SYSTEM_PROGRAM),
        ],
    )


def ix_transfer_spl(index, mint, source, destination, amount, decimals=6):
    return Instruction(
        PROGRAM_ID,
        bytes([TAG_TRANSFER_SPL]) + u16(index) + u64(amount) + bytes([decimals]) + u64(0),
        [
            am(INITIALIZER.pubkey(), True),
            am(global_config_pda()),
            am(vault_config_pda(AGENT_ASSET)),
            am(wallet_pda(AGENT_ASSET, index)),
            am(AGENT_ASSET),
            am(mint),
            am(source, writable=True),
            am(destination, writable=True),
            am(TOKEN_PROGRAM),
        ],
    )


def ix_close_wallet_ata(index, mint, rent_receiver):
    wallet = wallet_pda(AGENT_ASSET, index)
    return Instruction(
        PROGRAM_ID,
        bytes([TAG_CLOSE_WALLET_ATA]) + u16(index),
        [
            am(INITIALIZER.pubkey(), True),
            am(global_config_pda()),
            am(vault_config_pda(AGENT_ASSET)),
            am(wallet),
            am(AGENT_ASSET),
            am(mint),
            am(ata(wallet, mint), writable=True),
            am(rent_receiver, writable=True),
            am(ASSOCIATED_TOKEN_PROGRAM),
            am(TOKEN_PROGRAM),
        ],
    )


def ix_wrap_sol(index, amount):
    wallet = wallet_pda(AGENT_ASSET, index)
    return Instruction(
        PROGRAM_ID,
        bytes([TAG_WRAP_SOL]) + u16(index) + u64(amount),
        [
            am(INITIALIZER.pubkey(), True, True),
            am(global_config_pda()),
            am(vault_config_pda(AGENT_ASSET)),
            am(wallet, writable=True),
            am(AGENT_ASSET),
            am(ata(wallet, NATIVE_MINT), writable=True),
            am(NATIVE_MINT),
            am(TOKEN_PROGRAM),
            am(RENT_SYSVAR),
        ],
    )


def ix_sync_native(wsol_ata):
    return Instruction(TOKEN_PROGRAM, bytes([17]), [am(wsol_ata, writable=True)])


def ix_unwrap_sol(index):
    wallet = wallet_pda(AGENT_ASSET, index)
    return Instruction(
        PROGRAM_ID,
        bytes([TAG_UNWRAP_SOL]) + u16(index),
        [
            am(INITIALIZER.pubkey(), True),
            am(global_config_pda()),
            am(vault_config_pda(AGENT_ASSET)),
            am(wallet, writable=True),
            am(AGENT_ASSET),
            am(ata(wallet, NATIVE_MINT), writable=True),
            am(TOKEN_PROGRAM),
        ],
    )


def ix_execute_cpi_checked_mock_swap(amount_in, max_input, amount_out, min_output):
    wallet = wallet_pda(AGENT_ASSET, 0)
    user_input = ata(wallet, MINT_SWAP_IN)
    pool_output = ata(wallet, MINT_SWAP_OUT)
    target_data = u64(amount_in) + u64(amount_out) + bytes([6, 6])
    data = bytes([TAG_EXECUTE_CPI_CHECKED]) + u16(0) + bytes([0, 7]) + u16(len(target_data)) + target_data
    data += bytes([5])
    data += bytes([7, 1, 5]) + bytes(MINT_SWAP_IN) + u64(max_input)
    data += bytes([9, 1, 5])
    data += bytes([7, 3, 6]) + bytes(MINT_SWAP_OUT) + u64(amount_out)
    data += bytes([9, 3, 6])
    data += bytes([6, 4, 6]) + bytes(MINT_SWAP_OUT) + u64(min_output)
    return Instruction(
        PROGRAM_ID,
        data,
        [
            am(INITIALIZER.pubkey(), True),
            am(global_config_pda()),
            am(vault_config_pda(AGENT_ASSET)),
            am(wallet),
            am(AGENT_ASSET),
            am(MOCK_AMM_PROGRAM),
            am(user_input, writable=True),
            am(POOL_INPUT, writable=True),
            am(pool_output, writable=True),
            am(SWAP_USER_OUTPUT, writable=True),
            am(MINT_SWAP_IN),
            am(MINT_SWAP_OUT),
            am(TOKEN_PROGRAM),
        ],
    )


def token_mint_data(decimals=6):
    data = bytearray(TOKEN_MINT_LEN)
    data[44] = decimals
    data[45] = 1
    return bytes(data)


def token_account_data(mint, authority, amount):
    data = bytearray(TOKEN_ACCOUNT_LEN)
    data[0:32] = bytes(mint)
    data[32:64] = bytes(authority)
    data[64:72] = u64(amount)
    data[108] = 1
    return bytes(data)


def core_asset_data():
    data = bytearray(CORE_ASSET_MIN_LEN)
    data[0] = CORE_ASSET_V1_KEY
    data[CORE_ASSET_OWNER_OFFSET : CORE_ASSET_OWNER_OFFSET + 32] = bytes(INITIALIZER.pubkey())
    data[CORE_ASSET_COLLECTION_TAG_OFFSET] = CORE_ASSET_COLLECTION_TAG
    data[CORE_ASSET_COLLECTION_OFFSET : CORE_ASSET_COLLECTION_OFFSET + 32] = bytes(COLLECTION)
    return bytes(data)


def agent_account_data(agent_account_bump):
    data = bytearray(AGENT_ACCOUNT_MIN_LEN)
    data[0:8] = AGENT_ACCOUNT_DISCRIMINATOR
    data[AGENT_ACCOUNT_COLLECTION_OFFSET : AGENT_ACCOUNT_COLLECTION_OFFSET + 32] = bytes(COLLECTION)
    data[AGENT_ACCOUNT_CREATOR_OFFSET : AGENT_ACCOUNT_CREATOR_OFFSET + 32] = bytes(INITIALIZER.pubkey())
    data[AGENT_ACCOUNT_OWNER_OFFSET : AGENT_ACCOUNT_OWNER_OFFSET + 32] = bytes(INITIALIZER.pubkey())
    data[AGENT_ACCOUNT_ASSET_OFFSET : AGENT_ACCOUNT_ASSET_OFFSET + 32] = bytes(AGENT_ASSET)
    data[AGENT_ACCOUNT_BUMP_OFFSET] = agent_account_bump
    return bytes(data)


def write_account_fixture(path, pubkey, lamports, owner, data=b"", executable=False):
    payload = {
        "pubkey": str(pubkey),
        "account": {
            "lamports": lamports,
            "data": [base64.b64encode(data).decode(), "base64"],
            "owner": str(owner),
            "executable": executable,
            "rentEpoch": 0,
            "space": len(data),
        },
    }
    path.write_text(json.dumps(payload, indent=2))


def write_fixtures():
    if TMP.exists():
        shutil.rmtree(TMP)
    FIXTURES.mkdir(parents=True)
    agent_account, bump = registry_agent_pda(AGENT_ASSET)
    wallet0 = wallet_pda(AGENT_ASSET, 0)
    transfer_source = ata(wallet0, MINT_TRANSFER)
    swap_input = ata(wallet0, MINT_SWAP_IN)
    swap_pool_output = ata(wallet0, MINT_SWAP_OUT)

    fixtures = [
        ("agent-asset.json", AGENT_ASSET, 1_000_000, METAPLEX_CORE_PROGRAM, core_asset_data()),
        ("agent-account.json", agent_account, 1_000_000, REGISTRY_PROGRAM, agent_account_data(bump)),
    ]
    for idx, mint in enumerate([MINT_CREATE_ATA, MINT_TRANSFER, MINT_SWAP_IN, MINT_SWAP_OUT], start=1):
        fixtures.append((f"mint-{idx}.json", mint, 1_000_000, TOKEN_PROGRAM, token_mint_data(6)))
    fixtures.extend(
        [
            (
                "transfer-source.json",
                transfer_source,
                2_039_280,
                TOKEN_PROGRAM,
                token_account_data(MINT_TRANSFER, wallet0, 1_000),
            ),
            (
                "transfer-destination.json",
                TRANSFER_DEST,
                2_039_280,
                TOKEN_PROGRAM,
                token_account_data(MINT_TRANSFER, RECIPIENT, 0),
            ),
            (
                "swap-input.json",
                swap_input,
                2_039_280,
                TOKEN_PROGRAM,
                token_account_data(MINT_SWAP_IN, wallet0, 1_000),
            ),
            (
                "swap-pool-output.json",
                swap_pool_output,
                2_039_280,
                TOKEN_PROGRAM,
                token_account_data(MINT_SWAP_OUT, wallet0, 500),
            ),
            (
                "pool-input.json",
                POOL_INPUT,
                2_039_280,
                TOKEN_PROGRAM,
                token_account_data(MINT_SWAP_IN, POOL_AUTHORITY, 0),
            ),
            (
                "swap-user-output.json",
                SWAP_USER_OUTPUT,
                2_039_280,
                TOKEN_PROGRAM,
                token_account_data(MINT_SWAP_OUT, POOL_AUTHORITY, 0),
            ),
        ]
    )
    for name, pubkey, lamports, owner, data in fixtures:
        write_account_fixture(FIXTURES / name, pubkey, lamports, owner, data)
    return agent_account


def start_validator():
    args = [
        "solana-test-validator",
        "--reset",
        "--quiet",
        "--ledger",
        str(TMP / "ledger"),
        "--rpc-port",
        "8899",
        "--bpf-program",
        str(PROGRAM_ID),
        str(TMP / "agent_vault_localnet.so"),
        "--bpf-program",
        str(MOCK_AMM_PROGRAM),
        str(TMP / "mock_amm.so"),
    ]
    for fixture in sorted(FIXTURES.glob("*.json")):
        payload = json.loads(fixture.read_text())
        args.extend(["--account", payload["pubkey"], str(fixture)])
    process = subprocess.Popen(args, cwd=ROOT)
    try:
        wait_for_rpc()
    except Exception:
        process.terminate()
        raise
    return process


def token_amount(pubkey):
    account = get_account(pubkey)
    if account is None:
        raise AssertionError(f"missing token account {pubkey}")
    return int.from_bytes(account["data"][64:72], "little")


def require(condition, message):
    if not condition:
        raise AssertionError(message)


def build_artifacts():
    run(
        [
            "cargo",
            "build-sbf",
            "--manifest-path",
            "programs/agent-vault/Cargo.toml",
            "--no-default-features",
            "--features",
            "localnet",
        ]
    )
    shutil.copy2(ROOT / "target" / "deploy" / "agent_vault.so", TMP / "agent_vault_localnet.so")
    run(["cargo", "build-sbf", "--manifest-path", "programs/mock-amm/Cargo.toml"])
    shutil.copy2(ROOT / "target" / "deploy" / "mock_amm.so", TMP / "mock_amm.so")


def save_deploy_artifacts():
    agent_vault_so = ROOT / "target" / "deploy" / "agent_vault.so"
    mock_amm_so = ROOT / "target" / "deploy" / "mock_amm.so"
    if agent_vault_so.exists():
        shutil.copy2(agent_vault_so, RESTORE_AGENT_VAULT_SO)
    if mock_amm_so.exists():
        shutil.copy2(mock_amm_so, RESTORE_MOCK_AMM_SO)


def restore_deploy_artifacts():
    deploy_dir = ROOT / "target" / "deploy"
    deploy_dir.mkdir(parents=True, exist_ok=True)
    if RESTORE_AGENT_VAULT_SO.exists():
        shutil.copy2(RESTORE_AGENT_VAULT_SO, deploy_dir / "agent_vault.so")
    if RESTORE_MOCK_AMM_SO.exists():
        shutil.copy2(RESTORE_MOCK_AMM_SO, deploy_dir / "mock_amm.so")


def run_e2e():
    agent_account = write_fixtures()
    save_deploy_artifacts()
    validator = None
    success = False
    try:
        build_artifacts()
        validator = start_validator()
        request_airdrop(INITIALIZER.pubkey(), 20_000_000_000)
        request_airdrop(FEE_TREASURY, 1_000_000)
        request_airdrop(RECIPIENT, 1_000_000)

        print("localnet: initialize global config")
        send_ixs([ix_initialize_global_config()])
        require(get_account(global_config_pda()) is not None, "global config was not created")

        print("localnet: init vault and create wallets")
        send_ixs([ix_init_vault_config(agent_account)])
        send_ixs([ix_create_wallet(0, "treasury")])
        send_ixs([ix_create_wallet(1, "ops")])
        send_ixs([ix_update_wallet_label(0, "trading")])

        wallet0 = wallet_pda(AGENT_ASSET, 0)
        wallet1 = wallet_pda(AGENT_ASSET, 1)
        require(get_account(wallet0) is not None, "wallet #0 was not created")
        require(get_account(wallet1) is not None, "wallet #1 was not created")

        print("localnet: SOL deposit, withdraw, transfer, close")
        send_ixs([ix_deposit_sol(0, 3_000_000)])
        wallet0_after_deposit = get_balance(wallet0)
        send_ixs([ix_withdraw_sol(0, RECIPIENT, 500_000)])
        send_ixs([ix_transfer_sol(0, 1, 400_000)])
        send_ixs([ix_withdraw_sol(1, RECIPIENT, 400_000)])
        send_ixs([ix_close_wallet(1, RECIPIENT)])
        require(get_balance(wallet0) < wallet0_after_deposit, "wallet #0 SOL flow did not move lamports")

        print("localnet: ATA create and close")
        created_ata = ata(wallet0, MINT_CREATE_ATA)
        send_ixs([ix_create_wallet_ata(0, MINT_CREATE_ATA)])
        require(get_account(created_ata) is not None, "wallet ATA was not created")
        send_ixs([ix_close_wallet_ata(0, MINT_CREATE_ATA, RECIPIENT)])
        require(get_account(created_ata) is None, "wallet ATA was not closed")

        print("localnet: SPL transfer")
        transfer_source = ata(wallet0, MINT_TRANSFER)
        send_ixs([ix_transfer_spl(0, MINT_TRANSFER, transfer_source, TRANSFER_DEST, 250)])
        require(token_amount(transfer_source) == 750, "SPL source amount mismatch")
        require(token_amount(TRANSFER_DEST) == 250, "SPL destination amount mismatch")

        print("localnet: WSOL wrap and unwrap")
        wsol_ata = ata(wallet0, NATIVE_MINT)
        send_ixs([ix_create_wallet_ata(0, NATIVE_MINT)])
        send_ixs([ix_wrap_sol(0, 600_000), ix_sync_native(wsol_ata)])
        require(token_amount(wsol_ata) == 600_000, "WSOL amount mismatch after wrap")
        send_ixs([ix_unwrap_sol(0)])
        require(get_account(wsol_ata) is None, "WSOL ATA was not closed by unwrap")

        print("localnet: checked CPI mock swap")
        send_ixs([ix_execute_cpi_checked_mock_swap(100, 100, 40, 40)])
        require(token_amount(ata(wallet0, MINT_SWAP_IN)) == 900, "swap input amount mismatch")
        require(token_amount(POOL_INPUT) == 100, "pool input amount mismatch")
        require(token_amount(ata(wallet0, MINT_SWAP_OUT)) == 460, "pool output amount mismatch")
        require(token_amount(SWAP_USER_OUTPUT) == 40, "swap output amount mismatch")

        print("localnet e2e: ok")
        success = True
    finally:
        if validator is not None:
            validator.send_signal(signal.SIGINT)
            try:
                validator.wait(timeout=10)
            except subprocess.TimeoutExpired:
                validator.kill()
                validator.wait(timeout=5)
        restore_deploy_artifacts()
        if success:
            shutil.rmtree(TMP, ignore_errors=True)


if __name__ == "__main__":
    try:
        run_e2e()
    except Exception as exc:
        print(f"localnet e2e failed: {exc}", file=sys.stderr)
        sys.exit(1)
