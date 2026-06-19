import * as anchor from "@coral-xyz/anchor";
import { Program, BN } from "@coral-xyz/anchor";
import { AmmProgram } from "../target/types/amm_program";
import {
  createMint,
  createAssociatedTokenAccount,
  getOrCreateAssociatedTokenAccount,
  mintTo,
  getAssociatedTokenAddressSync,
  TOKEN_PROGRAM_ID,
  ASSOCIATED_TOKEN_PROGRAM_ID,
} from "@solana/spl-token";
import { PublicKey, SystemProgram } from "@solana/web3.js";
import { expect } from "chai";

describe("amm-program — full lifecycle", () => {
  const provider = anchor.AnchorProvider.env();
  anchor.setProvider(provider);
  const program = anchor.workspace.AmmProgram as Program<AmmProgram>;
  const payer = (provider.wallet as anchor.Wallet).payer;
  const connection = provider.connection;

  let mintX: PublicKey;
  let mintY: PublicKey;
  let configPda: PublicKey;
  let mintLpPda: PublicKey;
  let vaultX: PublicKey;
  let vaultY: PublicKey;
  let ataX: PublicKey;
  let ataY: PublicKey;
  let ataLp: PublicKey;

  const seed = new BN(Math.floor(Math.random() * 1_000_000));
  const fee = 30;

  const FIRST_LP = new BN(1_000_000);
  const FIRST_X = new BN(100_000_000);
  const FIRST_Y = new BN(200_000_000);

  before(async () => {
    mintX = await createMint(connection, payer, payer.publicKey, null, 6);
    mintY = await createMint(connection, payer, payer.publicKey, null, 6);
    if (mintX.toBuffer().compare(mintY.toBuffer()) > 0) {
      [mintX, mintY] = [mintY, mintX];
    }

    [configPda] = PublicKey.findProgramAddressSync(
      [Buffer.from("config"), mintX.toBuffer(), mintY.toBuffer(), seed.toArrayLike(Buffer, "le", 8)],
      program.programId
    );
    [mintLpPda] = PublicKey.findProgramAddressSync(
      [Buffer.from("lp"), configPda.toBuffer()],
      program.programId
    );
    vaultX = getAssociatedTokenAddressSync(mintX, configPda, true);
    vaultY = getAssociatedTokenAddressSync(mintY, configPda, true);

    ataX = (await getOrCreateAssociatedTokenAccount(connection, payer, mintX, payer.publicKey)).address;
    ataY = (await getOrCreateAssociatedTokenAccount(connection, payer, mintY, payer.publicKey)).address;

    await mintTo(connection, payer, mintX, ataX, payer, 1_000_000_000);
    await mintTo(connection, payer, mintY, ataY, payer, 1_000_000_000);
  });

  const poolAccounts = () => ({
    mintX,
    mintY,
    config: configPda,
    mintLp: mintLpPda,
    vaultX,
    vaultY,
    tokenProgram: TOKEN_PROGRAM_ID,
    associatedTokenProgram: ASSOCIATED_TOKEN_PROGRAM_ID,
    systemProgram: SystemProgram.programId,
  });

  it("step 1 — initializes the pool", async () => {
    await program.methods
      .initialize(seed, fee, null)
      .accountsPartial({ initializer: payer.publicKey, ...poolAccounts() })
      .rpc();

    const cfg = await program.account.config.fetch(configPda);
    expect(Number(cfg.fee)).to.equal(fee);
    expect(cfg.locked).to.equal(false);
    expect(cfg.mintX.toBase58()).to.equal(mintX.toBase58());
    expect(cfg.mintY.toBase58()).to.equal(mintY.toBase58());
  });

  it("step 2 — accepts the first deposit", async () => {
    // Create LP ATA for payer
    ataLp = (await getOrCreateAssociatedTokenAccount(connection, payer, mintLpPda, payer.publicKey)).address;

    await program.methods
      .deposit(FIRST_LP, FIRST_X, FIRST_Y)
      .accountsPartial({
        lpProvider: payer.publicKey,
        lpProviderAtaX: ataX,
        lpProviderAtaY: ataY,
        lpProviderAtaLp: ataLp,
        ...poolAccounts(),
      })
      .rpc();

    const vx = await connection.getTokenAccountBalance(vaultX);
    const vy = await connection.getTokenAccountBalance(vaultY);
    expect(vx.value.amount).to.equal(FIRST_X.toString());
    expect(vy.value.amount).to.equal(FIRST_Y.toString());

    const lp = await connection.getTokenAccountBalance(ataLp);
    expect(lp.value.amount).to.equal(FIRST_LP.toString());
  });

  it("step 3 — swaps X for Y and k grows", async () => {
    const balXBefore = BigInt((await connection.getTokenAccountBalance(ataX)).value.amount);
    const balYBefore = BigInt((await connection.getTokenAccountBalance(ataY)).value.amount);

    await program.methods
      .swap({ isX: true, amount: new BN(10_000_000), min: new BN(1) })
      .accountsPartial({
        user: payer.publicKey,
        userAtaX: ataX,
        userAtaY: ataY,
        tokenProgram: TOKEN_PROGRAM_ID,
        associatedTokenProgram: ASSOCIATED_TOKEN_PROGRAM_ID,
        mintX, mintY, config: configPda, mintLp: mintLpPda, vaultX, vaultY,
      })
      .rpc();

    const balXAfter = BigInt((await connection.getTokenAccountBalance(ataX)).value.amount);
    const balYAfter = BigInt((await connection.getTokenAccountBalance(ataY)).value.amount);

    expect(balXBefore - balXAfter).to.equal(BigInt(10_000_000));
    expect(balYAfter > balYBefore).to.be.true;

    const vx = BigInt((await connection.getTokenAccountBalance(vaultX)).value.amount);
    const vy = BigInt((await connection.getTokenAccountBalance(vaultY)).value.amount);
    expect(vx * vy >= BigInt(FIRST_X.toString()) * BigInt(FIRST_Y.toString())).to.be.true;
  });

  it("step 3b — swaps Y for X (reverse direction)", async () => {
    const balXBefore = BigInt((await connection.getTokenAccountBalance(ataX)).value.amount);
    const balYBefore = BigInt((await connection.getTokenAccountBalance(ataY)).value.amount);

    await program.methods
      .swap({ isX: false, amount: new BN(10_000_000), min: new BN(1) })
      .accountsPartial({
        user: payer.publicKey,
        userAtaX: ataX,
        userAtaY: ataY,
        tokenProgram: TOKEN_PROGRAM_ID,
        associatedTokenProgram: ASSOCIATED_TOKEN_PROGRAM_ID,
        mintX, mintY, config: configPda, mintLp: mintLpPda, vaultX, vaultY,
      })
      .rpc();

    const balXAfter = BigInt((await connection.getTokenAccountBalance(ataX)).value.amount);
    const balYAfter = BigInt((await connection.getTokenAccountBalance(ataY)).value.amount);

    expect(balYBefore - balYAfter).to.equal(BigInt(10_000_000));
    expect(balXAfter > balXBefore).to.be.true;
  });

  it("step 4 — withdraws half the LP", async () => {
    const lpBefore = BigInt((await connection.getTokenAccountBalance(ataLp)).value.amount);
    const xBefore = BigInt((await connection.getTokenAccountBalance(ataX)).value.amount);
    const yBefore = BigInt((await connection.getTokenAccountBalance(ataY)).value.amount);
    const burnAmount = new BN(500_000);

    await program.methods
      .withdraw(burnAmount, new BN(1), new BN(1))
      .accountsPartial({
        lpProvider: payer.publicKey,
        lpProviderAtaX: ataX,
        lpProviderAtaY: ataY,
        lpProviderAtaLp: ataLp,
        ...poolAccounts(),
      })
      .rpc();

    const lpAfter = BigInt((await connection.getTokenAccountBalance(ataLp)).value.amount);
    const xAfter = BigInt((await connection.getTokenAccountBalance(ataX)).value.amount);
    const yAfter = BigInt((await connection.getTokenAccountBalance(ataY)).value.amount);

    expect(lpBefore - lpAfter).to.equal(BigInt(burnAmount.toString()));
    expect(xAfter > xBefore).to.be.true;
    expect(yAfter > yBefore).to.be.true;
  });

  // ── negative cases ──────────────────────────────────────────────────────────

  it("rejects a swap with min too high (slippage)", async () => {
    try {
      await program.methods
        .swap({ isX: true, amount: new BN(10_000_000), min: new BN(999_999_999_999) })
        .accountsPartial({
          user: payer.publicKey,
          userAtaX: ataX,
          userAtaY: ataY,
          tokenProgram: TOKEN_PROGRAM_ID,
          associatedTokenProgram: ASSOCIATED_TOKEN_PROGRAM_ID,
          mintX, mintY, config: configPda, mintLp: mintLpPda, vaultX, vaultY,
        })
        .rpc();
      expect.fail("should have reverted");
    } catch (e: any) {
      expect(e.toString()).to.match(/InvalidAmount|Invalid Amount|0x1775/i);
    }
  });

  it("rejects deposit when max_y is too small", async () => {
    try {
      await program.methods
        .deposit(new BN(100_000), new BN(100_000_000), new BN(1))
        .accountsPartial({
          lpProvider: payer.publicKey,
          lpProviderAtaX: ataX,
          lpProviderAtaY: ataY,
          lpProviderAtaLp: ataLp,
          ...poolAccounts(),
        })
        .rpc();
      expect.fail("should have reverted");
    } catch (e: any) {
      expect(e.toString()).to.match(/InsufficientTokenY|Insufficient amount of token Y|0x1777/i);
    }
  });
});
