# Bets Pallet

A simple Substrate pallet that allows each account to play both the role of better and bookmaker.

## Overview

The module allows each user to create a match to bet on and to place bets in matches created by other users, through the following dispatchable functions: 

* **create_match:** Passing as arguments the ID of the external match, and the odds, it creates a match on which to act as a bookmaker and let other users bet on this.
* **place_bet:** Allows a user to bet on an open match. To do this, the user need to select the ID of the match on which bet on, the predicted result and the amount wagered. Once the transaction and the bet have been submitted, an amount equal to the bet one will be reserved in the bettor's account, an amount equal to the bet one multiplied by the established odds will be reserved in the bookmaker's account.
* **set_match_result:** Retrieves the match result and saves it in storage. Subsequently, based on the latter, it scrolls all the bets related to that match and establishes the outcome, unreserving the entire amount of the bet to the winner (bettor or bookmaker). N.B.:
    * This call that can be made by any user at the moment, should be scheduled after the end of the event, saving the end-of-event timestamp among the match data.
    * The retrieval of a match result should be done through HTTP request using an ocw. To simplify this function, the RandomnessCollectiveFlip implementation of Randomness was used to generate the scores of the teams.

## Usage

### Prerequisites
* Rust and the Rust toolchain.
* Substrate node template installed locally

### Installation

Add this code to runtime/src/lib.rs, just before the construct_runtime! Macro
```rust
parameter_types! {
	pub const BetsPalletId: PalletId = PalletId(*b"py/bbets");
}

/// Configure the pallet-bets
impl pallet_bets::Config for Runtime {
	type PalletId = BetsPalletId;
	type Currency = Balances;
	type Event = Event;
	type Randomness = RandomnessCollectiveFlip;
}
```

Add this line to the construct_runtime!, as well as for pallets already present:
```rust
BetsModule: pallet_bets,
```

Add PalletId struct, inserting this line inside  'pub use frame_support::{'
```rust
PalletId,
```

Add the first line to Local Dependencies of runtime's Cargo.toml, then the second into the std feature
```rust
pallet-bets = { version = "1.0.0-dev", default-features = false, git = "https://github.com/mns23/bets.git", branch = "main" }

"pallet-bets/std",
```

## Related Modules

* [`System`](https://docs.rs/frame-system/latest/frame_system/)
* [`Support`](https://docs.rs/frame-support/latest/frame_support/)
* [`pallet-balances`](https://docs.rs/pallet-balances/latest/pallet_balances/)
* [`pallet-lottery`](https://docs.rs/pallet-lottery/latest/pallet_lottery/) for Randomness

License: Unlicense