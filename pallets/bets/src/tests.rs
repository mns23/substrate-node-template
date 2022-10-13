//! Tests for the module.

use super::*;
use frame_support::{
	assert_noop,
	assert_ok,
};
use mock::{
	new_test_ext, Balances, Bets, Origin,
	Test,
};
// use pallet_balances::Error as BalancesError;
// use sp_runtime::traits::BadOrigin;

#[test]
fn initial_state() {
	new_test_ext().execute_with(|| {
		assert_eq!(Balances::free_balance(1), 100);
		assert_eq!(Balances::free_balance(2), 100);
		assert_eq!(Balances::free_balance(3), 100);
		assert_eq!(Balances::free_balance(4), 100);
		assert_eq!(Balances::free_balance(5), 100);
	});
}

#[test]
fn three_bets_process() {
	new_test_ext().execute_with(|| {
		let id_match: u32 = 1;
		assert_eq!(Balances::total_issuance(), 500);
		assert_ok!(Bets::create_match(Origin::signed(1), id_match, 2, 2, 2, 2, 2));
		assert_eq!(Matches::<Test>::contains_key(1), true);
		assert_ok!(Bets::place_bet(Origin::signed(2), id_match, Prediction::Homewin, 20));
		assert_eq!(Bets::bets_count(), 1);
		assert_ok!(Bets::place_bet(Origin::signed(3), id_match, Prediction::Draw, 20));
		assert_eq!(Bets::bets_count(), 2);
		assert_noop!(Bets::place_bet(Origin::signed(4), id_match, Prediction::Draw, 11), Error::<Test>::MatchAccountInsufficientBalance);
		assert_eq!(Bets::bets_count(), 2);
		assert_ok!(Bets::place_bet(Origin::signed(4), id_match, Prediction::Awaywin, 10));
		assert_eq!(Bets::bets_count(), 3);
		assert_eq!(Balances::free_balance(1), 0);
		assert_ok!(Bets::set_match_result(Origin::signed(5), id_match));
		assert_eq!(Balances::free_balance(1) > 100, true);
		assert_eq!(Balances::total_issuance(), 500);
	});
}