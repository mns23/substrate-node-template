//! Test utilities

use super::*;
use crate as pallet_bets;

use frame_support::{
	parameter_types,
	traits::{ConstU32, ConstU64},
};
use frame_support_test::TestRandomness;
//use frame_system::EnsureRoot;
use sp_core::{
	offchain::{self, testing, OffchainWorkerExt, TransactionPoolExt},
	H256, crypto::KeyTypeId, OpaqueMetadata, sr25519::Signature,
};
use sp_keystore::{testing::KeyStore, KeystoreExt, SyncCryptoStore};
use std::sync::Arc;
use sp_runtime::{
	create_runtime_str, generic, impl_opaque_keys,
	testing::{Header,TestXt},
	traits::{BlakeTwo256, IdentityLookup, IdentifyAccount, Verify, Extrinsic as ExtrinsicT},
	Perbill, RuntimeAppPublic, MultiSignature, SaturatedConversion,
};
use sp_consensus_aura::sr25519::AuthorityId as AuraId;
//pub use pallet_timestamp::Call as TimestampCall;

type UncheckedExtrinsic = frame_system::mocking::MockUncheckedExtrinsic<Test>;
type Block = frame_system::mocking::MockBlock<Test>;
/// Alias to 512-bit hash when used in the context of a transaction signature on the chain.
//pub type Signature = MultiSignature;
/// Some way of identifying an account on the chain. We intentionally make it equivalent
/// to the public key of our transaction signing scheme.
pub type AccountId = <<Signature as Verify>::Signer as IdentifyAccount>::AccountId;
/// The SignedExtension to the basic transaction logic.
// pub type SignedExtra = (
// 	frame_system::CheckNonZeroSender<Test>,
// 	frame_system::CheckSpecVersion<Test>,
// 	frame_system::CheckTxVersion<Test>,
// 	frame_system::CheckGenesis<Test>,
// 	frame_system::CheckEra<Test>,
// 	frame_system::CheckNonce<Test>,
// 	frame_system::CheckWeight<Test>,
// 	//pallet_transaction_payment::ChargeTransactionPayment<Test>,
// );
/// Index of a transaction in the chain.
pub type Index = u32;
/// An index to a block.
pub type BlockNumber = u32;

pub use frame_system::Call as SystemCall;
//pub use pallet_balances::Call as BalancesCall;


frame_support::construct_runtime!(
	pub enum Test where
		Block = Block,
		NodeBlock = Block,
		UncheckedExtrinsic = UncheckedExtrinsic,
	{
		System: frame_system::{Pallet, Call, Config, Storage, Event<T>},
		Balances: pallet_balances::{Pallet, Storage, Config<T>, Event<T>},
		Bets: pallet_bets::{Pallet, Call, Storage, Event<T>},
		Aura: pallet_aura::{Pallet, Storage, Config<T>},
		Timestamp: pallet_timestamp::{Pallet, Call, Storage, Inherent},
	}
);

parameter_types! {
	pub const AvailableBlockRatio: Perbill = Perbill::one();
	pub const BlockHashCount: BlockNumber = 2400;
}

impl frame_system::Config for Test {
	type BaseCallFilter = frame_support::traits::Everything;
	type BlockWeights = ();
	type BlockLength = ();
	type DbWeight = ();
	type Origin = Origin;
	type Index = u64;
	type BlockNumber = u64;
	type Hash = H256;
	type Hashing = BlakeTwo256;
	type AccountId = sp_core::sr25519::Public;
	type Lookup = IdentityLookup<Self::AccountId>;
	type Header = Header;
	type BlockHashCount = ConstU64<250>;
	type Version = ();
	type PalletInfo = PalletInfo;
	type AccountData = pallet_balances::AccountData<u64>;
	type OnNewAccount = ();
	type OnKilledAccount = ();
	type SystemWeightInfo = ();
	type SS58Prefix = ();
	type OnSetCode = ();
	type MaxConsumers = ConstU32<16>;
	type Event = Event;
	type Call = Call;
}

