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
use frame_system::{
	offchain::{
		AppCrypto, CreateSignedTransaction, SendSignedTransaction,
		SignedPayload, Signer, SubmitTransaction,
	},
	self as system,
};
pub use pallet::*;
use lite_json::json::JsonValue;
use sp_core::crypto::KeyTypeId;
use scale_info::prelude::format;
use frame_support::sp_runtime::{
	offchain::{
		http,
		//storage::{MutateStorageError, StorageRetrievalError, StorageValueRef},
		Duration,
	},
	traits::{Saturating, Zero},
	//transaction_validity::{InvalidTransaction, TransactionValidity, ValidTransaction},
	Percent,
};
use sp_std::prelude::*;
//pub use weights::WeightInfo;

/// Id of EventCategory es: Football League X, Football League Y, ESport Game X, Esport Game Y. 
pub type MatchCategoryId = u16;
/// Id which, given the category, specifies an event belonging to it
pub type MatchIntraCategoryId = u64;
/// An index of a Match
pub type MatchId = (MatchCategoryId, MatchIntraCategoryId);
pub type OddsId<T> = (MatchId, AccountIdOf<T>);
/// An index of a Bet
pub type BetIndex = u64;
type AccountIdOf<T> = <T as frame_system::Config>::AccountId;
type BalanceOf<T> = <<T as Config>::Currency as Currency<AccountIdOf<T>>>::Balance;
type BetOf<T> = Bet<AccountIdOf<T>, BalanceOf<T>, OddsId<T>>;
/// Odd touple composed by integer e fractional part through Percent
type Odd = (u32, u8);

#[derive(
	Encode, Decode, Default, Clone, RuntimeDebug, MaxEncodedLen, TypeInfo, PartialEq, Copy,
)]

/// A Match have an initial state (Open), and 2 final states (Closed, Postponed).
pub enum MatchStatus {
	#[default]
	Locked,
	Open,
	Closed,
	Postponed,
}

#[derive(
	Encode, Decode, Default, RuntimeDebug, MaxEncodedLen, TypeInfo, PartialEq, Clone, Copy,
)]
pub struct Match {
	/// The status of the match : open, closed or postponed.
	pub status: MatchStatus,
	pub home_score: u32,
	pub away_score: u32,
	pub timestamp_start: u64,
}

#[derive(
	Encode, Decode, Default, RuntimeDebug, MaxEncodedLen, TypeInfo, PartialEq, Clone, Copy,
)]
pub struct Odds {
	pub homewin: Odd,
	pub awaywin: Odd,
	pub draw: Odd,
	pub under: Odd,
	pub over: Odd,
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
	Open,
	Lost,
	Won,
}

#[derive(
	Encode, Decode, Default, RuntimeDebug, MaxEncodedLen, TypeInfo, PartialEq,
)]
pub struct Bet<AccountId, Balance, OddsId> {
	/// The owner of the bet, bettor account.
	pub owner: AccountId,
	/// Reference to the match odds on which bet on.
	pub id_odds: OddsId,
	/// Result prediction.
	pub prediction: Prediction,
	/// Save Odd value at the moment of Bet (Odds could changhe).
	pub odd: Odd,
	/// The amount wagered.
	pub amount: Balance,
	/// The status of the bet
	pub status: BetStatus,
}

// Offchain worker
pub const KEY_TYPE: KeyTypeId = KeyTypeId(*b"btc!");
pub mod crypto {
	use super::KEY_TYPE;
	use sp_core::sr25519::Signature as Sr25519Signature;
	use frame_support::sp_runtime::{
		app_crypto::{app_crypto, sr25519},
		traits::Verify,
		MultiSignature, MultiSigner,
	};
	app_crypto!(sr25519, KEY_TYPE);

	pub struct TestAuthId;

	impl frame_system::offchain::AppCrypto<MultiSigner, MultiSignature> for TestAuthId {
		type RuntimeAppPublic = Public;
		type GenericSignature = sp_core::sr25519::Signature;
		type GenericPublic = sp_core::sr25519::Public;
	}

