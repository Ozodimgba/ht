use anchor_lang::prelude::*;
use crate::constants::{
    MAX_SLIPPAGE, 
    MEDIUM_TRADE_THRESHOLD, 
    SMALL_TRADE_THRESHOLD, 
    VOLUME_THRESHOLD
};
use crate::errors::HypeBondError;
use crate::constant;

#[account]
#[derive(Default)]
pub struct BondingCurve {
    // Reserve tracking
    pub virtual_token_reserves: u64,
    pub virtual_sol_reserves: u64,
    pub real_token_reserves: u64,
    pub real_sol_reserves: u64,
    pub token_total_supply: u64,
    pub complete: bool,

    // Protection parameters
    pub last_hour_volume: u64,
    pub last_hour_timestamp: i64,
    pub min_trade_amount: u64,
    pub max_trade_amount: u64,
    pub base_slippage: u64,
    pub volume_multiplier: u64,
}

impl BondingCurve {
    // Core price calculation using constant product formula
    pub fn get_base_price(&self, amount: u64, is_buy: bool) -> Result<u64> {
        let k = self.virtual_token_reserves
            .checked_mul(self.virtual_sol_reserves)
            .ok_or(HypeBondError::MathOverflow)?;

        if is_buy {
            let new_token_reserves = self.virtual_token_reserves
                .checked_add(amount)
                .ok_or(HypeBondError::MathOverflow)?;
            
            let new_sol_reserves = k / new_token_reserves;
            
            self.virtual_sol_reserves
                .checked_sub(new_sol_reserves)
                .ok_or(HypeBondError::MathOverflow.into())
        } else {
            let new_token_reserves = self.virtual_token_reserves
                .checked_sub(amount)
                .ok_or(HypeBondError::MathOverflow)?;
            
            let new_sol_reserves = k / new_token_reserves;
            
            new_sol_reserves
                .checked_sub(self.virtual_sol_reserves)
                .ok_or(HypeBondError::MathOverflow.into())
        }
    }

    // Calculate buy price with all protections
    pub fn calculate_buy_price(&self, amount: u64) -> Result<u64> {
        // Get base price from bonding curve
        let base_amount = self.get_base_price(amount, true)?;

        // Apply progressive protection based on size
        let protection_multiplier = if amount < SMALL_TRADE_THRESHOLD {
            101 // 1% slippage
        } else if amount < MEDIUM_TRADE_THRESHOLD {
            103 // 3% slippage
        } else {
            105 // 5% slippage
        };

        // Apply dynamic safety bands based on volume
        let volume_factor = self.last_hour_volume
            .checked_div(VOLUME_THRESHOLD)
            .unwrap_or(1);
        
        let dynamic_multiplier = self.base_slippage
            .checked_add(volume_factor * self.volume_multiplier)
            .unwrap_or(MAX_SLIPPAGE);

        // Calculate final amount with all protections
        base_amount
            .checked_mul(protection_multiplier)
            .unwrap()
            .checked_mul(dynamic_multiplier)
            .unwrap()
            .checked_div(10000)
            .ok_or(HypeBondError::MathOverflow.into())
    }

    // Protective sell price calculation
    pub fn calculate_sell_price(&self, amount: u64) -> Result<u64> {
        // Get base price from bonding curve (x * y = k)
        let base_amount = self.get_base_price(amount, false)?;

        // Apply stronger protection for sells
        let protection_multiplier = if amount < SMALL_TRADE_THRESHOLD {
            99  // 1% penalty
        } else if amount < MEDIUM_TRADE_THRESHOLD {
            97  // 3% penalty
        } else {
            95  // 5% penalty
        };

        // Dynamic slippage based on recent volume
        let volume_factor = self.last_hour_volume
            .checked_div(VOLUME_THRESHOLD)
            .unwrap_or(1);
        
        let dynamic_multiplier = self.base_slippage
            .checked_add(volume_factor * self.volume_multiplier)
            .unwrap_or(MAX_SLIPPAGE);

        // Calculate final price with all protections
        base_amount
            .checked_mul(protection_multiplier)
            .unwrap()
            .checked_mul(dynamic_multiplier)
            .unwrap()
            .checked_div(10000)
            .ok_or(HypeBondError::MathOverflow.into())
    }

    // Update reserves after trade
    pub fn update_reserves(
        &mut self,
        token_amount: u64,
        sol_amount: u64,
        is_buy: bool,
    ) -> Result<()> {
        if is_buy {
            self.virtual_token_reserves = self.virtual_token_reserves
                .checked_add(token_amount)
                .ok_or(HypeBondError::MathOverflow)?;
            
            self.virtual_sol_reserves = self.virtual_sol_reserves
                .checked_sub(sol_amount)
                .ok_or(HypeBondError::MathOverflow)?;
            
            self.real_token_reserves = self.real_token_reserves
                .checked_sub(token_amount)
                .ok_or(HypeBondError::MathOverflow)?;
            
            self.real_sol_reserves = self.real_sol_reserves
                .checked_add(sol_amount)
                .ok_or(HypeBondError::MathOverflow)?;
        } else {
            self.virtual_token_reserves = self.virtual_token_reserves
                .checked_sub(token_amount)
                .ok_or(HypeBondError::MathOverflow)?;
            
            self.virtual_sol_reserves = self.virtual_sol_reserves
                .checked_add(sol_amount)
                .ok_or(HypeBondError::MathOverflow)?;
            
            self.real_token_reserves = self.real_token_reserves
                .checked_add(token_amount)
                .ok_or(HypeBondError::MathOverflow)?;
            
            self.real_sol_reserves = self.real_sol_reserves
                .checked_sub(sol_amount)
                .ok_or(HypeBondError::MathOverflow)?;
        }
        Ok(())
    }

    pub fn update_volume(&mut self, amount: u64, timestamp: i64) {
        if timestamp - self.last_hour_timestamp > 3600 {
            self.last_hour_volume = amount;
            self.last_hour_timestamp = timestamp;
        } else {
            self.last_hour_volume = self.last_hour_volume
                .checked_add(amount)
                .unwrap_or(u64::MAX);
        }
    }
}