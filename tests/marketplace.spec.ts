import '@polkadot/api-augment';
import { assert, expect, use } from 'chai';
import chaiAsPromised from 'chai-as-promised';
import { encodeAddress } from '@polkadot/keyring';
import { FrameSystemAccountInfo } from '@polkadot/types/lookup';
import BN from 'bn.js';
import Market_factory from '../types/constructors/marketplace';
import Market from '../types/contracts/marketplace';
import TestPSP34_factory from '../types/constructors/test_psp34';
import TestPSP34 from '../types/contracts/test_psp34';
import Shiden34_Factory from '../types/constructors/shiden34';
import Shiden34 from '../types/contracts/shiden34';

import { ApiPromise, WsProvider, Keyring } from '@polkadot/api';
import { KeyringPair } from '@polkadot/keyring/types';
// import { AccountId } from '../types/types-arguments/marketplace_contract';
import { ResultBuilder, ReturnNumber } from '@supercolony/typechain-types';
import { Hash } from 'types-arguments/marketplace';

use(chaiAsPromised);

const MAX_SUPPLY = 888;
const BASE_URI = "ipfs://tokenUriPrefix/";
const COLLECTION_METADATA = "ipfs://collectionMetadata/data.json";
const TOKEN_URI_1 = "ipfs://tokenUriPrefix/1.json";
const TOKEN_URI_5 = "ipfs://tokenUriPrefix/5.json";
const ONE = new BN(10).pow(new BN(18));
const PRICE_PER_MINT = ONE;

// Create a new instance of contract
const wsProvider = new WsProvider('ws://127.0.0.1:9944');
// Create a keyring instance
const keyring = new Keyring({ type: 'sr25519' });