	// implemented for mock runtime in test
	impl frame_system::offchain::AppCrypto<<Sr25519Signature as Verify>::Signer, Sr25519Signature>
		for TestAuthId
	{
		type RuntimeAppPublic = Public;
		type GenericSignature = sp_core::sr25519::Signature;
		type GenericPublic = sp_core::sr25519::Public;
	}
}

#[frame_support::pallet]
pub mod pallet {
	use super::*;
	use frame_system::pallet_prelude::*;

	#[pallet::pallet]
	#[pallet::generate_store(pub(super) trait Store)]
	pub struct Pallet<T>(_);

	#[pallet::config]
	pub trait Config: CreateSignedTransaction<Call<Self>> + pallet_timestamp::Config + frame_system::Config {
		/// The bets pallet id.
		#[pallet::constant]
		type PalletId: Get<PalletId>;
		/// Event type.
		type Event: From<Event<Self>> + IsType<<Self as frame_system::Config>::Event>;
		/// The currency mechanism.
		type Currency: ReservableCurrency<Self::AccountId>;
		/// Something that provides randomness in the runtime.
		type Randomness: Randomness<Self::Hash, Self::BlockNumber>;
		/// The identifier type for an offchain worker.
		type AuthorityId: AppCrypto<Self::Public, Self::Signature>;
	}

	/// Mapping matches using match_index as key.
	#[pallet::storage]
	#[pallet::getter(fn matches)]
	pub(super) type Matches<T> =
		StorageMap<_, Blake2_128Concat, MatchId, Match, OptionQuery>;

	/// Mapping matches using match_index as key.
	#[pallet::storage]
	#[pallet::getter(fn odds)]
	pub(super) type Odds<T> =
		StorageMap<_, Blake2_128Concat, OddsId<T>, super::Odds, OptionQuery>;

	/// Mapping bets using bet_index as key.
	#[pallet::storage]
	#[pallet::getter(fn bets)]
	pub(super) type Bets<T: Config> =
		StorageMap<_, Blake2_128Concat, BetIndex, BetOf<T>, OptionQuery>;

	/// A Storage Double Map of bets. Referenced by the id match to which it refers
	/// (to quickly find all the bets related to a specific match), and its index.
	// #[pallet::storage]
	// #[pallet::getter(fn bets)]
	// pub(super) type Bets<T: Config> =
	// 	StorageDoubleMap<_, Blake2_128Concat, u32, Blake2_128Concat, BetIndex, BetInfoOf<T>, OptionQuery>;

	/// Auto-incrementing bet counter
	#[pallet::storage]
	#[pallet::getter(fn bets_count)]
	pub(super) type BetCount<T: Config> = StorageValue<_, BetIndex, ValueQuery>;

	#[pallet::event]
	#[pallet::generate_deposit(pub(super) fn deposit_event)]
	pub enum Event<T: Config> {
		/// A Match was created.
		MatchCreated(MatchId),
		/// A Match was opened.
		MatchOpened(MatchId),
		/// Some Odds was created.
		OddsCreated(OddsId<T>),
		/// A Bet was placed.
		BetPlaced(BetIndex),
		/// A Match was closed.
		MatchClosed(MatchId),
		/// A Match was closed.
		BetSettled(BetIndex),
	}

	#[pallet::error]
	pub enum Error<T> {
		/// A specific match does not exist, cannot place a bet or set match result.
		MatchNotExists,
		/// A specific match is not locked, cannot open it to odds.
		MatchNotLocked,
		/// A specific odds does not exist, cannot place a bet.
		OddsNotExist,
		/// Bet owner and match owner must be different.
		SameMatchOwner,
		/// Fractional part of the Odd out of bound, must be into <0...99> range.
		OddFracPartOutOfBound,
		/// Integer part of the Odd out of bound, must be 1 <= odd.0 <= 4_294_967_295u32.
		OddIntPartOutOfBound,
		/// Match not open for bets or updates, functions available only on open matches.
		MatchClosed,
		/// Match open during a bet claim.
		MatchOpen,
		/// Insufficient free-balance to offer a bet.
		OddsAccountInsufficientBalance,
		/// Insufficient free-balance to place a bet.
		BetAccountInsufficientBalance,
		/// A specific bet does not exists.
		BetNotExists,
		/// Match not open for bets or updates, functions available only on open matches.
		BetSettled,
		/// Payoff procedure failed.
		PayoffError,
	}

