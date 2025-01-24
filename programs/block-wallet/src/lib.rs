use anchor_lang::prelude::*;
use anchor_spl::token::{self, Token, TokenAccount, Transfer};

declare_id!("A77rXueYXYg3bJejEFRuQFv2bbV3sBhCZwotH4cy4HCX");

#[program]
pub mod block_wallet {
    use super::*;

    // Block a wallet with an expiry time
    pub fn block_wallet(ctx: Context<BlockWallet>, expiry_time: i64) -> Result<()> {
        let blocked_wallet = &mut ctx.accounts.blocked_wallet;
        let clock = Clock::get()?;

        // Set wallet block details
        blocked_wallet.wallet = ctx.accounts.user.key();
        blocked_wallet.block_until = clock.unix_timestamp + expiry_time; // Expiry time in seconds
        blocked_wallet.is_blocked = true;

        // Charge block fee
        let fee_account = &mut ctx.accounts.fee_account;
        let user = &ctx.accounts.user;

        let cpi_accounts = Transfer {
            from: ctx.accounts.user_token_account.to_account_info(),
            to: fee_account.to_account_info(),
            authority: user.to_account_info(),
        };

        let cpi_ctx = CpiContext::new(ctx.accounts.token_program.to_account_info(), cpi_accounts);
        token::transfer(cpi_ctx, 50_000_000)?; // 0.05 SOL (adjust decimals based on SOL token)

        Ok(())
    }

    // Unblock a wallet before expiry by paying a higher fee
    pub fn unblock_wallet(ctx: Context<UnblockWallet>) -> Result<()> {
        let blocked_wallet = &mut ctx.accounts.blocked_wallet;

        require!(blocked_wallet.is_blocked, CustomError::WalletNotBlocked);

        // Charge unblock fee
        let fee_account = &mut ctx.accounts.fee_account;
        let user = &ctx.accounts.user;

        let cpi_accounts = Transfer {
            from: ctx.accounts.user_token_account.to_account_info(),
            to: fee_account.to_account_info(),
            authority: user.to_account_info(),
        };

        let cpi_ctx = CpiContext::new(ctx.accounts.token_program.to_account_info(), cpi_accounts);
        token::transfer(cpi_ctx, 250_000_000)?; // 0.25 SOL (adjust decimals based on SOL token)

        // Unblock the wallet
        blocked_wallet.is_blocked = false;
        blocked_wallet.block_until = 0;

        Ok(())
    }

    // Sell tokens (if wallet is blocked, check provenance of tokens)
    pub fn sell_tokens(ctx: Context<SellTokens>, token_mint: Pubkey, amount: u64) -> Result<()> {
        let blocked_wallet = &ctx.accounts.blocked_wallet;
        let clock = Clock::get()?;

        // Restrict transactions for blocked wallets
        if blocked_wallet.is_blocked && clock.unix_timestamp < blocked_wallet.block_until {
            require!(
                !BLOCKED_TOKENS.contains(&token_mint.to_string().as_str()),
                CustomError::BlockedToken
            );
        }

        // Proceed with the sale
        let cpi_accounts = Transfer {
            from: ctx.accounts.user_token_account.to_account_info(),
            to: ctx.accounts.destination_account.to_account_info(),
            authority: ctx.accounts.authority.to_account_info(),
        };

        let cpi_ctx = CpiContext::new(ctx.accounts.token_program.to_account_info(), cpi_accounts);
        token::transfer(cpi_ctx, amount)?;

        Ok(())
    }
}

// Contexts for each instruction
#[derive(Accounts)]
pub struct BlockWallet<'info> {
    #[account(mut)]
    pub blocked_wallet: Account<'info, BlockedWallet>,
    #[account(mut)]
    pub user: Signer<'info>,
    #[account(mut)]
    pub user_token_account: Account<'info, TokenAccount>,
    #[account(mut)]
    pub fee_account: Account<'info, TokenAccount>,
    pub token_program: Program<'info, Token>,
    pub system_program: Program<'info, System>,
    pub rent: Sysvar<'info, Rent>,
}

#[derive(Accounts)]
pub struct UnblockWallet<'info> {
    #[account(mut)]
    pub blocked_wallet: Account<'info, BlockedWallet>,
    #[account(mut)]
    pub user: Signer<'info>,
    #[account(mut)]
    pub user_token_account: Account<'info, TokenAccount>,
    #[account(mut)]
    pub fee_account: Account<'info, TokenAccount>,
    pub token_program: Program<'info, Token>,
}

#[derive(Accounts)]
pub struct SellTokens<'info> {
    #[account(mut)]
    pub blocked_wallet: Account<'info, BlockedWallet>,
    #[account(mut)]
    pub user_token_account: Account<'info, TokenAccount>,
    #[account(mut)]
    pub destination_account: Account<'info, TokenAccount>,
    pub authority: Signer<'info>,
    pub token_program: Program<'info, Token>,
}

// Data structure for blocked wallet account
#[account]
pub struct BlockedWallet {
    pub wallet: Pubkey,        // Wallet address
    pub block_until: i64,      // Timestamp until which the wallet is blocked
    pub is_blocked: bool,      // Block status
}

// Errors
#[error_code]
pub enum CustomError {
    #[msg("This wallet is not blocked.")]
    WalletNotBlocked,
    #[msg("This token is blocked.")]
    BlockedToken,
}

// Constants
const BLOCKED_TOKENS: [&str; 2] = ["pump.fun_mint_address", "moonshot_mint_address"]; // Replace with actual mint addresses