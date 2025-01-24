import * as anchor from "@coral-xyz/anchor";
import { Program } from "@coral-xyz/anchor";
import { BlockWallet } from "../target/types/block_wallet";
import { PublicKey, SystemProgram } from "@solana/web3.js";
import { TOKEN_PROGRAM_ID, createMint, createAccount, mintTo, getAccount } from "@solana/spl-token";
import assert from "assert";

describe("block-wallet", () => {
  // Configure the client to use the local cluster.
  const provider = anchor.AnchorProvider.env();
  anchor.setProvider(provider);

  const program = anchor.workspace.BlockWallet as Program<BlockWallet>;

  let mint: PublicKey;
  let userTokenAccount: PublicKey;
  let feeAccount: PublicKey;
  let blockedWalletAccount: PublicKey;
  let wallet = provider.wallet as anchor.Wallet;

  const BLOCKED_TOKENS = ["pump.fun_mint_address", "moonshot_mint_address"]; // Replace with actual mint addresses.

  before(async () => {
    // Create a token mint
    mint = await createMint(provider.connection, wallet.payer, wallet.publicKey, null, 9);

    // Create a user token account
    userTokenAccount = await createAccount(provider.connection, wallet.payer, mint, wallet.publicKey);

    // Mint tokens to user account
    await mintTo(provider.connection, wallet.payer, mint, userTokenAccount, wallet.payer, 1_000_000_000); // 10 tokens

    // Create a fee account
    feeAccount = await createAccount(provider.connection, wallet.payer, mint, wallet.publicKey);

    // Derive a PDA for the blocked wallet account
    [blockedWalletAccount] = PublicKey.findProgramAddressSync(
      [wallet.publicKey.toBuffer()],
      program.programId
    );
  });

  it("Blocks a wallet", async () => {
    const blockDuration = 24 * 60 * 60; // 24 hours in seconds

    await program.methods
      .blockWallet(new anchor.BN(blockDuration))
      .accounts({
        blockedWallet: blockedWalletAccount,
        user: wallet.publicKey,
        userTokenAccount,
        feeAccount,
        tokenProgram: TOKEN_PROGRAM_ID,
        systemProgram: SystemProgram.programId,
        rent: anchor.web3.SYSVAR_RENT_PUBKEY,
      })
      .signers([])
      .rpc();

    // Fetch blocked wallet account data
    const blockedWallet = await program.account.blockedWallet.fetch(blockedWalletAccount);

    assert.ok(blockedWallet.isBlocked);
    assert.ok(blockedWallet.blockUntil.toNumber() > 0);

    // Verify fee deduction
    const feeAccountData = await getAccount(provider.connection, feeAccount);
    assert.strictEqual(feeAccountData.amount.toString(), "50000000"); // 0.05 SOL
  });

  it("Unblocks a wallet", async () => {
    await program.methods
      .unblockWallet()
      .accounts({
        blockedWallet: blockedWalletAccount,
        user: wallet.publicKey,
        userTokenAccount,
        feeAccount,
        tokenProgram: TOKEN_PROGRAM_ID,
      })
      .signers([])
      .rpc();

    // Fetch blocked wallet account data
    const blockedWallet = await program.account.blockedWallet.fetch(blockedWalletAccount);

    assert.ok(!blockedWallet.isBlocked);
    assert.strictEqual(blockedWallet.blockUntil.toNumber(), 0);

    // Verify higher fee deduction for unblock
    const feeAccountData = await getAccount(provider.connection, feeAccount);
    assert.strictEqual(feeAccountData.amount.toString(), "300000000"); // 0.25 SOL (total fees)
  });

  it("Allows selling tokens for unblocked wallets", async () => {
    const destinationAccount = await createAccount(provider.connection, wallet.payer, mint, wallet.publicKey);

    await program.methods
      .sellTokens(mint, new anchor.BN(100_000_000)) // 1 token
      .accounts({
        blockedWallet: blockedWalletAccount,
        userTokenAccount,
        destinationAccount,
        authority: wallet.publicKey,
        tokenProgram: TOKEN_PROGRAM_ID,
      })
      .signers([])
      .rpc();

    // Verify destination account received the tokens
    const destinationAccountData = await getAccount(provider.connection, destinationAccount);
    assert.strictEqual(destinationAccountData.amount.toString(), "100000000"); // 1 token
  });

  it("Blocks selling specific tokens for blocked wallets", async () => {
    // Block the wallet again
    const blockDuration = 24 * 60 * 60; // 24 hours in seconds
    await program.methods
      .blockWallet(new anchor.BN(blockDuration))
      .accounts({
        blockedWallet: blockedWalletAccount,
        user: wallet.publicKey,
        userTokenAccount,
        feeAccount,
        tokenProgram: TOKEN_PROGRAM_ID,
        systemProgram: SystemProgram.programId,
        rent: anchor.web3.SYSVAR_RENT_PUBKEY,
      })
      .signers([])
      .rpc();

    try {
      await program.methods
        .sellTokens(new PublicKey(BLOCKED_TOKENS[0]), new anchor.BN(100_000_000)) // Blocked token
        .accounts({
          blockedWallet: blockedWalletAccount,
          userTokenAccount,
          destinationAccount: feeAccount, // Dummy account for testing
          authority: wallet.publicKey,
          tokenProgram: TOKEN_PROGRAM_ID,
        })
        .signers([])
        .rpc();
      assert.fail("Transaction should have failed for blocked tokens.");
    } catch (err) {
      assert.ok(err.message.includes("This token is blocked."));
    }
  });
});