	#[pallet::hooks]
	impl<T: Config> Hooks<BlockNumberFor<T>> for Pallet<T> {
		/// Offchain Worker entry point.
		fn offchain_worker(block_number: T::BlockNumber) {
			
			log::info!("Hello World from offchain workers!");
			let parent_hash = <system::Pallet<T>>::block_hash(block_number - 1u32.into());
			log::debug!("Current block: {:?} (parent hash: {:?})", block_number, parent_hash);
			for matches in <Matches<T>>::iter() {
				log::info!("Current match status: {:?}", matches.1.status);
				if matches.1.status == MatchStatus::Locked {
					log::info!("Match {:?} Locked", matches.0);
					let res = Self::fetch_timestamp_and_send_signed(matches.0);
					if let Err(e) = res {
						log::error!("Error: {}", e);
					}
				}
			}

		}
	}


	#[pallet::call]
	impl<T: Config> Pallet<T> {
		/// Passing as arguments the ID of the external match, and the odds,
		/// it creates a match on which to act as a bookmaker and let other users bet on this.
		#[pallet::weight(10_000)]
		pub fn set_odds(
			origin: OriginFor<T>,
			id_match: MatchId,
			odds: super::Odds,
		) -> DispatchResult {
			// Check that the extrinsic was signed and get the signer.
			let odds_owner = ensure_signed(origin)?;
			// Check fractional part of Odd into <0...99> range. Due to the usage of Percent type we have to accept only values between 00 and 99.
			ensure!(odds.homewin.1 < 99 && odds.awaywin.1 < 99 && odds.draw.1 < 99 && odds.under.1 < 99 && odds.over.1 < 99, Error::<T>::OddFracPartOutOfBound);
			// Check integer part of Odd >= 1.
			ensure!(odds.homewin.0 > 0 && odds.awaywin.0 > 0 && odds.draw.0 > 0 && odds.under.0 > 0 && odds.over.0 > 0, Error::<T>::OddIntPartOutOfBound);

			// If the match is not in storage, add it.
			if !<Matches<T>>::contains_key(id_match) {
				let match_to_create = Match {
					status: MatchStatus::Locked,
					home_score: 0, 
					away_score: 0,
					timestamp_start: 0,
				};
				// Store the match with id_match as key.
				<Matches<T>>::insert(id_match, match_to_create);
				Self::deposit_event(Event::MatchCreated(id_match));
			}

			//ensure!(!<Odds<T>>::contains_key((id_match, odds_owner.clone())), Error::<T>::OddIntPartOutOfBound); comment it if accept update of odds
			<Odds<T>>::insert((id_match, odds_owner.clone()), odds);

			Self::deposit_event(Event::OddsCreated((id_match, odds_owner)));
			Ok(())
		}

		/// Saves the match result into storage. At the moment the results are generated randomly,
		/// in future developments it can be called by the oracle.
		#[pallet::weight(10_000)]
		pub fn set_match_start(
			origin: OriginFor<T>,
			id_match: MatchId,
			timestamp_start: u64,
		) -> DispatchResult {
			let _who = ensure_signed(origin)?;
			let mut selected_match = Self::matches(id_match).ok_or(Error::<T>::MatchNotExists)?;
			// Check if match is locked.
			ensure!(selected_match.status == MatchStatus::Locked, Error::<T>::MatchNotLocked);
			// Update match status and results.
			// todo: randomize also MatchStatus.
			selected_match.status = MatchStatus::Open;
			selected_match.timestamp_start = timestamp_start;
			<Matches<T>>::insert(id_match, selected_match);
			// todo: maybe can try also this way: <Matches<T>>::try_mutate, instead of insert.
			
			Self::deposit_event(Event::MatchOpened(id_match));
			Ok(().into())
		}

