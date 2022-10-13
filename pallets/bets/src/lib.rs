//! # Bets Pallet
//!
//! A simple Substrate pallet that allows each account to play both the role of better and bookmaker.
//!
//! ## Overview
//!
//! The module allows each user to create a match to bet on and to place bets in matches created by other users,
//! through the following dispatchable functions: 
//!
//! * **create_match:** Passing as arguments the ID of the external match, and the odds,
//! 	it creates a match on which to act as a bookmaker and let other users bet on this.
//! * **place_bet:** Allows a user to bet on an open match. To do this, the user need to select the ID of the match
//! 	on which bet on, the predicted result and the amount wagered. Once the transaction and the bet have been submitted,
//! 	an amount equal to the bet one will be reserved in the bettor's account, an amount equal to the bet one multiplied
//! 	by the established odds will be reserved in the bookmaker's account.
//! * **set_match_result:** Retrieves the match result and saves it in storage. Subsequently, based on the latter,
//! 	it scrolls all the bets related to that match and establishes the outcome, unreserving the entire amount of the bet
//! 	to the winner (bettor or bookmaker). N.B.:
//!     	* This call that can be made by any user at the moment, should be scheduled after the end of the event,
//! 		saving the end-of-event timestamp among the match data.
//!     	* The retrieval of a match result should be done through HTTP request using an ocw. To simplify this function,
//! 		the RandomnessCollectiveFlip implementation of Randomness was used to generate the scores of the teams.


#![cfg_attr(not(feature = "std"), no_std)]

#[cfg(test)]
mod mock;
#[cfg(test)]
mod tests;

use codec::{Decode, Encode};
use frame_support::{
	dispatch::{DispatchResult},
	ensure,
	pallet_prelude::*,
	traits::{Currency, Get, ReservableCurrency, BalanceStatus, Randomness},
	PalletId, RuntimeDebug,
};
pub use pallet::*;
use frame_support::sp_runtime::{
	traits::{Saturating, Zero},
};
use sp_std::prelude::*;
//pub use weights::WeightInfo;


pub type BetIndex = u32;
type AccountIdOf<T> = <T as frame_system::Config>::AccountId;
type BalanceOf<T> = <<T as Config>::Currency as Currency<AccountIdOf<T>>>::Balance;
type BetInfoOf<T> = Bet<AccountIdOf<T>, BalanceOf<T>>;

#[derive(
	Encode, Decode, Default, Clone, RuntimeDebug, MaxEncodedLen, TypeInfo, PartialEq, Copy,
)]

/// A Match have an initial state (Open), and 2 final states (Closed, Postponed).
pub enum MatchStatus {
	#[default]
	Open,
	Closed,
	Postponed,
}

#[derive(
	Encode, Decode, Default, RuntimeDebug, MaxEncodedLen, TypeInfo, PartialEq, Clone, Copy,
)]
pub struct SingleMatch<AccountId> {
	pub owner: AccountId,
	pub id_match: u32,
	pub status: MatchStatus,
	pub home_score: u32,
	pub away_score: u32,
	pub odd_homewin: u32,
	pub odd_awaywin: u32,
	pub odd_draw: u32,
	pub odd_under: u32,
	pub odd_over: u32,
}

#[derive(
	Encode, Decode, Default, Clone, RuntimeDebug, MaxEncodedLen, TypeInfo, PartialEq,
)]
pub enum Prediction {
	#[default]
	Homewin,
	Awaywin,
	Draw,
	Under,
	Over,
}

#[derive(
	Encode, Decode, Default, Clone, RuntimeDebug, MaxEncodedLen, TypeInfo, PartialEq, Copy,
)]
pub enum BetStatus {
	#[default]
	Lost,
	Won,
}

#[derive(
	Encode, Decode, Default, RuntimeDebug, MaxEncodedLen, TypeInfo, PartialEq,
)]
pub struct Bet<AccountId, Balance> {
	pub owner: AccountId,
	pub id_match: u32,
	pub prediction: Prediction,
	pub odd: u32,
	pub amount: Balance,
}

#[frame_support::pallet]
pub mod pallet {
	use super::*;
	use frame_system::pallet_prelude::*;

	#[pallet::pallet]
	#[pallet::generate_store(pub(super) trait Store)]
	pub struct Pallet<T>(_);

	#[pallet::config]
	pub trait Config: frame_system::Config {
		/// The bets pallet id.
		#[pallet::constant]
		type PalletId: Get<PalletId>;
		/// Event type.
		type Event: From<Event<Self>> + IsType<<Self as frame_system::Config>::Event>;
		/// The currency mechanism.
		type Currency: ReservableCurrency<Self::AccountId>;
		/// Something that provides randomness in the runtime.
		type Randomness: Randomness<Self::Hash, Self::BlockNumber>;
	}

	#[pallet::storage]
	/// Mapping matches using its id as key.
	#[pallet::getter(fn matches_by_id)]
	pub(super) type Matches <T> =
		StorageMap<_, Blake2_128Concat, u32, SingleMatch<AccountIdOf<T>>, OptionQuery>;
	
