import * as anchor from "@coral-xyz/anchor";
import { Program, BN } from "@coral-xyz/anchor";
import { AmmProgram } from "../target/types/amm_program";
import {
  createMint,
  createAssociatedTokenAccount,
  mintTo,
  getAssociatedTokenAddressSync,
  TOKEN_PROGRAM_ID,
  ASSOCIATED_TOKEN_PROGRAM_ID,
} from "@solana/spl-token";
import { PublicKey, SystemProgram } from "@solana/web3.js";
import { expect } from "chai";

describe("amm-program — init + deposit", () => {
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

  const seed = new BN(42);
  const fee = 30;

  before(async () => {
    mintX = await createMint(connection, payer, payer.publicKey, null, 6);
    mintY = await createMint(connection, payer, payer.publicKey, null, 6);

    // Sort mints to ensure consistent ordering
    if (mintX.toBuffer().compare(mintY.toBuffer()) > 0) {
      [mintX, mintY] = [mintY, mintX];
    }

    [configPda] = PublicKey.findProgramAddressSync(
      [
        Buffer.from("config"),
        mintX.toBuffer(),
        mintY.toBuffer(),
        seed.toArrayLike(Buffer, "le", 8),
      ],
      program.programId
    );

    [mintLpPda] = PublicKey.findProgramAddressSync(
      [Buffer.from("lp"), configPda.toBuffer()],
      program.programId
    );

    vaultX = getAssociatedTokenAddressSync(mintX, configPda, true);
    vaultY = getAssociatedTokenAddressSync(mintY, configPda, true);

    const ataX = await createAssociatedTokenAccount(
      connection, payer, mintX, payer.publicKey
    );
    const ataY = await createAssociatedTokenAccount(
      connection, payer, mintY, payer.publicKey
    );

    await mintTo(connection, payer, mintX, ataX, payer, 1_000_000_000);
    await mintTo(connection, payer, mintY, ataY, payer, 1_000_000_000);
  });

  it("initializes a pool", async () => {
    await program.methods
      .initialize(seed, fee, null)
      .accountsPartial({
        initializer: payer.publicKey,
        mintX,
        mintY,
        mintLp: mintLpPda,
        vaultX,
        vaultY,
        config: configPda,
        tokenProgram: TOKEN_PROGRAM_ID,
        associatedTokenProgram: ASSOCIATED_TOKEN_PROGRAM_ID,
        systemProgram: SystemProgram.programId,
      })
      .rpc();

    const cfg = await program.account.config.fetch(configPda);
    expect(cfg.fee).to.equal(fee);
    expect(cfg.locked).to.equal(false);
    expect(cfg.mintX.toBase58()).to.equal(mintX.toBase58());
    expect(cfg.mintY.toBase58()).to.equal(mintY.toBase58());
  });

  it("accepts the first deposit and mints LP tokens", async () => {
    const ataX = getAssociatedTokenAddressSync(mintX, payer.publicKey);
    const ataY = getAssociatedTokenAddressSync(mintY, payer.publicKey);
    const ataLp = getAssociatedTokenAddressSync(mintLpPda, payer.publicKey);

    const lpAmount = new BN(1_000_000);
    const maxX = new BN(100_000_000);
    const maxY = new BN(200_000_000);

    await program.methods
      .deposit(lpAmount, maxX, maxY)
      .accountsPartial({
        lpProvider: payer.publicKey,
        mintX,
        mintY,
        config: configPda,
        mintLp: mintLpPda,
        vaultX,
        vaultY,
        lpProviderAtaX: ataX,
        lpProviderAtaY: ataY,
        lpProviderAtaLp: ataLp,
        tokenProgram: TOKEN_PROGRAM_ID,
        associatedTokenProgram: ASSOCIATED_TOKEN_PROGRAM_ID,
        systemProgram: SystemProgram.programId,
      })
      .rpc();

    const vx = await connection.getTokenAccountBalance(vaultX);
    const vy = await connection.getTokenAccountBalance(vaultY);
    expect(vx.value.amount).to.equal(maxX.toString());
    expect(vy.value.amount).to.equal(maxY.toString());

    const lp = await connection.getTokenAccountBalance(ataLp);
    expect(lp.value.amount).to.equal(lpAmount.toString());
  });
});