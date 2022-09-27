#![cfg_attr(not(feature = "std"), no_std)]

/// Edit this file to define custom logic or remove it if it is not needed.
/// Learn more about FRAME and the core library of Substrate FRAME pallets:
/// <https://docs.substrate.io/reference/frame-pallets/>
use frame_support::pallet_prelude::*;

pub use pallet::*;

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
pub struct SingleMatch {
	pub id_match: u32,
	pub status: MatchStatus,
	//pub description: String,
	pub home_score: u32,
	pub away_score: u32,
}
// use codec::{Decode, Encode};
// use frame_support::{
// 	dispatch::{DispatchResult, Dispatchable, GetDispatchInfo},
// 	ensure,
// 	pallet_prelude::MaxEncodedLen,
// 	storage::bounded_vec::BoundedVec,
// 	traits::{Currency, ExistenceRequirement::KeepAlive, Get, Randomness, ReservableCurrency},
// 	PalletId, RuntimeDebug,
// };
// use sp_runtime::{
// 	traits::{AccountIdConversion, Saturating, Zero},
// 	ArithmeticError, DispatchError,
// };
// use sp_std::prelude::*;
// pub use weights::WeightInfo;


// pub enum Prediction {
// 	Homewin,
// 	Awaywin,
// 	Draw,
// 	Under,
// 	Over,
// }

// #[derive(
// 	Encode, Decode, Default, RuntimeDebug
// )]
// pub struct Bet<Balance> {
// 	pub id_match: u32,
// 	pub prediction: u32,
// 	pub odd: u32,
// 	pub amount: Balance,
// }


#[frame_support::pallet]
pub mod pallet {
	use super::*;
	//use frame_support::pallet_prelude::*;
	use frame_system::pallet_prelude::*;


	#[pallet::pallet]
	#[pallet::generate_store(pub(super) trait Store)]
	pub struct Pallet<T>(_);

	/// Configure the pallet by specifying the parameters and types on which it depends.
	#[pallet::config]
	pub trait Config: frame_system::Config {
		/// Because this pallet emits events, it depends on the runtime's definition of an event.
		type Event: From<Event<Self>> + IsType<<Self as frame_system::Config>::Event>;
	}

	// The pallet's runtime storage items.
	// https://docs.substrate.io/main-docs/build/runtime-storage/
	#[pallet::storage]
	//pub(super) type Matches<T: Config> = StorageMap<_, Blake2_128Concat, T::Hash, (T::AccountId, T::BlockNumber)>;
	#[pallet::getter(fn matches_by_id)]
	pub(super) type Matches <T> =
		StorageMap<_, Blake2_128Concat, u32, SingleMatch, ValueQuery>;
	// Learn more about declaring storage items:
	// https://docs.substrate.io/main-docs/build/runtime-storage/#declaring-storage-items
	// pub type Something<T> = StorageValue<_, u32>;

	// Pallets use events to inform users when important changes are made.
	// https://docs.substrate.io/main-docs/build/events-errors/
	#[pallet::event]
	#[pallet::generate_deposit(pub(super) fn deposit_event)]
	pub enum Event<T: Config> {
		/// Event emitted when a claim has been created.
		ClaimCreated { who: T::AccountId, claim: T::Hash },
		/// Event emitted when a claim is revoked by the owner.
		ClaimRevoked { who: T::AccountId, claim: T::Hash },
	}

	// Errors inform users that something went wrong.
	#[pallet::error]
	pub enum Error<T> {
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
		#[pallet::weight(0)]
		pub fn create_match(
			origin: OriginFor<T>,
			id_match: u32,
			status: MatchStatus,
			home_score: u32,
			away_score: u32,
			hash: T::Hash
		) -> DispatchResult {
			// Check that the extrinsic was signed and get the signer.
			// This function will return an error if the extrinsic is not signed.
			let sender = ensure_signed(origin)?;

			// Verify that the specified claim has not already been stored.
			//ensure!(!Claims::<T>::contains_key(&claim), Error::<T>::AlreadyClaimed);

			// Get the block number from the FRAME System pallet.
			//let current_block = <frame_system::Pallet<T>>::block_number();
			let single_match = SingleMatch {
				id_match,
				status,
				home_score,
				away_score,
			};
			// Store the claim with the sender and block number.
			<Matches<T>>::insert(id_match, single_match);

			// Emit an event that the claim was created.
			Self::deposit_event(Event::ClaimCreated { who: sender, claim: hash});

			Ok(())
		}

		// #[pallet::weight(0)]
		// pub fn revoke_claim(origin: OriginFor<T>, claim: T::Hash) -> DispatchResult {
		// 	// Check that the extrinsic was signed and get the signer.
		// 	// This function will return an error if the extrinsic is not signed.
		// 	let sender = ensure_signed(origin)?;

		// 	// Get owner of the claim, if none return an error.
		// 	let (owner, _) = Claims::<T>::get(&claim).ok_or(Error::<T>::NoSuchClaim)?;

		// 	// Verify that sender of the current call is the claim owner.
		// 	ensure!(sender == owner, Error::<T>::NotClaimOwner);

		// 	// Remove claim from storage.
		// 	Claims::<T>::remove(&claim);

		// 	// Emit an event that the claim was erased.
		// 	Self::deposit_event(Event::ClaimRevoked { who: sender, claim });
		// 	Ok(())
		// }
	}
}