	#[pallet::storage]
	/// A Storage Double Map of bets. Referenced by the id match to which it refers
	/// (to quickly find all the bets related to a specific match), and its index.
	#[pallet::getter(fn bets)]
	pub(super) type Bets<T: Config> =
		StorageDoubleMap<_, Blake2_128Concat, u32, Blake2_128Concat, BetIndex, BetInfoOf<T>, OptionQuery>;

	#[pallet::storage]
	/// Auto-incrementing bet counter
	#[pallet::getter(fn bets_count)]
	pub(super) type BetCount<T: Config> = StorageValue<_, BetIndex, ValueQuery>;

	#[pallet::event]
	#[pallet::generate_deposit(pub(super) fn deposit_event)]
	pub enum Event<T: Config> {
		/// A match was created.
		MatchCreated(u32),
		/// A bet was placed.
		BetPlaced(BetIndex),
		/// A match was closed.
		MatchClosed(u32),
	}

	#[pallet::error]
	pub enum Error<T> {
		/// Cannot create a match with an already existing id_match.
		IdMatchAlreadyExists,
		/// A specific match does not exist, cannot place a bet or set match result.
		MatchNotExists,
		/// Bet owner and match owner must be different.
		SameMatchOwner,
		/// Match not open for bets or updates, functions available only on open matches
		MatchClosed,
		/// Insufficient free-balance to offer a bet.
		MatchAccountInsufficientBalance,
		/// Insufficient free-balance to place a bet.
		BetAccountInsufficientBalance,
		/// No bet associated with a specific match exists.
		NoBetExists,
		/// Payoff procedure failed.
		PayoffError,
	}

	#[pallet::call]
	impl<T: Config> Pallet<T> {
		/// Passing as arguments the ID of the external match, and the odds,
		/// it creates a match on which to act as a bookmaker and let other users bet on this.
		#[pallet::weight(10_000)]
		pub fn create_match(
			origin: OriginFor<T>,
			id_match: u32,
			odd_homewin: u32,
			odd_awaywin: u32,
			odd_draw: u32,
			odd_under: u32,
			odd_over: u32,
		) -> DispatchResult {
			// Check that the extrinsic was signed and get the signer.
			// This function will return an error if the extrinsic is not signed.
			let owner = ensure_signed(origin)?;

			// Verify that the specified claim has not already been stored.
			ensure!(!<Matches<T>>::contains_key(id_match), Error::<T>::IdMatchAlreadyExists);

			// Get the block number from the FRAME System pallet.
			//let current_block = <frame_system::Pallet<T>>::block_number();
			let single_match = SingleMatch {
				owner,
				id_match,
				status: MatchStatus::Open,
				home_score: 0, 
				away_score: 0,
				odd_homewin,
				odd_awaywin,
				odd_draw,
				odd_under,
				odd_over,
			};
			// Store the claim with the sender and block number.
			<Matches<T>>::insert(id_match, single_match);

			Self::deposit_event(Event::MatchCreated(id_match));

			Ok(())
		}

		/// Allows a user to bet on an open match. To do this, the user need to select the ID of the match
		/// on which bet on, the predicted result and the amount wagered. Once the transaction and the bet have been submitted,
		/// an amount equal to the bet one will be reserved in the bettor's account, an amount equal to the bet one multiplied
		/// by the established odds will be reserved in the bookmaker's account.
		#[pallet::weight(10_000)]
		pub fn place_bet(
			origin: OriginFor<T>,
			id_match: u32,
			prediction: Prediction,
			amount: BalanceOf<T>,
		) -> DispatchResult {
			let bet_owner = ensure_signed(origin)?;
			ensure!(T::Currency::can_reserve(&bet_owner, amount), Error::<T>::BetAccountInsufficientBalance);
			let bet_index = BetCount::<T>::get();
			let selected_match = Self::matches_by_id(id_match).ok_or(Error::<T>::MatchNotExists)?;
			let match_owner = selected_match.owner;
			ensure!(bet_owner != match_owner, Error::<T>::SameMatchOwner);
			ensure!(selected_match.status == MatchStatus::Open, Error::<T>::MatchClosed);

			// T::Currency::transfer(
			// 	&bet_owner,
			// 	&match_owner,
			// 	amount,
			// 	ExistenceRequirement::AllowDeath,
			// )?;
			let odd: u32 = match prediction {
				Prediction::Homewin => selected_match.odd_homewin,
				Prediction::Awaywin => selected_match.odd_awaywin,
				Prediction::Draw => selected_match.odd_draw,
				Prediction::Over => selected_match.odd_over,
				Prediction::Under => selected_match.odd_under,
			};

			//todo: add mod arithmetic for fixed point odd.
			ensure!(T::Currency::can_reserve(&match_owner, amount.saturating_mul((odd as u32).into())), Error::<T>::MatchAccountInsufficientBalance);
			T::Currency::reserve(&bet_owner, amount)?;
			//todo: add mod arithmetic for fixed point odd.
			T::Currency::reserve(&match_owner, amount.saturating_mul((odd as u32).into()))?;

			let bet = Bet {
				owner: bet_owner,
				id_match,
				prediction,
				odd,
				amount,
			};
			// not protected against overflow, see safemath section.
			BetCount::<T>::put(bet_index + 1);
			<Bets<T>>::insert(id_match, bet_index,bet);

			Self::deposit_event(Event::BetPlaced(bet_index));
			Ok(().into())
		}