		/// Allows a user to bet on an open match. To do this, the user need to select the ID of the match
		/// on which bet on, the predicted result and the amount wagered. Once the transaction and the bet have been submitted,
		/// an amount equal to the bet one will be reserved in the bettor's account, an amount equal to the bet one multiplied
		/// by the established odds will be reserved in the bookmaker's account.
		#[pallet::weight(10_000)]
		pub fn place_bet(
			origin: OriginFor<T>,
			id_match: MatchId,
			odds_owner: AccountIdOf<T>,
			prediction: Prediction,
			amount: BalanceOf<T>,
		) -> DispatchResult {
			// Check that the extrinsic was signed and get the signer.
			let bet_owner = ensure_signed(origin)?;
			let bet_index = BetCount::<T>::get();
			// Retrieve the match struct and match_owner
			let odds = Self::odds((id_match, odds_owner.clone())).ok_or(Error::<T>::OddsNotExist)?;
			let match_to_bet_on = Self::matches(id_match).ok_or(Error::<T>::MatchNotExists)?;
			// Ensure bet owner and match owner are not the same account.
			ensure!(bet_owner != odds_owner.clone(), Error::<T>::SameMatchOwner);
			// Ensure match is open.
			ensure!(match_to_bet_on.status == MatchStatus::Open, Error::<T>::MatchClosed);
			// Ensure that bettor account have suffient free balance.
			ensure!(T::Currency::can_reserve(&bet_owner, amount), Error::<T>::BetAccountInsufficientBalance);

			let odd: Odd = match prediction {
				Prediction::Homewin => odds.homewin,
				Prediction::Awaywin => odds.awaywin,
				Prediction::Draw => odds.draw,
				Prediction::Over => odds.over,
				Prediction::Under => odds.under,
			};

			let winnable_amount = (Percent::from_percent(odd.1) * amount).saturating_add(amount.saturating_mul((odd.0 as u32).into()));
			// Ensure that bookie account have suffient free balance.
			ensure!(T::Currency::can_reserve(&odds_owner, winnable_amount), Error::<T>::OddsAccountInsufficientBalance);
			T::Currency::reserve(&bet_owner, amount)?;
			T::Currency::reserve(&odds_owner, winnable_amount)?;

			let bet = Bet {
				owner: bet_owner,
				id_odds: (id_match, odds_owner),
				prediction,
				odd,
				amount,
				status: BetStatus::Open,
			};
			
			// Insert bet into storage.
			<Bets<T>>::insert(bet_index, bet);
			// Not protected against overflow.
			BetCount::<T>::put(bet_index + 1);

			// Emit Event
			Self::deposit_event(Event::BetPlaced(bet_index));
			Ok(().into())
		}

		/// Saves the match result into storage. At the moment the results are generated randomly,
		/// in future developments it can be called by the oracle.
		#[pallet::weight(10_000)]
		pub fn set_match_result(
			origin: OriginFor<T>,
			id_match: MatchId,
			home_score: u32,
			away_score: u32,
		) -> DispatchResult {
			let _who = ensure_signed(origin)?;
			let mut selected_match = Self::matches(id_match).ok_or(Error::<T>::MatchNotExists)?;
			// Check if match is open.
			ensure!(selected_match.status == MatchStatus::Open, Error::<T>::MatchClosed);
			// Update match status and results.
			// todo: randomize also MatchStatus.
			selected_match.status = MatchStatus::Closed;
			selected_match.home_score = home_score;
			selected_match.away_score = away_score;
			<Matches<T>>::insert(id_match, selected_match);
			// todo: maybe can try also this way: <Matches<T>>::try_mutate, instead of insert.
			
			Self::deposit_event(Event::MatchClosed(id_match));
			Ok(().into())
		}

