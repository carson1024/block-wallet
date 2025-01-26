use anchor_lang::prelude::*;
use anchor_lang::solana_program::{program::invoke, system_instruction};

declare_id!("A77rXueYXYg3bJejEFRuQFv2bbV3sBhCZwotH4cy4HCX");

#[program]
pub mod block_wallet {
    use super::*;

    const BLOCK_FEE_LAMPORTS: u64 = 50_000_000; // 0.05 SOL
    const UNBLOCK_FEE_LAMPORTS: u64 = 250_000_000; // 0.25 SOL

    pub fn initialize(ctx: Context<Initialize>, fee_account: Pubkey) -> Result<()> {
        let wallet = &mut ctx.accounts.wallet;
        wallet.block_expiry = 0;
        wallet.fee_account = fee_account;
        Ok(())
    }

    pub fn block_wallet(ctx: Context<BlockWallet>, block_duration: i64) -> Result<()> {
        let wallet = &mut ctx.accounts.wallet;
        let clock = Clock::get()?;

        // Ensure the wallet is blocked and block duration has expired
        require!(clock.unix_timestamp > wallet.block_expiry, CustomError::BlockNotExpired);

        // Set the block expiry time
        wallet.block_expiry = clock.unix_timestamp + block_duration;

        // Transfer 0.05 SOL as block fee
        invoke(
            &system_instruction::transfer(
                &ctx.accounts.user.key(),
                &ctx.accounts.fee_account.key(),
                BLOCK_FEE_LAMPORTS,
            ),
            &[
                ctx.accounts.user.to_account_info(),
                ctx.accounts.fee_account.to_account_info(),
            ],
        )?;

        Ok(())
    }

    pub fn unblock_wallet(ctx: Context<UnblockWallet>) -> Result<()> {
        let wallet = &mut ctx.accounts.wallet;
        let clock = Clock::get()?;

        require!(wallet.block_expiry > 0, CustomError::BlockExpired);

        if wallet.block_expiry > clock.unix_timestamp {
            // Transfer 0.25 SOL as unblock fee
            invoke(
                &system_instruction::transfer(
                    &ctx.accounts.user.key(),
                    &ctx.accounts.fee_account.key(),
                    UNBLOCK_FEE_LAMPORTS,
                ),
                &[
                    ctx.accounts.user.to_account_info(),
                    ctx.accounts.fee_account.to_account_info(),
                ],
            )?;
        }
        // Reset block expiry
        wallet.block_expiry = 0;

        Ok(())
    }
    
    pub fn get_block_status(ctx: Context<GetBlockStatus>) -> Result<i64> {
        let wallet = &ctx.accounts.wallet;
        Ok(wallet.block_expiry)
    }

}

const EXPECTED_WALLET_PUBLIC_KEY: &str = "FwLFdJeGwx7UUAQReU4tx94KA4KZjyp4eX8kdWf4yyG8";

#[derive(Accounts)]
pub struct Initialize<'info> {
    #[account(mut)]
    pub user: Signer<'info>,
    #[account(init, seeds = [b"wallet", user.key().as_ref()], bump, payer = user, space = 8 + std::mem::size_of::<Wallet>())]
    pub wallet: Account<'info, Wallet>,
    pub system_program: Program<'info, System>,
}

#[derive(Accounts)]
pub struct BlockWallet<'info> {
    #[account(mut)]
    pub user: Signer<'info>,
    #[account(mut, seeds = [b"wallet", user.key().as_ref()], bump)]    
    pub wallet: Account<'info, Wallet>,
    /// CHECK: The wallet account is explicitly validated to match the Phantom wallet's public key.
    #[account(mut, constraint = fee_account.key() == EXPECTED_WALLET_PUBLIC_KEY.parse::<Pubkey>().unwrap())]
    pub fee_account: UncheckedAccount<'info>,
    pub system_program: Program<'info, System>,
}

#[derive(Accounts)]
pub struct UnblockWallet<'info> {
    #[account(mut)]
    pub user: Signer<'info>,
    #[account(mut, seeds = [b"wallet", user.key().as_ref()], bump)]
    pub wallet: Account<'info, Wallet>,
    /// CHECK: The wallet account is explicitly validated to match the Phantom wallet's public key.
    #[account(mut, constraint = fee_account.key() == EXPECTED_WALLET_PUBLIC_KEY.parse::<Pubkey>().unwrap())]
    pub fee_account: UncheckedAccount<'info>,
    pub system_program: Program<'info, System>,
}

#[derive(Accounts)]
pub struct GetBlockStatus<'info> {
    #[account(mut, seeds = [b"wallet", user.key().as_ref()], bump)]
    pub wallet: Account<'info, Wallet>,
    pub user: Signer<'info>,
}

#[account]
pub struct Wallet {
    pub block_expiry: i64, // Timestamp for block expiry
    pub fee_account: Pubkey,
}

#[error_code]
pub enum CustomError {
    #[msg("Block duration has not yet expired.")]
    BlockNotExpired,
    #[msg("Block duration already expired.")]
    BlockExpired,
}