		/// Retrieves the match result and saves it in storage. Subsequently, based on the latter,
		/// it scrolls all the bets related to that match and establishes the outcome, unreserving the entire amount of the bet
		/// to the winner (bettor or bookmaker). N.B.:
		///     * This call that can be made by any user at the moment, should be scheduled after the end of the event,
		/// 	saving the end-of-event timestamp among the match data.
		///     * The retrieval of a match result should be done through HTTP request using an ocw. To simplify this function,
		/// 	the RandomnessCollectiveFlip implementation of Randomness was used to generate the scores of the teams.
		#[pallet::weight(10_000)]
		pub fn set_match_result(
			origin: OriginFor<T>,
			id_match: u32,
		) -> DispatchResult {
			let _who = ensure_signed(origin)?;
			let mut selected_match = Self::matches_by_id(id_match).ok_or(Error::<T>::MatchNotExists)?;
			ensure!(selected_match.status == MatchStatus::Open, Error::<T>::MatchClosed);
			let match_owner = selected_match.owner.clone();
			let selected_bets = <Bets<T>>::iter_prefix_values(id_match);

			// Update match status and results.
			// todo: randomize also MatchStatus.
			selected_match.status = MatchStatus::Closed;
			selected_match.home_score = Self::generate_random_score(0);
			selected_match.away_score = Self::generate_random_score(1);
			<Matches<T>>::insert(id_match, selected_match.clone());
			// <Matches<T>>::try_mutate(id_match, |matchh| {
			
			// Check the winning status of the bet compared to match results.
			let mut payoff_result = true;
			selected_bets.for_each(|bet|{
				let bet_status: BetStatus = match &bet.prediction {
					Prediction::Homewin if selected_match.home_score > selected_match.away_score => BetStatus::Won,
					Prediction::Awaywin if selected_match.home_score < selected_match.away_score => BetStatus::Won,
					Prediction::Draw if selected_match.home_score == selected_match.away_score => BetStatus::Won,
					Prediction::Over if selected_match.home_score + selected_match.away_score > 3 => BetStatus::Won,
					Prediction::Under if selected_match.home_score + selected_match.away_score < 3 => BetStatus::Won,
					_ => BetStatus::Lost,
				};
				
				// Pay off the bet.
				if bet_status == BetStatus::Won {
					let repatriate_result_mtob = T::Currency::repatriate_reserved(&match_owner, &(bet.owner), bet.amount.saturating_mul((bet.odd as u32).into()), BalanceStatus::Free).is_ok();
					let repatriate_result_btom = T::Currency::repatriate_reserved(&(bet.owner), &match_owner, bet.amount, BalanceStatus::Free).is_ok();
					if repatriate_result_mtob == false || repatriate_result_btom == false {
						payoff_result = false;
					}
				} else {
					let repatriate_result = T::Currency::repatriate_reserved(&(bet.owner), &match_owner, bet.amount.clone(), BalanceStatus::Free).is_ok();
					let unreserve_result = T::Currency::unreserve(&match_owner, bet.amount.saturating_mul((bet.odd as u32).into()));
					if repatriate_result == false || !unreserve_result.is_zero() {
						payoff_result = false;
					}
				}
				
				//change bet status and save.
				//bet.status = new_bet_status;
				//let key : u8 = selected_bets.last_raw_key();
				//<Bets<T>>::insert(id_match, selected_bets.last_raw_key(),bet);
			});
			ensure!(payoff_result == true, Error::<T>::PayoffError);
			Self::deposit_event(Event::MatchClosed(id_match));
			Ok(().into())
		}

	}
}

impl<T: Config> Pallet<T> {
	///internal function from lottery pallet.
	fn generate_random_score(seed_diff: u32) -> u32 {
		let mut random_number = Self::generate_random_number(seed_diff);
		let max_trials: u32 = 10;
		let max_score: u32 = 9;

		// Best effort attempt to remove bias from modulus operator.
		for i in 1..max_trials {
			if random_number < u32::MAX - u32::MAX % max_score {
				break
			}
			random_number = Self::generate_random_number(seed_diff + i);
		}

		random_number % max_score
	}

	///internal function from lottery pallet.
	fn generate_random_number(seed: u32) -> u32 {
		let (random_seed, _) = T::Randomness::random(&(T::PalletId::get(), seed).encode());
		let random_number = <u32>::decode(&mut random_seed.as_ref())
			.expect("secure hashes should always be bigger than u32; qed");
		random_number
	}
}