		/// Settles a bet, unlocking all funds towards the winner.
		#[pallet::weight(10_000)]
		pub fn settle_bet(
			origin: OriginFor<T>,
			id_bet: BetIndex,
		) -> DispatchResult {
			let _who = ensure_signed(origin)?;
			let mut bet = Self::bets(id_bet).ok_or(Error::<T>::BetNotExists)?;
			// Check if bet is open.
			ensure!(bet.status == BetStatus::Open, Error::<T>::BetSettled);
			let selected_match = Self::matches(bet.id_odds.0).ok_or(Error::<T>::MatchNotExists)?;
			// Check if match is open.
			ensure!(selected_match.status == MatchStatus::Closed || selected_match.status == MatchStatus::Postponed, Error::<T>::MatchOpen);
			let bet_status: BetStatus = match bet.prediction {
				Prediction::Homewin if selected_match.home_score > selected_match.away_score => BetStatus::Won,
				Prediction::Awaywin if selected_match.home_score < selected_match.away_score => BetStatus::Won,
				Prediction::Draw if selected_match.home_score == selected_match.away_score => BetStatus::Won,
				Prediction::Over if selected_match.home_score + selected_match.away_score > 3 => BetStatus::Won,
				Prediction::Under if selected_match.home_score + selected_match.away_score < 3 => BetStatus::Won,
				_ => BetStatus::Lost,
			};
			let winnable_amount = (Percent::from_percent(bet.odd.1) * bet.amount).saturating_add(bet.amount.saturating_mul((bet.odd.0 as u32).into()));
			// Pay off the bet.
			let odds_owner = &(bet.id_odds.1);
			if bet_status == BetStatus::Won {
				T::Currency::repatriate_reserved(odds_owner, &(bet.owner), winnable_amount, BalanceStatus::Free)?;
				T::Currency::repatriate_reserved(&(bet.owner), odds_owner, bet.amount, BalanceStatus::Free)?;
			} else {
				T::Currency::repatriate_reserved(&(bet.owner), odds_owner, bet.amount.clone(), BalanceStatus::Free)?;
				T::Currency::unreserve(odds_owner, winnable_amount);
			}
			
			// Change bet status and save.
			bet.status = bet_status;
			<Bets<T>>::insert(id_bet, bet);
			
			Self::deposit_event(Event::BetSettled(id_bet));
			Ok(().into())
		}

		/// Saves the match result into storage. At the moment the results are generated randomly,
		/// in future developments it can be called by the oracle.
		#[pallet::weight(10_000)]
		pub fn set_random_match_result(
			origin: OriginFor<T>,
			id_match: MatchId,
		) -> DispatchResult {
			let _who = ensure_signed(origin)?;
			let mut selected_match = Self::matches(id_match).ok_or(Error::<T>::MatchNotExists)?;
			// Check if match is open.
			ensure!(selected_match.status == MatchStatus::Open, Error::<T>::MatchClosed);
			// Update match status and results.
			// todo: randomize also MatchStatus.
			selected_match.status = MatchStatus::Closed;
			selected_match.home_score = Self::generate_random_score(0);
			selected_match.away_score = Self::generate_random_score(1);
			<Matches<T>>::insert(id_match, selected_match);
			// todo: maybe can try also this way: <Matches<T>>::try_mutate, instead of insert.
			
			Self::deposit_event(Event::MatchClosed(id_match));
			Ok(().into())
		}
	}
}

impl<T: Config> Pallet<T> {
	/// A helper function to fetch the price and send signed transaction.
	fn fetch_timestamp_and_send_signed(id_match : MatchId) -> Result<(), &'static str> {
		let signer = Signer::<T, T::AuthorityId>::all_accounts();
		if !signer.can_sign() {
			return Err(
				"No local accounts available. Consider adding one via `author_insertKey` RPC.",
			)
		}
		// Make an external HTTP request to fetch the current price.
		// Note this call will block until response is received.
		let timestamp = Self::fetch_timestamp().map_err(|_| "Failed to fetch price")?;