// impl<LocalCall> frame_system::offchain::CreateSignedTransaction<LocalCall> for Test
// where
// 	Call: From<LocalCall>,
// {
// 	fn create_transaction<C: frame_system::offchain::AppCrypto<Self::Public, Self::Signature>>(
// 		call: Call,
// 		public: <Signature as sp_runtime::traits::Verify>::Signer,
// 		account: AccountId,
// 		index: Index,
// 	) -> Option<(Call, <UncheckedExtrinsic as sp_runtime::traits::Extrinsic>::SignaturePayload)> {
// 		let period = BlockHashCount::get() as u64;
// 		let current_block = (System::block_number())
// 			.saturated_into::<u64>()
// 			.saturating_sub(1);
// 		let tip = 0;
// 		let extra: SignedExtra = (
// 			frame_system::CheckNonZeroSender::<Test>::new(),
// 			frame_system::CheckSpecVersion::<Test>::new(),
// 			frame_system::CheckTxVersion::<Test>::new(),
// 			frame_system::CheckGenesis::<Test>::new(),
// 			frame_system::CheckEra::<Test>::from(generic::Era::mortal(period, current_block)),
// 			frame_system::CheckNonce::<Test>::from(index),
// 			frame_system::CheckWeight::<Test>::new(),
// 			//pallet_transaction_payment::ChargeTransactionPayment::<Runtime>::from(tip),
// 		);

// 		let raw_payload = SignedPayload::new(call, extra)
// 			.map_err(|e| {
// 				//log::warn!("Unable to create signed payload: {:?}", e);
// 			})
// 			.ok()?;
// 		let signature = raw_payload.using_encoded(|payload| C::sign(payload, public))?;
// 		let address = account;
// 		let (call, extra, _) = raw_payload.deconstruct();
// 		Some((call, (sp_runtime::MultiAddress::Id(address), signature.into(), extra)))
// 	}
// }

// impl frame_system::offchain::SigningTypes for Test {
// 	type Public = <Signature as sp_runtime::traits::Verify>::Signer;
// 	type Signature = Signature;
// }

// impl<LocalCall> frame_system::offchain::SendTransactionTypes<LocalCall> for Test
// where
// 	Call: From<LocalCall>,
// {
// 	type OverarchingCall = Call;
// 	type Extrinsic = TestXt<Call, ()>;
// }

type Extrinsic = TestXt<Call, ()>;

impl frame_system::offchain::SigningTypes for Test {
	type Public = <Signature as Verify>::Signer;
	type Signature = Signature;
}

impl<LocalCall> frame_system::offchain::SendTransactionTypes<LocalCall> for Test
where
	Call: From<LocalCall>,
{
	type OverarchingCall = Call;
	type Extrinsic = Extrinsic;
}

impl<LocalCall> frame_system::offchain::CreateSignedTransaction<LocalCall> for Test
where
	Call: From<LocalCall>,
{
	fn create_transaction<C: frame_system::offchain::AppCrypto<Self::Public, Self::Signature>>(
		call: Call,
		_public: <Signature as Verify>::Signer,
		_account: AccountId,
		nonce: u64,
	) -> Option<(Call, <Extrinsic as ExtrinsicT>::SignaturePayload)> {
		Some((call, (nonce, ())))
	}
}

impl pallet_balances::Config for Test {
	type MaxLocks = ();
	type MaxReserves = ();
	type ReserveIdentifier = [u8; 8];
	type Balance = u64;
	type DustRemoval = ();
	type ExistentialDeposit = ConstU64<1>;
	type AccountStore = System;
	type WeightInfo = ();
	type Event = Event;
}

impl pallet_timestamp::Config for Test {
	type Moment = u64;
	type OnTimestampSet = Aura;
	type MinimumPeriod = ConstU64<1>;
	type WeightInfo = ();
}

