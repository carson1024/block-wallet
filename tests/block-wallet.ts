import * as anchor from "@coral-xyz/anchor";
import { Program } from "@coral-xyz/anchor";
import { BlockWallet } from "../target/types/block_wallet";
import { assert } from "chai";
import { SYSTEM_PROGRAM_ID } from "@coral-xyz/anchor/dist/cjs/native/system";

describe("wallet_blocker", () => {
  const provider = anchor.AnchorProvider.env();
  anchor.setProvider(provider);
  const program = anchor.workspace.BlockWallet as Program<BlockWallet>;

  let walletPDA: anchor.web3.PublicKey;
  let walletBump: number;
  const FEE_ACCOUNT = new anchor.web3.PublicKey("Hi9Q3RsB8MDTK1riqgUWDMcts5Y4UeJJVbTDPTkNPdsr");

  before(async () => {
    // Derive the PDA for the wallet
    [walletPDA, walletBump] = await anchor.web3.PublicKey.findProgramAddress(
      [Buffer.from("wallet"), provider.wallet.publicKey.toBuffer()],
      program.programId
    );

    // Airdrop SOL to the payer for transaction fees
    const airdropTx = await provider.connection.requestAirdrop(
      provider.wallet.publicKey,
      anchor.web3.LAMPORTS_PER_SOL
    );
    
    await provider.connection.confirmTransaction(airdropTx);

    // Initialize the wallet account
    // const tx = await program.methods
    //   .initialize(FEE_ACCOUNT)
    //   .accounts({
    //     user: provider.wallet.publicKey
    //   })
    //   .rpc();

    // console.log("Wallet initialized, transaction signature:", tx);
  });

  it("blocks the wallet", async () => {
    const blockDuration = new anchor.BN(60); // Block for 60 seconds

    const tx = await program.methods
      .blockWallet(blockDuration)
      .accounts({
        user: provider.wallet.publicKey,
        feeAccount: FEE_ACCOUNT,
      })
      .rpc();

    console.log("Block wallet transaction signature:", tx);

    const walletState = await program.account.wallet.fetch(walletPDA);
    console.log("Block", walletPDA, walletState.blockExpiry.toNumber());
    assert.ok(walletState.blockExpiry.toNumber() > 0, "Block expiry should be set");
  });

  it("unblocks the wallet", async () => {
    // Wait for block duration to expire (for simplicity, assume expiry has passed in this test)

    const tx = await program.methods
      .unblockWallet()
      .accounts({
        user: provider.wallet.publicKey,
        feeAccount: FEE_ACCOUNT,
      })
      .rpc();

    console.log("Unblock wallet transaction signature:", tx);

    const walletState = await program.account.wallet.fetch(walletPDA);
    console.log("UnBlock", walletPDA, walletState.blockExpiry);
    assert.equal(walletState.blockExpiry.toNumber(), 0, "Block expiry should be reset");
  });
});