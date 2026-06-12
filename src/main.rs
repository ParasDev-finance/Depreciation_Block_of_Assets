use serde::Deserialize;
use std::error::Error;

// ==============================================================================
// 1. ASSET CLASSES & LEGAL DEPRECIATION RATES (Appendix I, IT Rules)
// ==============================================================================

#[derive(Debug, PartialEq, Clone, Copy)]
pub enum BuildingType {
    Residential,
    Commercial,
    Temporary,
}

#[derive(Debug, PartialEq, Clone, Copy)]
pub enum PlantType {
    General,
    MotorCar,
    Computer,
    PollutionControl,
}

#[derive(Debug, PartialEq, Clone, Copy)]
pub enum AssetClass {
    Building(BuildingType),
    FurnitureAndFittings,
    PlantAndMachinery(PlantType),
    Intangible,
}

impl AssetClass {
    // Master rate lookup table
    pub fn get_rate(&self) -> f64 {
        match self {
            AssetClass::Building(b) => match b {
                BuildingType::Residential => 0.05,
                BuildingType::Commercial => 0.10,
                BuildingType::Temporary => 0.40,
            },
            AssetClass::FurnitureAndFittings => 0.10,
            AssetClass::PlantAndMachinery(p) => match p {
                PlantType::General => 0.15,
                PlantType::MotorCar => 0.15,
                PlantType::Computer => 0.40,
                PlantType::PollutionControl => 0.40,
            },
            AssetClass::Intangible => 0.25,
        }
    }

    // Sanitization layer for external text injection
    pub fn from_csv_string(raw: &str) -> Result<AssetClass, String> {
        match raw.trim().to_lowercase().as_str() {
            "residential building" => Ok(AssetClass::Building(BuildingType::Residential)),
            "commercial building" => Ok(AssetClass::Building(BuildingType::Commercial)),
            "temporary building" => Ok(AssetClass::Building(BuildingType::Temporary)),
            "furniture" => Ok(AssetClass::FurnitureAndFittings),
            "general machinery" => Ok(AssetClass::PlantAndMachinery(PlantType::General)),
            "motor car" => Ok(AssetClass::PlantAndMachinery(PlantType::MotorCar)),
            "computer" => Ok(AssetClass::PlantAndMachinery(PlantType::Computer)),
            "pollution control" => Ok(AssetClass::PlantAndMachinery(PlantType::PollutionControl)),
            "intangible" => Ok(AssetClass::Intangible),
            _ => Err(format!(
                "CRITICAL VALIDATION ERROR: Unknown asset category '{}'",
                raw
            )),
        }
    }
}

#[derive(Debug, PartialEq, Clone, Copy)]
pub enum UsagePeriod {
    MoreThan180Days,
    LessThan180Days,
}

// ==============================================================================
// 2. CORE DOMAIN STRUCTS
// ==============================================================================

#[derive(Debug, Clone)]
pub struct AssetBlock {
    pub class: AssetClass,

    // Financial Ledgers
    pub opening_wdv: f64,
    pub additions_more_than_180_days: f64,
    pub additions_less_than_180_days: f64,
    pub sale_consideration: f64,

    // Physical State
    pub is_block_empty: bool,
}

#[derive(Debug)]
pub struct DepreciationResult {
    pub normal_depreciation: f64,
    pub closing_wdv: f64,
    pub short_term_capital_gain: f64,
    pub short_term_capital_loss: f64,
}

// ==============================================================================
// 3. MATHEMATICAL ENGINE
// ==============================================================================

impl AssetBlock {
    // Guardrail against IEEE 754 floating-point poison attacks (NaN/Infinity)
    fn is_memory_corrupted(&self) -> bool {
        !self.opening_wdv.is_finite()
            || !self.additions_more_than_180_days.is_finite()
            || !self.additions_less_than_180_days.is_finite()
            || !self.sale_consideration.is_finite()
    }

    pub fn calculate_depreciation(&self) -> Result<DepreciationResult, String> {
        if self.is_memory_corrupted() {
            return Err(String::from(
                "SECURITY ALERT: Corrupted memory (NaN or Infinity) detected in financial data.",
            ));
        }

        let total_additions = self.additions_more_than_180_days + self.additions_less_than_180_days;
        let total_pool_before_dep = self.opening_wdv + total_additions;

        // Scenario 1: Section 50 Short-Term Capital Gain
        if self.sale_consideration > total_pool_before_dep {
            return Ok(DepreciationResult {
                normal_depreciation: 0.0,
                closing_wdv: 0.0,
                short_term_capital_gain: self.sale_consideration - total_pool_before_dep,
                short_term_capital_loss: 0.0,
            });
        }

        let remaining_wdv_after_sale = total_pool_before_dep - self.sale_consideration;

        // Scenario 2: Section 50 Short-Term Capital Loss
        if self.is_block_empty {
            return Ok(DepreciationResult {
                normal_depreciation: 0.0,
                closing_wdv: 0.0,
                short_term_capital_gain: 0.0,
                short_term_capital_loss: remaining_wdv_after_sale,
            });
        }

        // Scenario 3: Normal Depreciation Calculation
        let mut normal_dep = 0.0;
        let full_rate_pool = self.opening_wdv + self.additions_more_than_180_days;

        if self.sale_consideration <= full_rate_pool {
            let balance_for_full_rate = full_rate_pool - self.sale_consideration;
            normal_dep += balance_for_full_rate * self.class.get_rate();
            normal_dep += self.additions_less_than_180_days * (self.class.get_rate() / 2.0);
        } else {
            normal_dep += remaining_wdv_after_sale * (self.class.get_rate() / 2.0);
        }

        Ok(DepreciationResult {
            normal_depreciation: normal_dep,
            closing_wdv: remaining_wdv_after_sale - normal_dep,
            short_term_capital_gain: 0.0,
            short_term_capital_loss: 0.0,
        })
    }
}