impl pallet_aura::Config for Test {
	type AuthorityId = AuraId;
	type DisabledValidators = ();
	type MaxAuthorities = ConstU32<32>;
}

parameter_types! {
	pub const BetsPalletId: PalletId = PalletId(*b"py/bbets");
}

impl Config for Test {
	type PalletId = BetsPalletId;
	type Currency = Balances;
	type Event = Event;
	type Randomness = TestRandomness<Self>;
	type AuthorityId = pallet_bets::crypto::TestAuthId;
}

pub fn acc_pub(acc_num: u8) -> sp_core::sr25519::Public {
	sp_core::sr25519::Public::from_raw([acc_num; 32])
}

pub fn new_test_ext() -> sp_io::TestExternalities {
	let mut t = frame_system::GenesisConfig::default().build_storage::<Test>().unwrap();
	pallet_balances::GenesisConfig::<Test> {
		balances: vec![(acc_pub(1), 100), (acc_pub(2), 100), (acc_pub(3), 100), (acc_pub(4), 100), (acc_pub(5), 100)],
	}
	.assimilate_storage(&mut t)
	.unwrap();

	t.into()
}

pub fn new_test_ext_ocw() -> sp_io::TestExternalities {
	let mut t = frame_system::GenesisConfig::default().build_storage::<Test>().unwrap();
	pallet_balances::GenesisConfig::<Test> {
		balances: vec![(acc_pub(1), 100), (acc_pub(2), 100), (acc_pub(3), 100), (acc_pub(4), 100), (acc_pub(5), 100)],
	}
	.assimilate_storage(&mut t)
	.unwrap();

	//Offchain setup
	const PHRASE: &str = "news slush supreme milk chapter athlete soap sausage put clutch what kitten";
	let (offchain, offchain_state) = testing::TestOffchainExt::new();
	let (pool, pool_state) = testing::TestTransactionPoolExt::new();
	let keystore = KeyStore::new();
	SyncCryptoStore::sr25519_generate_new(
		&keystore,
		crate::crypto::Public::ID,
		Some(&format!("{}/hunter1", PHRASE)),
	)
	.unwrap();
	let mut ext = sp_io::TestExternalities::new(t);
	ext.register_extension(OffchainWorkerExt::new(offchain));
	ext.register_extension(TransactionPoolExt::new(pool));
	ext.register_extension(KeystoreExt(Arc::new(keystore)));
	offchain_state.write().expect_request(testing::PendingRequest {
		method: "GET".into(),
		uri: "http://www.randomnumberapi.com/api/v1.0/random?min=0&max=600000&count=1".into(),
		response: Some(br#"[1667758138]"#.to_vec()),
		sent: true,
		..Default::default()
	});
	ext.execute_with(|| {
		//Timestamp::set_timestamp(Timestamp::now());
		let id_match = (1,23);
		let odds = Odds {
			homewin: (2,00),
			awaywin: (2,00),
			draw: (2,00),
			under: (2,00),
			over: (2,00),
		};
		assert_eq!(Balances::total_issuance(), 500);
		Bets::set_odds(Origin::signed(acc_pub(1)), id_match, odds).unwrap();
		Bets::fetch_timestamp_and_send_signed(id_match).unwrap();
		// let match_created = Bets::matches(id_match).unwrap();
		// assert_eq!(match_created.timestamp_start, 1667758138);
		let tx = pool_state.write().transactions.pop().unwrap();
		assert!(pool_state.read().transactions.is_empty());
		let tx = Extrinsic::decode(&mut &*tx).unwrap();
		assert_eq!(tx.signature.unwrap().0, 0);
		assert_eq!(tx.call, Call::Bets(crate::Call::set_match_start { id_match: (id_match), timestamp_start: (1667758138) }));
		let match_created = Bets::matches(id_match).unwrap();
		assert_eq!(match_created.timestamp_start, 1667758138);
	});

	ext
}
