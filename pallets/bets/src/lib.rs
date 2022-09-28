#![cfg_attr(not(feature = "std"), no_std)]

/// Edit this file to define custom logic or remove it if it is not needed.
/// Learn more about FRAME and the core library of Substrate FRAME pallets:
/// <https://docs.substrate.io/reference/frame-pallets/>
use codec::{Decode, Encode};
use frame_support::{
	dispatch::{DispatchResult, Dispatchable, GetDispatchInfo},
	ensure,
	pallet_prelude::*,
	storage::bounded_vec::BoundedVec,
	traits::{Currency, ExistenceRequirement, Get, ReservableCurrency, WithdrawReasons, BalanceStatus},
	PalletId, RuntimeDebug,
};
pub use pallet::*;
use frame_support::sp_runtime::{
	traits::{AccountIdConversion, Saturating, Zero},
	ArithmeticError, DispatchError,
};
use sp_std::prelude::*;
//pub use weights::WeightInfo;

pub type BetIndex = u32;
type AccountIdOf<T> = <T as frame_system::Config>::AccountId;
type BalanceOf<T> = <<T as Config>::Currency as Currency<AccountIdOf<T>>>::Balance;
type BetInfoOf<T> = Bet<AccountIdOf<T>, BalanceOf<T>>;

#[derive(
	Encode, Decode, Default, Clone, RuntimeDebug, MaxEncodedLen, TypeInfo, PartialEq,
)]
pub enum MatchStatus {
	#[default]
	Open,
	Closed,
	Postponed,
}

#[derive(
	Encode, Decode, Default, RuntimeDebug, MaxEncodedLen, TypeInfo, PartialEq,
)]
pub struct SingleMatch<AccountId> {
	pub owner: AccountId,
	pub id_match: u32,
	pub status: MatchStatus,
	//pub description: String,
	pub home_score: u32,
	pub away_score: u32,
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
	use frame_support::pallet_prelude::*;
	use frame_system::pallet_prelude::*;


	#[pallet::pallet]
	#[pallet::generate_store(pub(super) trait Store)]
	pub struct Pallet<T>(_);

	/// Configure the pallet by specifying the parameters and types on which it depends.
	#[pallet::config]
	pub trait Config: frame_system::Config {
		/// Because this pallet emits events, it depends on the runtime's definition of an event.
		type Event: From<Event<Self>> + IsType<<Self as frame_system::Config>::Event>;

		type Currency: ReservableCurrency<Self::AccountId>;
	}

	// The pallet's runtime storage items.
	// https://docs.substrate.io/main-docs/build/runtime-storage/
	#[pallet::storage]
	#[pallet::getter(fn matches_by_id)]
	pub(super) type Matches <T> =
		StorageMap<_, Blake2_128Concat, u32, SingleMatch<AccountIdOf<T>>, OptionQuery>;
	
	#[pallet::storage]
	#[pallet::getter(fn bets)]
	pub(super) type Bets<T: Config> =
		StorageMap<_, Blake2_128Concat, BetIndex, BetInfoOf<T>, OptionQuery>;

	#[pallet::storage]
	#[pallet::getter(fn bets_count)]
	pub(super) type BetCount<T: Config> = StorageValue<_, BetIndex, ValueQuery>;
	// Learn more about declaring storage items:
	// https://docs.substrate.io/main-docs/build/runtime-storage/#declaring-storage-items
	// pub type Something<T> = StorageValue<_, u32>;

	// Pallets use events to inform users when important changes are made.
	// https://docs.substrate.io/main-docs/build/events-errors/
	#[pallet::event]
	#[pallet::generate_deposit(pub(super) fn deposit_event)]
	pub enum Event<T: Config> {
		MatchCreated(u32),
		BetPlaced(BetIndex),
		MatchClosed(u32),
		/// Event emitted when a claim has been created.
		ClaimCreated { who: T::AccountId, claim: T::Hash },
		/// Event emitted when a claim is revoked by the owner.
		ClaimRevoked { who: T::AccountId, claim: T::Hash },
	}

	// Errors inform users that something went wrong.
	#[pallet::error]
	pub enum Error<T> {
		MatchNotExists,
		NoBetExists,
		/// The claim already exists.
		AlreadyClaimed,
		/// The claim does not exist, so it cannot be revoked.
		NoSuchClaim,
		/// The claim is owned by another account, so caller can't revoke it.
		NotClaimOwner,
	}

	// Dispatchable functions allow users to interact with the pallet and invoke state changes.
	// These functions materialize as "extrinsics", which are often compared to transactions.
	// Dispatchable functions must be annotated with a weight and must return a DispatchResult.
	#[pallet::call]
	impl<T: Config> Pallet<T> {
		#[pallet::weight(10_000)]
		pub fn create_match(
			origin: OriginFor<T>,
			id_match: u32,
			status: MatchStatus,
			home_score: u32,
			away_score: u32,
		) -> DispatchResult {
			// Check that the extrinsic was signed and get the signer.
			// This function will return an error if the extrinsic is not signed.
			let owner = ensure_signed(origin)?;

			// Verify that the specified claim has not already been stored.
			//ensure!(!Claims::<T>::contains_key(&claim), Error::<T>::AlreadyClaimed);

			// Get the block number from the FRAME System pallet.
			//let current_block = <frame_system::Pallet<T>>::block_number();
			let single_match = SingleMatch {
				owner,
				id_match,
				status,
				home_score,
				away_score,
			};
			// Store the claim with the sender and block number.
			<Matches<T>>::insert(id_match, single_match);

			Self::deposit_event(Event::MatchCreated(id_match));

			Ok(())
		}

		#[pallet::weight(10_000)]
		pub fn place_bet(
			origin: OriginFor<T>,
			id_match: u32,
			prediction: Prediction,
			odd: u32,
			amount: BalanceOf<T>,
		) -> DispatchResult {
			let bet_owner = ensure_signed(origin)?;

			//ensure!(end > now, <Error<T>>::EndTooEarly);

			let index = BetCount::<T>::get();
			// not protected against overflow, see safemath section
			BetCount::<T>::put(index + 1);


			let selected_match = Self::matches_by_id(id_match).ok_or(Error::<T>::MatchNotExists)?;
			let match_owner = selected_match.owner;

			// T::Currency::transfer(
			// 	&bet_owner,
			// 	&match_owner,
			// 	amount,
			// 	ExistenceRequirement::AllowDeath,
			// )?;

			T::Currency::reserve(&bet_owner, amount)?;
			//todo: multiply amount by odd
			T::Currency::reserve(&match_owner, amount)?;

			let bet = Bet {
				owner: bet_owner,
				id_match,
				prediction,
				odd,
				amount,
			};

			<Bets<T>>::insert(index,bet);

			Self::deposit_event(Event::BetPlaced(index));
			Ok(().into())
		}

		#[pallet::weight(10_000)]
		pub fn end_match(
			origin: OriginFor<T>,
			id_match: u32,
			status: MatchStatus,
		) -> DispatchResult {
			let selected_match = Self::matches_by_id(id_match).ok_or(Error::<T>::MatchNotExists)?;
			let match_owner = selected_match.owner;
			let index = BetCount::<T>::get();
			let selected_bet = Self::bets(0).ok_or(Error::<T>::NoBetExists)?;

			// Move the deposit to the new owner.
			T::Currency::repatriate_reserved(&(selected_bet.owner), &match_owner, selected_bet.amount, BalanceStatus::Free)?;

			Self::deposit_event(Event::MatchClosed(index));
			Ok(().into())
		}

	}
}