// ==============================================================================
// 4. DATA INGESTION LAYER
// ==============================================================================

#[derive(Debug, Deserialize)]
pub struct RawAssetRecord {
    pub category: String,
    pub opening_wdv: f64,
    pub additions_more_than_180_days: f64,
    pub additions_less_than_180_days: f64,
    pub sale_consideration: f64,
    pub is_block_empty: bool,
}

pub fn ingest_csv_data(csv_data: &str) -> Result<Vec<AssetBlock>, Box<dyn Error>> {
    let mut reader = csv::Reader::from_reader(csv_data.as_bytes());
    let mut verified_blocks = Vec::new();

    for result in reader.deserialize() {
        let raw_record: RawAssetRecord = result?;
        let strict_class = AssetClass::from_csv_string(&raw_record.category)?;

        let safe_block = AssetBlock {
            class: strict_class,
            opening_wdv: raw_record.opening_wdv,
            additions_more_than_180_days: raw_record.additions_more_than_180_days,
            additions_less_than_180_days: raw_record.additions_less_than_180_days,
            sale_consideration: raw_record.sale_consideration,
            is_block_empty: raw_record.is_block_empty,
        };

        verified_blocks.push(safe_block);
    }

    Ok(verified_blocks)
}

// ==============================================================================
// 5. APPLICATION ENTRY
// ==============================================================================

fn main() {
    println!("Welcome to the IronLedger Depreciation Engine!");
}


// ==============================================================================
// 6. RED TEAM: FUZZING & SECURITY AUDIT SUITE
// ==============================================================================

#[cfg(test)]
mod tests {
    // Import everything from the main file
    use super::*;
    
    // Import the fuzzer tools
    use proptest::prelude::*;
    use proptest::num::f64::ANY;

    proptest! {
        // Force the fuzzer to run 10,000 times instead of the default 256
        #![proptest_config(ProptestConfig::with_cases(10_000))]
        
        // ----------------------------------------------------------------------
        // AUDIT 1: Standard Mathematical Constraints
        // ----------------------------------------------------------------------
        #[test]
        fn fuzz_test_wdv_is_never_negative(
            // Generate random floats between 0 and 1 Billion
            random_opening in 0.0f64..1_000_000_000.0,
            random_addition in 0.0f64..1_000_000_000.0,
            random_sale in 0.0f64..2_000_000_000.0,
        ) {
            let block = AssetBlock {
                class: AssetClass::PlantAndMachinery(PlantType::General),
                opening_wdv: random_opening,
                additions_more_than_180_days: random_addition,
                additions_less_than_180_days: 0.0,
                sale_consideration: random_sale,
                is_block_empty: false,
            };

            let result = block.calculate_depreciation();

            // Only test the math if the engine accepted the inputs as valid
            if let Ok(safe_result) = result {
                prop_assert!(safe_result.closing_wdv >= 0.0, "CRITICAL VULNERABILITY: WDV dropped below zero!");
                prop_assert!(safe_result.normal_depreciation >= 0.0, "CRITICAL VULNERABILITY: Negative depreciation calculated!");
            }
        }

        // ----------------------------------------------------------------------
        // AUDIT 2: IEEE 754 Memory Corruption Attack (NaN / Infinity)
        // ----------------------------------------------------------------------
        #[test]
        fn fuzz_test_survives_memory_corruption(
            // ANY strategy injects pure chaos, including NaN and +/- Infinity
            random_opening in ANY,
            random_addition in ANY,
            random_sale in ANY,
        ) {
            let block = AssetBlock {
                class: AssetClass::Building(BuildingType::Commercial),
                opening_wdv: random_opening,
                additions_more_than_180_days: random_addition,
                additions_less_than_180_days: 0.0,
                sale_consideration: random_sale,
                is_block_empty: false,
            };

            let result = block.calculate_depreciation();

            // If the engine fails to block the poison, it returns Ok() with corrupted data.
            // The assertion will then evaluate NaN >= 0.0 (which is false) and trigger the alarm.
            if let Ok(safe_result) = result {
                prop_assert!(safe_result.closing_wdv >= 0.0, "CRITICAL: Engine failed to block negative or NaN values!");
                prop_assert!(safe_result.normal_depreciation >= 0.0, "CRITICAL: Engine failed to block negative or NaN values!");
            }
        }
    }
}