use anchor_lang::prelude::*;
use crate::constants::{
    MAX_SLIPPAGE, 
    MEDIUM_TRADE_THRESHOLD, 
    SMALL_TRADE_THRESHOLD, 
    VOLUME_THRESHOLD
};
use crate::errors::HypeBondError;
use crate::constant;

#[derive(AnchorSerialize, AnchorDeserialize, Clone, PartialEq, Eq)]
pub enum CurveState {
    PriceDiscovery,  // Initial phase - price decreases over time
    BondingCurve     // Second phase - traditional x*y=k bonding curve
}

impl Default for CurveState {
    fn default() -> Self {
        CurveState::PriceDiscovery
    }
}

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
    
    // Doppler two-phase approach parameters
    pub curve_state: CurveState,
    pub discovery_start_time: i64,
    pub discovery_duration: i64,
    pub discovery_start_price: u64,
    pub discovery_end_price: u64,
    pub clearing_price: u64,
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

    // Get current price in discovery phase (decreasing over time)
    pub fn get_discovery_price(&self, current_time: i64) -> Result<u64> {
        let elapsed = current_time.saturating_sub(self.discovery_start_time);
        
        // If discovery phase is complete, return the end price
        if elapsed >= self.discovery_duration {
            return Ok(self.discovery_end_price);
        }
        
        // Calculate price decay based on elapsed time (linear decay)
        let price_range = self.discovery_start_price.saturating_sub(self.discovery_end_price);
        let price_decay = price_range
            .checked_mul(elapsed as u64)
            .unwrap_or(0)
            .checked_div(self.discovery_duration as u64)
            .unwrap_or(0);
        
        Ok(self.discovery_start_price.saturating_sub(price_decay))
    }

    // Check and update curve state if needed
    pub fn check_and_update_state(&mut self, current_time: i64) -> Result<()> {
        if self.curve_state == CurveState::PriceDiscovery && 
           current_time >= self.discovery_start_time + self.discovery_duration {
            
            // Transition to bonding curve phase
            self.curve_state = CurveState::BondingCurve;
            self.clearing_price = self.discovery_end_price;
            
            // Initialize virtual reserves to establish the k constant
            // that gives us our clearing price as the starting point
            let initial_token_amount = self.token_total_supply / 10; // 10% of supply
            
            // Set virtual reserves to create the desired k constant
            self.virtual_token_reserves = self.token_total_supply - self.real_token_reserves;
            
            // Calculate virtual SOL reserves based on clearing price
            // Formula: sol_reserves = (token_reserves * token_price) / amount_to_buy_for_1_sol
            // We're setting up k so that buying 1 SOL worth would cost the clearing price
            let sol_per_token = 1_000_000_000 / self.clearing_price; // lamports per token
            self.virtual_sol_reserves = self.virtual_token_reserves
                .checked_mul(sol_per_token)
                .unwrap_or(0)
                .checked_div(1_000_000_000)
                .unwrap_or(0);
            
            msg!("Transitioned to bonding curve phase. Clearing price: {}", self.clearing_price);
        }
        
        Ok(())
    }

    // Apply protection multipliers to a base amount
    pub fn apply_protections(&self, base_amount: u64, amount: u64) -> Result<u64> {
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
            .unwrap_or(base_amount)
            .checked_mul(dynamic_multiplier)
            .unwrap_or(base_amount)
            .checked_div(10000)
            .ok_or(HypeBondError::MathOverflow.into())
    }

    // Calculate buy price with all protections - now phase-aware
    pub fn calculate_buy_price(&self, amount: u64, current_time: i64) -> Result<u64> {
        match self.curve_state {
            CurveState::PriceDiscovery => {
                // In discovery phase, price decreases over time
                let token_price = self.get_discovery_price(current_time)?;
                
                // Total cost for amount of tokens at this price
                let base_sol_cost = amount
                    .checked_mul(token_price)
                    .unwrap_or(0)
                    .checked_div(1_000_000_000) // Convert from lamports
                    .unwrap_or(0);
                
                // Apply protections
                self.apply_protections(base_sol_cost, amount)
            },
            CurveState::BondingCurve => {
                // In bonding curve phase, use constant product formula
                let base_amount = self.get_base_price(amount, true)?;
                self.apply_protections(base_amount, amount)
            }
        }
    }

    // Protective sell price calculation - now phase-aware
    pub fn calculate_sell_price(&self, amount: u64, current_time: i64) -> Result<u64> {
        match self.curve_state {
            CurveState::PriceDiscovery => {
                // In discovery phase, sell at current price with penalty
                let token_price = self.get_discovery_price(current_time)?;
                
                // Apply a fixed 5% penalty on discovery phase sells
                let base_sol_receive = amount
                    .checked_mul(token_price)
                    .unwrap_or(0)
                    .checked_mul(95) // 5% penalty
                    .unwrap_or(0)
                    .checked_div(100)
                    .unwrap_or(0)
                    .checked_div(1_000_000_000) // Convert from lamports
                    .unwrap_or(0);
                
                // Apply standard protections
                Ok(base_sol_receive)
            },
            CurveState::BondingCurve => {
                // In bonding curve phase, use constant product formula
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
                    .unwrap_or(base_amount)
                    .checked_mul(dynamic_multiplier)
                    .unwrap_or(base_amount)
                    .checked_div(10000)
                    .ok_or(HypeBondError::MathOverflow.into())
            }
        }
    }

    // Update reserves after trade
    pub fn update_reserves(
        &mut self,
        token_amount: u64,
        sol_amount: u64,
        is_buy: bool,
    ) -> Result<()> {
        // In discovery phase, we only update real reserves
        if self.curve_state == CurveState::PriceDiscovery {
            if is_buy {
                self.real_token_reserves = self.real_token_reserves
                    .checked_sub(token_amount)
                    .ok_or(HypeBondError::MathOverflow)?;
                
                self.real_sol_reserves = self.real_sol_reserves
                    .checked_add(sol_amount)
                    .ok_or(HypeBondError::MathOverflow)?;
            } else {
                self.real_token_reserves = self.real_token_reserves
                    .checked_add(token_amount)
                    .ok_or(HypeBondError::MathOverflow)?;
                
                self.real_sol_reserves = self.real_sol_reserves
                    .checked_sub(sol_amount)
                    .ok_or(HypeBondError::MathOverflow)?;
            }
            return Ok(());
        }
        
        // In bonding curve phase, update both virtual and real reserves
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
    
    // Initialize curve with discovery phase parameters
    pub fn initialize_discovery(
        &mut self,
        start_time: i64,
        duration: i64,
        start_price: u64,
        end_price: u64,
        total_supply: u64,
    ) -> Result<()> {
        self.curve_state = CurveState::PriceDiscovery;
        self.discovery_start_time = start_time;
        self.discovery_duration = duration;
        self.discovery_start_price = start_price;
        self.discovery_end_price = end_price;
        self.token_total_supply = total_supply;
        self.real_token_reserves = total_supply;
        
        Ok(())
    }
}