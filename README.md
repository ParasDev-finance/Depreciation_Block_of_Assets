# IronLedger: Indian Income Tax Act Depreciation Engine

A memory-safe, mathematically verified Rust backend library for calculating "Block of Assets" depreciation under Section 32 and Section 50 of the Indian Income Tax Act (1961).

## ⚠️ Critical System Constraints (Read Before Use)

This engine is designed for strict, mathematically safe ingestion. Version 1.0 enforces the following data rules:

1. **Strict Column Headers:** The CSV parser requires exact header matches. The asset classification column **must** be named exactly `category` (all lowercase). Custom headers, abbreviations (e.g., `buid_b1`, `AssetType`), or capitalized letters will cause the ingestion engine to reject the file to prevent data misalignment.
2. **Manual Date Categorization:** This engine does not parse calendar dates (e.g., `14-Nov-2025`). The 180-day rule is handled via strict column allocation. You must manually split the financial value of new additions into the `additions_more_than_180_days` and `additions_less_than_180_days` columns. 
3. **Boolean Block State:** The physical existence of the block is tracked explicitly via the `is_block_empty` boolean column (`true` or `false`), which automatically triggers Section 50 Short-Term Capital Loss logic if the block is empty but still holds financial value.

---

## 🛡️ Architecture & Security
Built with IT Audit and Fintech security principles in mind:
* **Domain-Driven Design:** Strictly enforces legal depreciation caps (5%, 10%, 15%, 25%, 40%) using bounded Enums to prevent illegal tax rate calculations.
* **Memory-Safe:** Defends against IEEE 754 floating-point corruption (NaN / Infinity injections).
* **Fuzzer Audited:** Core logic is tested against 10,000+ randomized edge-case combinations via `proptest` to mathematically guarantee zero negative Written Down Values (WDV).

## ⚙️ Features
* Calculates Normal Depreciation (including the <180 Days Half-Rate Rule).
* Automatically detects and triggers Section 50 Short-Term Capital Gains (STCG).
* Automatically detects and triggers Section 50 Short-Term Capital Losses (STCL) on empty blocks.

## 📊 Data Ingestion Schema
Your uploaded `.csv` file must **exactly** match this schema:

| category | opening_wdv | additions_more_than_180_days | additions_less_than_180_days | sale_consideration | is_block_empty |
| :--- | :--- | :--- | :--- | :--- | :--- |
| computer | 500000.0 | 0.0 | 0.0 | 600000.0 | false |
| commercial building | 1000000.0 | 0.0 | 500000.0 | 0.0 | false |

*Note: The `category` string is automatically normalized and sanitized upon ingestion. Accepted internal strings include: "computer", "furniture", "residential building", "general machinery", "pollution control", etc.*

## 🚀 Running the Audit Suite
To verify the core mathematical properties and run the fuzzing suite locally, execute the following command in your terminal:
```bash
cargo test
