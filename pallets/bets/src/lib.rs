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
	Percent,
};
use sp_std::prelude::*;
//pub use weights::WeightInfo;


pub type BetIndex = u32;
type AccountIdOf<T> = <T as frame_system::Config>::AccountId;
type BalanceOf<T> = <<T as Config>::Currency as Currency<AccountIdOf<T>>>::Balance;
type BetInfoOf<T> = Bet<AccountIdOf<T>, BalanceOf<T>>;
/// Odd touple composed by integer e fractional part through Percent
type Odd = (u32, u8);

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
	pub odd_homewin: Odd,
	pub odd_awaywin: Odd,
	pub odd_draw: Odd,
	pub odd_under: Odd,
	pub odd_over: Odd,
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
	pub odd: Odd,
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
		/// Fractional part of the Odd out of bound, must be into <0...99> range.
		OddFracPartOutOfBound,
		/// Integer part of the Odd out of bound, must be 1 <= odd.0 <= 4_294_967_295u32.
		OddIntPartOutOfBound,
		/// Match not open for bets or updates, functions available only on open matches.
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
			odd_homewin: Odd,
			odd_awaywin: Odd,
			odd_draw: Odd,
			odd_under: Odd,
			odd_over: Odd,
		) -> DispatchResult {
			// Check that the extrinsic was signed and get the signer.
			// This function will return an error if the extrinsic is not signed.
			let owner = ensure_signed(origin)?;
			// Check fractional part of Odd into <0...99> range.
			ensure!(odd_homewin.1 < 99 && odd_awaywin.1 < 99 && odd_draw.1 < 99 && odd_under.1 < 99 && odd_over.1 < 99, Error::<T>::OddFracPartOutOfBound);
			// Check integer part of Odd >= 1.
			ensure!(odd_homewin.0 > 0 && odd_awaywin.0 > 0 && odd_draw.0 > 0 && odd_under.0 > 0 && odd_over.0 > 0, Error::<T>::OddIntPartOutOfBound);
			// Verify that the specified claim has not already been stored.
			ensure!(!<Matches<T>>::contains_key(id_match), Error::<T>::IdMatchAlreadyExists);

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
			// Store the match with id_match as key.
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
			// Check that the extrinsic was signed and get the signer.
			let bet_owner = ensure_signed(origin)?;
			let bet_index = BetCount::<T>::get();
			// Retrieve the match struct and match_owner
			let selected_match = Self::matches_by_id(id_match).ok_or(Error::<T>::MatchNotExists)?;
			let match_owner = selected_match.owner;
			// Ensure bet owner and match owner are not the same account.
			ensure!(bet_owner != match_owner, Error::<T>::SameMatchOwner);
			// Ensure match is open.
			ensure!(selected_match.status == MatchStatus::Open, Error::<T>::MatchClosed);
			// Ensure that bettor account have suffient free balance.
			ensure!(T::Currency::can_reserve(&bet_owner, amount), Error::<T>::BetAccountInsufficientBalance);

			let odd: Odd = match prediction {
				Prediction::Homewin => selected_match.odd_homewin,
				Prediction::Awaywin => selected_match.odd_awaywin,
				Prediction::Draw => selected_match.odd_draw,
				Prediction::Over => selected_match.odd_over,
				Prediction::Under => selected_match.odd_under,
			};

			let winnable_amount = (Percent::from_percent(odd.1) * amount).saturating_add(amount.saturating_mul((odd.0 as u32).into()));
			// todo: add mod arithmetic for fixed point odd.
			ensure!(T::Currency::can_reserve(&match_owner, winnable_amount), Error::<T>::MatchAccountInsufficientBalance);
			T::Currency::reserve(&bet_owner, amount)?;
			// todo: add mod arithmetic for fixed point odd.
			T::Currency::reserve(&match_owner, winnable_amount)?;

			let bet = Bet {
				owner: bet_owner,
				id_match,
				prediction,
				odd,
				amount,
			};
			// not protected against overflow, see safemath section.
			BetCount::<T>::put(bet_index + 1);
			// insert bet into its storage double map.
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
			// Check if match is open.
			ensure!(selected_match.status == MatchStatus::Open, Error::<T>::MatchClosed);
			let match_owner = selected_match.owner.clone();
			let selected_bets = <Bets<T>>::iter_prefix_values(id_match);
			// Update match status and results.
			// todo: randomize also MatchStatus.
			selected_match.status = MatchStatus::Closed;
			selected_match.home_score = Self::generate_random_score(0);
			selected_match.away_score = Self::generate_random_score(1);
			<Matches<T>>::insert(id_match, selected_match.clone());
			// todo: maybe can try also this way: <Matches<T>>::try_mutate, instead of insert.
			
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
				let winnable_amount = (Percent::from_percent(bet.odd.1) * bet.amount).saturating_add(bet.amount.saturating_mul((bet.odd.0 as u32).into()));
				// Pay off the bet.
				if bet_status == BetStatus::Won {
					let repatriate_result_mtob = T::Currency::repatriate_reserved(&match_owner, &(bet.owner), winnable_amount, BalanceStatus::Free).is_ok();
					let repatriate_result_btom = T::Currency::repatriate_reserved(&(bet.owner), &match_owner, bet.amount, BalanceStatus::Free).is_ok();
					if repatriate_result_mtob == false || repatriate_result_btom == false {
						payoff_result = false;
					}
				} else {
					let repatriate_result = T::Currency::repatriate_reserved(&(bet.owner), &match_owner, bet.amount.clone(), BalanceStatus::Free).is_ok();
					let unreserve_result = T::Currency::unreserve(&match_owner, winnable_amount);
					if repatriate_result == false || !unreserve_result.is_zero() {
						payoff_result = false;
					}
				}
				
				// change bet status and save.
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
	/// generate a random score for a match, some code from an internal function of lottery pallet.
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

	/// generate a random number, internal function from lottery pallet.
	fn generate_random_number(seed: u32) -> u32 {
		let (random_seed, _) = T::Randomness::random(&(T::PalletId::get(), seed).encode());
		let random_number = <u32>::decode(&mut random_seed.as_ref())
			.expect("secure hashes should always be bigger than u32; qed");
		random_number
	}
}
