import * as anchor from "@coral-xyz/anchor";
import { Program } from "@coral-xyz/anchor";
import { Clmm } from "../target/types/clmm";
import { assert } from "chai";
import { PublicKey, SystemProgram, Keypair } from "@solana/web3.js";
import {
  createMint,
  TOKEN_PROGRAM_ID,
  createAccount,
  mintTo,
  getAccount,
  createAssociatedTokenAccount,
} from "@solana/spl-token";

describe("clmm - pool creation and position opening test", () => {
  anchor.setProvider(anchor.AnchorProvider.env());
  const program = anchor.workspace.clmm as Program<Clmm>;

  const TICK_SPACING = 60;
  const INITIAL_SQRT_PRICE = new anchor.BN("79228162514264337593543950336"); // sqrt(1) * 2^96
  const TICKS_PER_ARRAY = 30; 

  let tokenMint0: PublicKey;
  let tokenMint1: PublicKey;
  let poolPda: PublicKey;
  let poolBump: number;
  let tokenVault0Keypair: Keypair;
  let tokenVault1Keypair: Keypair;
  

  let userTokenAccount0: PublicKey;
  let userTokenAccount1: PublicKey;
  
  const LOWER_TICK = 0; 
  const UPPER_TICK = 4000; 
  const LIQUIDITY_AMOUNT = new anchor.BN("100000"); 

  function i32ToLeBytes(value: number): Buffer {
    const buffer = Buffer.allocUnsafe(4);
    buffer.writeInt32LE(value, 0);
    return buffer;
  }

  function getTickArrayStartIndex(tick: number, tickSpacing: number): number {
    const ticksPerArrayI32 = TICKS_PER_ARRAY;
    const arrayIdx = Math.floor(Math.floor(tick / tickSpacing) / ticksPerArrayI32);
    return arrayIdx * ticksPerArrayI32 * tickSpacing;
  }

  before(async () => {
    console.log("Setting up test environment (creating mints and deriving PDAs)...");

    [poolPda, poolBump] = PublicKey.findProgramAddressSync(
      [
        Buffer.from("pool"),
      ],
      program.programId
    );

    tokenMint0 = await createMint(
      program.provider.connection,
      program.provider.wallet.payer,
      program.provider.wallet.publicKey, 
      null,
      6
    );

    tokenMint1 = await createMint(
      program.provider.connection,
      program.provider.wallet.payer,
      program.provider.wallet.publicKey,
      null,
      6
    );


    [poolPda, poolBump] = PublicKey.findProgramAddressSync(
      [
        Buffer.from("pool"),
        tokenMint0.toBuffer(),
        tokenMint1.toBuffer(),
        i32ToLeBytes(TICK_SPACING), 
      ],
      program.programId
    );

    tokenVault0Keypair = anchor.web3.Keypair.generate();
    tokenVault1Keypair = anchor.web3.Keypair.generate();

    userTokenAccount0 = await createAssociatedTokenAccount(
      program.provider.connection,
      program.provider.wallet.payer,
      tokenMint0,
      program.provider.wallet.publicKey
    );

    userTokenAccount1 = await createAssociatedTokenAccount(
      program.provider.connection,
      program.provider.wallet.payer,
      tokenMint1,
      program.provider.wallet.publicKey
    );

    await mintTo(
      program.provider.connection,
      program.provider.wallet.payer,
      tokenMint0,
      userTokenAccount0,
      program.provider.wallet.publicKey,
      1000000000 
    );

    await mintTo(
      program.provider.connection,
      program.provider.wallet.payer,
      tokenMint1,
      userTokenAccount1,
      program.provider.wallet.publicKey,
      1000000000
    );

    console.log("Test environment setup complete.");
    console.log("Token Mint 0:", tokenMint0.toString());
    console.log("Token Mint 1:", tokenMint1.toString());
    console.log("Pool PDA:", poolPda.toString());
  });

  it("Successfully creates a new CLMM pool", async () => {
    console.log("Attempting to initialize pool...");
    
    await program.methods
      .initializePool(TICK_SPACING, INITIAL_SQRT_PRICE)
      .accountsStrict({
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

    assert.equal(poolAccount.tickSpacing, TICK_SPACING);
    assert.equal(poolAccount.tokenMint0.toString(), tokenMint0.toString());
    assert.equal(poolAccount.tokenMint1.toString(), tokenMint1.toString());
    assert.equal(poolAccount.globalLiquidity.toString(), "0");
  });

  it("Successfully opens a position in the pool", async () => {
    console.log("Attempting to open position...");


    const lowerTickArrayStartIndex = getTickArrayStartIndex(LOWER_TICK, TICK_SPACING);
    const upperTickArrayStartIndex = getTickArrayStartIndex(UPPER_TICK, TICK_SPACING);

    console.log("Lower tick:", LOWER_TICK, "-> Array start:", lowerTickArrayStartIndex);
    console.log("Upper tick:", UPPER_TICK, "-> Array start:", upperTickArrayStartIndex);

    const [lowerTickArrayPda] = PublicKey.findProgramAddressSync(
      [
        Buffer.from("tick_array"),
        poolPda.toBuffer(),
        i32ToLeBytes(lowerTickArrayStartIndex), 
      ],
      program.programId
    );

    const [upperTickArrayPda] = PublicKey.findProgramAddressSync(
      [
        Buffer.from("tick_array"),
        poolPda.toBuffer(),
        i32ToLeBytes(upperTickArrayStartIndex),
      ],
      program.programId
    );

    const [positionPda] = PublicKey.findProgramAddressSync(
      [
        Buffer.from("position"),
        program.provider.wallet.publicKey.toBuffer(),
        poolPda.toBuffer(),
        i32ToLeBytes(LOWER_TICK), 
        i32ToLeBytes(UPPER_TICK), 
      ],
      program.programId
    );

    console.log("Position PDA:", positionPda.toString());
    console.log("Lower Tick Array PDA:", lowerTickArrayPda.toString());
    console.log("Upper Tick Array PDA:", upperTickArrayPda.toString());

    const userToken0Before = await getAccount(
      program.provider.connection,
      userTokenAccount0
    );
    const userToken1Before = await getAccount(
      program.provider.connection,
      userTokenAccount1
    );

    console.log("User token 0 balance before:", userToken0Before.amount.toString());
    console.log("User token 1 balance before:", userToken1Before.amount.toString());

    try{

      const tx = await program.methods
        .openPosition(
          program.provider.wallet.publicKey, // owner
          LOWER_TICK,                        // lower_tick
          UPPER_TICK,                        // upper_tick
          LIQUIDITY_AMOUNT,
          lowerTickArrayStartIndex,
          upperTickArrayStartIndex          
  
        )
        .accountsStrict({
          pool: poolPda,
          lowerTickArray: lowerTickArrayPda,
          upperTickArray: upperTickArrayPda,
          position: positionPda,
          userToken0: userTokenAccount0,
          userToken1: userTokenAccount1,
          poolToken0: tokenVault0Keypair.publicKey,
          poolToken1: tokenVault1Keypair.publicKey,
          payer: program.provider.wallet.publicKey,
          tokenMint0: tokenMint0,
          tokenMint1: tokenMint1,
          systemProgram: SystemProgram.programId,
          tokenProgram: TOKEN_PROGRAM_ID,
          rent: anchor.web3.SYSVAR_RENT_PUBKEY,
        })
        .rpc();
        console.log("Position opened! Transaction:", tx);
    }catch(e){
      console.log("Error:",e);
    }


    const positionAccount = await program.account.position.fetch(positionPda);
    console.log("Position account data:", positionAccount);

    assert.equal(positionAccount.owner.toString(), program.provider.wallet.publicKey.toString());
    assert.equal(positionAccount.pool.toString(), poolPda.toString());
    assert.equal(positionAccount.tickLower, LOWER_TICK);
    assert.equal(positionAccount.tickUpper, UPPER_TICK);
    assert.equal(positionAccount.liquidity.toString(), LIQUIDITY_AMOUNT.toString());

    const updatedPoolAccount = await program.account.pool.fetch(poolPda);
    console.log("Updated pool global liquidity:", updatedPoolAccount.globalLiquidity.toString());

    assert.equal(updatedPoolAccount.globalLiquidity.toString(), LIQUIDITY_AMOUNT.toString());

    const userToken0After = await getAccount(
      program.provider.connection,
      userTokenAccount0
    );
    const userToken1After = await getAccount(
      program.provider.connection,
      userTokenAccount1
    );

    console.log("User token 0 balance after:", userToken0After.amount.toString());
    console.log("User token 1 balance after:", userToken1After.amount.toString());

    const token0Transferred = userToken0Before.amount - userToken0After.amount;
    const token1Transferred = userToken1Before.amount - userToken1After.amount;

    console.log("Token 0 transferred:", token0Transferred.toString());
    console.log("Token 1 transferred:", token1Transferred.toString());

    assert.isTrue(
      token0Transferred > 0 || token1Transferred > 0,
      "At least one token should be transferred for liquidity"
    );

    try {
      const lowerTickArrayAccount = await program.account.tickArray.fetch(lowerTickArrayPda);
      console.log("Lower tick array initialized successfully");
    } catch (e) {
      console.log("Lower tick array fetch error:", e);
    }

    try {
      const upperTickArrayAccount = await program.account.tickArray.fetch(upperTickArrayPda);
      console.log("Upper tick array initialized successfully");
    } catch (e) {
      console.log("Upper tick array fetch error:", e);
    }
  });
});