import * as anchor from "@coral-xyz/anchor";
import { Program } from "@coral-xyz/anchor";
import { Clmm } from "../target/types/clmm";
import { assert } from "chai";
import { PublicKey, SystemProgram, Keypair } from "@solana/web3.js";
import {
  createMint,
  TOKEN_PROGRAM_ID,
} from "@solana/spl-token";

describe("clmm - simple pool creation test", () => {
  anchor.setProvider(anchor.AnchorProvider.env());
  const program = anchor.workspace.clmm as Program<Clmm>;

  const TICK_SPACING = 60;
  const INITIAL_SQRT_PRICE = new anchor.BN(1);

  let tokenMint0: PublicKey; 
  let tokenMint1: PublicKey; 
  let poolPda: PublicKey;
  let poolBump: number;
  let tokenVault0Keypair: Keypair; 
  let tokenVault1Keypair: Keypair; 

  before(async () => {
    console.log("Setting up test environment (creating mints and deriving PDAs)...");

    tokenMint0 = await createMint(
      program.provider.connection,
      program.provider.wallet.payer,
      poolPda,
      null,
      6, 
    );

    tokenMint1 = await createMint(
      program.provider.connection,
      program.provider.wallet.payer,
      poolPda,
      null,
      6,
    );

    [poolPda, poolBump] = PublicKey.findProgramAddressSync(
      [
        Buffer.from("pool"),
        tokenMint0.toBuffer(), 
        tokenMint1.toBuffer(), 
        Buffer.from(new anchor.BN(TICK_SPACING).toArray("le", 4)),
      ],
      program.programId
    );

    tokenVault0Keypair = anchor.web3.Keypair.generate();
    tokenVault1Keypair = anchor.web3.Keypair.generate();

    console.log("Test environment setup complete.");
  });

  it("Successfully creates a new CLMM pool", async () => {
    console.log("Attempting to initialize pool...");
    await program.methods
      .initializePool(TICK_SPACING, INITIAL_SQRT_PRICE)
      .accounts({
        payer: program.provider.wallet.publicKey,
        pool: poolPda,
        tokenMint0: tokenMint0, 
        tokenMint1: tokenMint1,
        tokenVault0: tokenVault0Keypair.publicKey,
        tokenVault1: tokenVault1Keypair.publicKey,
        systemProgram: anchor.web3.SystemProgram.programId,
        tokenProgram: TOKEN_PROGRAM_ID,
        rent: anchor.web3.SYSVAR_RENT_PUBKEY,
      })
      .signers([tokenVault0Keypair, tokenVault1Keypair])
      .rpc();

    console.log("Pool initialization transaction sent!");

    const poolAccount = await program.account.pool.fetch(poolPda);
    console.log("Pool account data:", poolAccount);

  });
});