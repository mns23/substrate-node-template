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
fn end_to_end_three_bets_works() {
	new_test_ext().execute_with(|| {
		let id_match: u32 = 1;
		assert_eq!(Balances::total_issuance(), 500);
		assert_ok!(Bets::create_match(Origin::signed(1), 1, (2,00), (2,00), (2,00), (2,00), (2,00)));
		assert_eq!(Matches::<Test>::contains_key(id_match), true);
		assert_ok!(Bets::place_bet(Origin::signed(2), id_match, Prediction::Homewin, 20));
		assert_eq!(Bets::bets_count(), 1);
		assert_ok!(Bets::place_bet(Origin::signed(3), id_match, Prediction::Draw, 20));
		assert_eq!(Bets::bets_count(), 2);
		assert_noop!(Bets::place_bet(Origin::signed(4), id_match, Prediction::Draw, 30), Error::<Test>::MatchAccountInsufficientBalance);
		assert_eq!(Bets::bets_count(), 2);
		assert_ok!(Bets::place_bet(Origin::signed(4), id_match, Prediction::Awaywin, 10));
		assert_eq!(Bets::bets_count(), 3);
		//assert_eq!(Balances::free_balance(1), 0);
		assert_ok!(Bets::set_match_result(Origin::signed(5), id_match));
		assert_eq!(Balances::free_balance(1) > 100, true);
		assert_eq!(Balances::total_issuance(), 500);
	});
}

#[test]
fn create_match_works() {
	new_test_ext().execute_with(|| {
		assert_noop!(Bets::create_match(Origin::signed(1), 1, (2,101), (2,00), (2,00), (2,00), (2,00)), Error::<Test>::OddFracPartOutOfBound);
		assert_noop!(Bets::create_match(Origin::signed(1), 1, (2,00), (0,70), (2,00), (2,00), (2,00)), Error::<Test>::OddIntPartOutOfBound);
		assert_ok!(Bets::create_match(Origin::signed(1), 1, (2,00), (2,00), (2,00), (2,00), (2,00)));
		assert_eq!(Matches::<Test>::contains_key(1), true);
		assert_eq!(Matches::<Test>::contains_key(3), false);
		assert_ok!(Bets::create_match(Origin::signed(2), 2, (2,00), (2,00), (2,00), (2,00), (2,00)));
		assert_eq!(Matches::<Test>::contains_key(2), true);
		assert_noop!(Bets::create_match(Origin::signed(3), 2, (2,00), (2,00), (2,00), (2,00), (2,00)), Error::<Test>::IdMatchAlreadyExists);
	});
}
#[test]
fn place_bet_works() {
	new_test_ext().execute_with(|| {
		assert_ok!(Bets::create_match(Origin::signed(1), 1, (2,00), (2,00), (2,00), (2,00), (2,00)));
		assert_eq!(Matches::<Test>::contains_key(1), true);
		assert_ok!(Bets::place_bet(Origin::signed(2), 1, Prediction::Homewin, 20));
		assert_ok!(Bets::place_bet(Origin::signed(3), 1, Prediction::Draw, 20));
		assert_noop!(Bets::place_bet(Origin::signed(4), 1, Prediction::Draw, 30), Error::<Test>::MatchAccountInsufficientBalance);
		assert_noop!(Bets::place_bet(Origin::signed(4), 2, Prediction::Awaywin, 10), Error::<Test>::MatchNotExists);
		assert_noop!(Bets::place_bet(Origin::signed(1), 1, Prediction::Awaywin, 10), Error::<Test>::SameMatchOwner);
		assert_ok!(Bets::place_bet(Origin::signed(4), 1, Prediction::Awaywin, 10));
		assert_ok!(Bets::set_match_result(Origin::signed(5), 1));
		assert_noop!(Bets::place_bet(Origin::signed(5), 1, Prediction::Awaywin, 10), Error::<Test>::MatchClosed);
		assert_eq!(Bets::bets_count(), 3);
	});
}

#[test]
fn set_match_result_works() {
	new_test_ext().execute_with(|| {
		assert_ok!(Bets::create_match(Origin::signed(1), 1, (2,00), (2,00), (2,00), (2,00), (2,00)));
		assert_eq!(Matches::<Test>::contains_key(1), true);
		assert_ok!(Bets::place_bet(Origin::signed(2), 1, Prediction::Homewin, 20));
		assert_ok!(Bets::place_bet(Origin::signed(3), 1, Prediction::Draw, 20));
		assert_ok!(Bets::place_bet(Origin::signed(4), 1, Prediction::Awaywin, 10));
		assert_eq!(Bets::bets_count(), 3);
		assert_noop!(Bets::set_match_result(Origin::signed(5), 2), Error::<Test>::MatchNotExists);
		assert_ok!(Bets::set_match_result(Origin::signed(5), 1));
		assert_eq!(Balances::free_balance(1) > 100, true);
		assert_noop!(Bets::set_match_result(Origin::signed(5), 1), Error::<Test>::MatchClosed);
	});
}