		// Using `send_signed_transaction` associated type we create and submit a transaction
		// representing the call, we've just created.
		// Submit signed will return a vector of results for all accounts that were found in the
		// local keystore with expected `KEY_TYPE`.
		let results = signer.send_signed_transaction(|_account| {
			// Received price is wrapped into a call to `submit_price` public function of this
			// pallet. This means that the transaction, when executed, will simply call that
			// function passing `price` as an argument.
			Call::set_match_start { id_match, timestamp_start: timestamp }
		});

		for (acc, res) in &results {
			match res {
				Ok(()) => log::info!("[{:?}] Submitted price of {} cents", acc.id, timestamp),
				Err(e) => log::error!("[{:?}] Failed to submit transaction: {:?}", acc.id, e),
			}
		}

		Ok(())
	}

	/// Fetch current price and return the result in cents.
	fn fetch_timestamp() -> Result<u64, http::Error> {
		let deadline = sp_io::offchain::timestamp().add(Duration::from_millis(2_000));
		let randomness_timerange = 600000u64; // milliseconds per 10 minutes
		let timestamp: <T as pallet_timestamp::Config>::Moment = <pallet_timestamp::Pallet<T>>::get();
		let mut min_rand_timestamp = 0u64; // initialize
		if let Ok(_rand_timestamp) = Self::convert_moment_to_u64_in_milliseconds(timestamp) {
			min_rand_timestamp = _rand_timestamp;
		}
		let mut max_rand_timestamp = 0u64;
		if let Some(_rand_timestamp_max) = min_rand_timestamp.checked_add(randomness_timerange) {
			max_rand_timestamp = _rand_timestamp_max;
		}
		log::info!("Timestamp min: {}\nTimestamp max {}", min_rand_timestamp, max_rand_timestamp);
		let url = format!("http://www.randomnumberapi.com/api/v1.0/random?min={}&max={}&count=1", min_rand_timestamp, max_rand_timestamp);
		
		let request =
			http::Request::get(&url);
			//http::Request::get("https://min-api.cryptocompare.com/data/price?fsym=BTC&tsyms=USD");
		let pending = request.deadline(deadline).send().map_err(|_| http::Error::IoError)?;
		let response = pending.try_wait(deadline).map_err(|_| http::Error::DeadlineReached)??;
		if response.code != 200 {
			log::warn!("Unexpected status code: {}", response.code);
			return Err(http::Error::Unknown)
		}
		let body = response.body().collect::<Vec<u8>>();
		let body_str = sp_std::str::from_utf8(&body).map_err(|_| {
			log::warn!("No UTF8 body");
			http::Error::Unknown
		})?;

		let timestamp: u64 = body_str.replace("[", "").replace("]", "").parse().unwrap();
		// let price = match Self::parse_price(body_str) {
		// 	Some(price) => Ok(price),
		// 	None => {
		// 		log::warn!("Unable to extract price from the response: {:?}", body_str);
		// 		Err(http::Error::Unknown)
		// 	},
		// }?;

		log::warn!("Got timestamp: {}", timestamp);

		Ok(timestamp)
	}

	/// Parse the price from the given JSON string using `lite-json`.
	///
	/// Returns `None` when parsing failed or `Some(price in cents)` when parsing is successful.
	fn parse_price(price_str: &str) -> Option<u32> {
		let val = lite_json::parse_json(price_str);
		let price = match val.ok()? {
			JsonValue::Object(obj) => {
				let (_, v) = obj.into_iter().find(|(k, _)| k.iter().copied().eq("USD".chars()))?;
				match v {
					JsonValue::Number(number) => number,
					_ => return None,
				}
			},
			_ => return None,
		};

		let exp = price.fraction_length.saturating_sub(2);
		Some(price.integer as u32 * 100 + (price.fraction / 10_u64.pow(exp)) as u32)
	}

	fn convert_moment_to_u64_in_milliseconds(date: T::Moment) -> Result<u64, DispatchError> {
        let date_as_u64_millis;
        if let Some(_date_as_u64) = TryInto::<u64>::try_into(date).ok() {
            date_as_u64_millis = _date_as_u64;
        } else {
            return Err(DispatchError::Other("Unable to convert Moment to i64 for date"));
        }
        return Ok(date_as_u64_millis);
    }

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