describe('Marketplace tests', () => {
  let marketplaceFactory: Market_factory;
  let psp34Factory: TestPSP34_factory;
  let shiden34Factory: Shiden34_Factory;
  let api: ApiPromise;
  let deployer: KeyringPair;
  let bob: KeyringPair;
  let charlie: KeyringPair;
  let marketplace: Market;
  let psp34: TestPSP34;
  let shiden34: Shiden34;

  const gasLimit = 18750000000;
  const ZERO_ADDRESS = encodeAddress(
    '0x0000000000000000000000000000000000000000000000000000000000000000',
  );
  let gasRequired: bigint;

  async function setup(): Promise<void> {
    api = await ApiPromise.create({ provider: wsProvider });
    deployer = keyring.addFromUri('//Alice');
    bob = keyring.addFromUri('//Bob');
    charlie = keyring.addFromUri('//Charlie');
    marketplaceFactory = new Market_factory(api, deployer);
    psp34Factory = new TestPSP34_factory(api, deployer);
    shiden34Factory = new Shiden34_Factory(api, deployer);
    marketplace = new Market((await marketplaceFactory.new(deployer.address)).address, deployer, api);
    psp34 = new TestPSP34((await psp34Factory.new()).address, deployer, api);
    shiden34 = new Shiden34((await shiden34Factory.new(
      'default'.split(''),
      'DFT'.split(''),
      'uri'.split(''),
      1000,
      1
    )).address, deployer, api);
  }

  it('setup and mint works', async () => {
    await setup();
    const { gasRequired } = await psp34.withSigner(bob).query.mint(bob.address, {u64: 1});
    let mintResult = await psp34.withSigner(bob).tx.mint(bob.address, {u64: 1}, {gasLimit: gasRequired * 2n });
    expect((await psp34.query.totalSupply()).value.rawNumber.toNumber()).to.equal(2);
    expect((await psp34.query.balanceOf(bob.address)).value).to.equal(1);
    expect((await psp34.query.ownerOf({ u64: 1 })).value).to.equal(bob.address);
  })

  it('setMarketplaceFee works', async () => {
    await setup();
    let { gasRequired } = await marketplace.query.setMarketplaceFee(120);

    let result = await marketplace.tx.setMarketplaceFee(120, { gasLimit: gasRequired });
    expect((await marketplace.query.getMarketplaceFee()).value).to.equal(120);
  })

  it('register contract works for the Marketplace owner', async () => {
    await setup();
    await registerContract(deployer);

    const contract = await marketplace.query.getRegisteredCollection(psp34.address);

    expect(contract.value.royaltyReceiver).to.be.equal(deployer.address);
    expect(contract.value.royalty).to.be.equal(100);
    expect(contract.value.marketplaceIpfs).to.be.equal(toHex(string2ascii('ipfs')));
  });

  it('register contract fails if fee is too high', async () => {
    await setup();

    const ipfs = string2ascii('ipfs');
    const { gasRequired } = await marketplace.withSigner(deployer).query.register(psp34.address, deployer.address, 10001, ipfs);
    const registerResult = await marketplace.withSigner(deployer).query.register(psp34.address, deployer.address, 10001, ipfs, { gasLimit: gasRequired * 2n });

    expect(registerResult.value.err.hasOwnProperty('feeTooHigh')).to.be.true;
  });

  it('list / unlist works', async () => {
    await setup();
    await mintToken(bob);
    await registerContract(deployer);
    
    // List token to the marketplace.
    await listToken(bob);
    
    // Check if the token is actually listed.
    expect((await marketplace.query.getPrice(psp34.address, {u64: 1})).value).to.equal(100);

    // Unlist token from the marketplace.
    const { gasRequired } = await marketplace.withSigner(bob).query.unlist(psp34.address, {u64: 1});
    const unlistResult = await marketplace.withSigner(bob).tx.unlist(psp34.address, {u64: 1}, { gasLimit: gasRequired * 2n });
    expect(unlistResult.result?.isInBlock).to.be.true;
    
    // Check if the token is actually unlisted.
    expect((await marketplace.query.getPrice(psp34.address, {u64: 1})).value).to.equal(null);
  });

  it('list fails if not a nft owner', async () => {
    await setup();
    await mintToken(bob);
    await registerContract(deployer);
    
    // Try to list token to the marketplace.
    const { gasRequired } = await marketplace.withSigner(charlie).query.list(psp34.address, {u64: 1}, 100);
    const listResult = await marketplace.withSigner(charlie).query.list(psp34.address, {u64: 1}, 100, { gasLimit: gasRequired * 2n });

    expect(listResult.value.err.hasOwnProperty('notOwner')).to.be.true;
  });

  it('list fails if token is already listed', async () => {
    await setup();
    await mintToken(bob);
    await registerContract(deployer);
    
    // List token to the marketplace.
    const { gasRequired } = await marketplace.withSigner(bob).query.list(psp34.address, {u64: 1}, 100);
    await marketplace.withSigner(bob).tx.list(psp34.address, {u64: 1}, 100, { gasLimit: gasRequired * 2n });

    // Try to list the same token again.
    const listResult = await marketplace.withSigner(bob).query.list(psp34.address, {u64: 1}, 100, { gasLimit: gasRequired * 2n });

    expect(listResult.value.err.hasOwnProperty('itemAlreadyListedForSale')).to.be.true;
  });

  it('unlist fails if token is not listed', async () => {
    await setup();
    await mintToken(bob);
    await registerContract(deployer);
    
    // unlist token to the marketplace.
    const { gasRequired } = await marketplace.withSigner(bob).query.unlist(psp34.address, {u64: 1});
    const unlistResult = await marketplace.withSigner(bob).query.unlist(psp34.address, {u64: 1}, { gasLimit: gasRequired * 2n });

    expect(unlistResult.value.err.hasOwnProperty('itemNotListedForSale')).to.be.true;
  });

  it('buy works', async () => {
    await setup();
    await mintToken(charlie);
    await registerContract(deployer);
    await listToken(charlie);

    // Charlie approves marketplace to be operator of the token
    const approveGas = (await psp34.withSigner(charlie).query.approve(marketplace.address, { u64: 1 }, true)).gasRequired;
    let approveResult = await psp34.withSigner(charlie).tx.approve(marketplace.address, { u64: 1 }, true, { gasLimit: approveGas });

    const deployerOriginalBalance = await getBalance(deployer);
    const bobOriginalBalance = await getBalance(bob);
    const charlieOriginalBalance = await getBalance(charlie);

    // Buy token
    const { gasRequired } = await marketplace.withSigner(bob).query.buy(psp34.address, {u64: 1});
    const buyResult = await marketplace.withSigner(bob).tx.buy(
      psp34.address, 
      {u64: 1},
      { gasLimit: gasRequired * 2n, value: new BN('100000000000000000000') });

    expect(buyResult.result?.isInBlock).to.be.true;

    // Balances check.
    const deployerBalance = await getBalance(deployer);
    const bobBalance = await getBalance(bob);
    const charlieBalance = await getBalance(charlie);
    
    // Check the marketplace fee receiver balance. ATM all royalties go to deployer.
    expect(deployerBalance.eq(deployerOriginalBalance.add(new BN('2000000000000000000')))).to.be.true;
    // Check seller's balance. Should be increased by price - fees
    expect(charlieBalance.toString()).to.be.equal(charlieOriginalBalance.add(new BN('98000000000000000000')).toString());
    // Check the token owner.
    expect((await psp34.query.ownerOf({ u64: 1 })).value).to.equal(bob.address);
    // Check if allowance is unset.
    expect((await psp34.query.allowance(charlie.address, marketplace.address, { u64: 1 })).value).to.equal(false);

    // Try to buy the same token again
    const reBuyResult = await marketplace.withSigner(bob).query.buy(
      psp34.address, 
      {u64: 1},
      { gasLimit: gasRequired * 2n, value: new BN('100000000000000000000') });
      expect(reBuyResult.value.err.hasOwnProperty('alreadyOwner')).to.be.true;
  });

  it('setContractMetadata works', async () => {
    await setup();
    await registerContract(deployer);
    const marketplace_ipfs = string2ascii('ipfs://test');

    const gas = (await marketplace.withSigner(deployer).query.setContractMetadata(psp34.address, marketplace_ipfs)).gasRequired;
    const approveResult = await marketplace.withSigner(deployer).tx.setContractMetadata(psp34.address, marketplace_ipfs, { gasLimit: gas });

    const contract = await marketplace.query.getRegisteredCollection(psp34.address);
    expect(contract.value.marketplaceIpfs).to.be.equal(toHex(marketplace_ipfs));
  });

  it('setContractMetadata returns error if no contract', async () => {
    await setup();
    const marketplace_ipfs = 'ipfs://test';

    const gas = (await marketplace.withSigner(deployer).query.setContractMetadata(psp34.address, marketplace_ipfs.split(''))).gasRequired;
    const approveResult = await marketplace.withSigner(deployer).query.setContractMetadata(psp34.address, marketplace_ipfs.split(''), { gasLimit: gas });

    expect(approveResult.value.err.hasOwnProperty('notRegisteredContract')).to.be.true;
  });

  it('setNftContractHash works', async () => {
    await setup();
    await registerContract(deployer);
    const hash = string2ascii('h'.repeat(32));

    const gas = (await marketplace.withSigner(deployer).query.setNftContractHash(hash)).gasRequired;
    await marketplace.withSigner(deployer).tx.setNftContractHash(hash, {gasLimit: gas});

    const hashValue = await marketplace.query.nftContractHash();
    expect(hashValue.value).to.be.equal(toHex(hash));
  });

  it('setNftContractHash fails if not an owner', async () => {
    await setup();
    await registerContract(deployer);
    const hash = string2ascii('h'.repeat(32));

    const gas = (await marketplace.withSigner(bob).query.setNftContractHash(hash)).gasRequired;
    const result = await marketplace.withSigner(bob).query.setNftContractHash(hash, {gasLimit: gas});

    expect(result.value.err.ownableError).to.equal('CallerIsNotOwner');
  });

  it('factory works', async () => {
    await setup();
    const marketplace_ipfs = 'ipfs://test';
    const siden34Hash: Hash = shiden34.abi.info.source.wasmHash.toHex();

    const hashGas = (await marketplace.withSigner(deployer).query.setNftContractHash(siden34Hash)).gasRequired;
    await marketplace.withSigner(deployer).tx.setNftContractHash(siden34Hash, { gasLimit: hashGas });
    
    const gas = (await marketplace.withSigner(deployer).query.factory(
      string2ascii(marketplace_ipfs),
      bob.address,
      200,
      string2ascii('testNft'),
      string2ascii('TST'),
      string2ascii('nftUri'),
      1000,
      100
    )).gasRequired;
    const factoryResult = await marketplace.withSigner(deployer).tx.factory(
      string2ascii(marketplace_ipfs),
      bob.address,
      200,
      string2ascii('testNft'),
      string2ascii('TST'),
      string2ascii('nftUri'),
      1000,
      100,
      {gasLimit: gas});
    
    // Check if Shiden34 contract has been deployed
    const instatiatedEvent = factoryResult.result.events.find(x => x.event.method === 'Instantiated' && x.event.section === 'contracts');
    expect(instatiatedEvent).is.not.empty;
    
    const shiden34Address = instatiatedEvent.event.data['contract'];
    expect(shiden34Address).is.not.empty;

    // Check if deployed contract has been registered
    const registerCheckResult = await marketplace.query.getRegisteredCollection(shiden34Address);
    expect(registerCheckResult).is.not.empty;
    expect(registerCheckResult.value.royalty).to.be.equal(200);
    expect(registerCheckResult.value.royaltyReceiver).to.be.equal(bob.address);
    expect(registerCheckResult.value.marketplaceIpfs).to.be.equal(toHex(string2ascii(marketplace_ipfs)));
  });

  // Helper function to mint a token.
  async function mintToken(signer: KeyringPair): Promise<void> {
    const { gasRequired } = await psp34.withSigner(signer).query.mint(signer.address, {u64: 1});
    const mintResult = await psp34.withSigner(signer).tx.mint(signer.address, {u64: 1}, { gasLimit: gasRequired * 2n });
    expect(mintResult.result?.isInBlock).to.be.true;
    expect((await psp34.query.ownerOf({ u64: 1 })).value).to.equal(signer.address);
  }

  // Helper function to register contract.
  async function registerContract(signer:KeyringPair) {
    const ipfs = string2ascii('ipfs');
    const { gasRequired } = await marketplace.withSigner(signer).query.register(psp34.address, signer.address, 100, ipfs);
    const registerResult = await marketplace.withSigner(signer).tx.register(psp34.address, signer.address, 100, ipfs, { gasLimit: gasRequired * 2n });
    expect(registerResult.result?.isInBlock).to.be.true;
  }

  // Helper function to list token for sale.
  async function listToken(signer:KeyringPair) {
    const { gasRequired } = await marketplace.withSigner(signer).query.list(psp34.address, {u64: 1}, 100);
    const listResult = await marketplace.withSigner(signer).tx.list(psp34.address, {u64: 1}, 100, { gasLimit: gasRequired * 2n });
    expect(listResult.result?.isInBlock).to.be.true;
  }

  // Helper function to get account balance
  async function getBalance(account:KeyringPair) {
    const balances = await api.query.system.account<FrameSystemAccountInfo>(account.address);

    return balances.data.free;
  }
})

// Helper function to parse Events
// eslint-disable-next-line @typescript-eslint/no-explicit-any
function emit(result: { events?: any }, name: string, args: any): void {
  const event = result.events.find(
    (event: { name: string }) => event.name === name,
  );
  for (const key of Object.keys(event.args)) {
    if (event.args[key] instanceof ReturnNumber) {
      event.args[key] = event.args[key].toNumber();
    }
  }
  expect(event).eql({ name, args, });
}

// Helper function to get ASCII array from string.
function string2ascii(inputString: string): number[] {
  let result: number[] = [];
  for (var i = 0; i < inputString.length; i ++) {
    result.push(inputString[i].charCodeAt(0));
  }

  return result;
}

// Helper function to get hex string from ASCII array.
function toHex(ascii: number[]): string {
  return '0x' + Buffer.from(ascii).toString('hex');
}