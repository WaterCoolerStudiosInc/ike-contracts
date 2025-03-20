import { ContractPromise } from "@polkadot/api-contract";
import {
  deployContract,
  contractTx,
  decodeOutput,
  contractQuery,
  DeployedContract,
} from "@scio-labs/use-inkathon";
import * as dotenv from "dotenv";
import { copyArtifacts } from './utils/copyArtifacts.js'
import { getDeploymentData} from "./utils/getDeploymentData.js";
import {uploadCode } from "./utils/uploadCode.js"
import { initPolkadotJs } from "./utils/initPolkadotJs.js";
import { writeContractAddresses } from "./utils/writeContractAddresses.js";
import { ApiPromise } from "@polkadot/api";
import { IKeyringPair } from "@polkadot/types/types";

// Dynamic environment variables
const chainId = process.env.CHAIN || "development";
dotenv.config({
  path: `.env.${chainId}`,
});

async function transfer_gov_tokens(
  api: ApiPromise,
  gov_token_instance: ContractPromise,
  from: IKeyringPair,
  to: string,
  amount: bigint,
): Promise<bigint> {
  await contractTx(
    api,
    from,
    gov_token_instance,
    'PSP22::transfer',
    {},
    [to, amount, []],
  )

  const get_balance_result = await contractQuery(
    api,
    '',
    gov_token_instance,
    'PSP22::balance_of',
    {},
    [to]
  )
  
  return decodeOutput(get_balance_result, gov_token_instance, 'PSP22::balance_of').output
}

async function registry_transfer_role(
  api: ApiPromise,
  registry_instance: ContractPromise,
  account: IKeyringPair,
  role: string,
  assignee: string,
) {
  await contractTx(
    api,
    account,
    registry_instance,
    'iRegistry::transfer_role',
    {},
    [role, assignee],
  )
}

async function main() {
  // Initialization
  const initParams = await initPolkadotJs();
  const { api, chain, account } = initParams;

  console.log('===== Code Hash Deployment =====')

  console.log(`\nUploading code hash: 'Governance Token' ...`)
  const gov_token_data = await getDeploymentData('governance_token')
  const gov_token_hash = await uploadCode(api, account, gov_token_data.contract)
  console.log(`Governance Token hash: ${gov_token_hash}`)

  console.log(`\nUploading code hash: 'Governance NFT' ...`)
  const gov_nft_data = await getDeploymentData('governance_nft')
  const gov_nft_hash = await uploadCode(api, account, gov_nft_data.contract)
  console.log(`Governance NFT hash: ${gov_nft_hash}`)

  console.log(`\nUploading code hash: 'Vesting' ...`)
  const vesting_data = await getDeploymentData('vesting')
  const vesting_hash = await uploadCode(api, account, vesting_data.contract)
  console.log(`Vesting hash: ${vesting_hash}`)

  console.log(`\nUploading code hash: 'Governance Staking' ...`)
  const gov_staking_data = await getDeploymentData('governance_staking')
  const gov_staking_hash = await uploadCode(api, account, gov_staking_data.contract)
  console.log(`Governance Staking hash: ${gov_staking_hash}`)

  console.log(`\nUploading code hash: 'Governance Council' ...`)
  const gov_council_data = await getDeploymentData('governance_council')
  const gov_council_hash = await uploadCode(api, account, gov_council_data.contract)
  console.log(`Council hash: ${gov_council_hash}`)

  console.log(`\nUploading code hash: 'Governance' ...`)
  const governance_data = await getDeploymentData('governance')
  const governance_hash = await uploadCode(api, account, governance_data.contract)
  console.log(`Governance hash: ${governance_hash}\n`)


  console.log('===== Contract Deployment =====')

  console.log(`\nDeploying contract: 'Governance Token' ...`)
  const TOTAL_SUPPLY = 100_000_000_000_000_000_000_000_000n;
  const gov_token = await deployContract(
    api,
    account,
    gov_token_data.abi,
    gov_token_data.wasm,
    'new',
    ["IKE Token", "IKE", 18, TOTAL_SUPPLY],
  )
  const gov_token_instance = new ContractPromise(api, gov_token_data.abi, gov_token.address)

  console.log(`\nDeploying contract: 'Vesting' ...`)
  const vesting = await deployContract(
    api,
    account,
    vesting_data.abi,
    vesting_data.wasm,
    'new',
    [gov_token.address],
  )
  const vesting_instance = new ContractPromise(api, vesting_data.abi, vesting.address)

  console.log(`\nDeploying contract: 'Governance' ...`)
  const vault = await getDeploymentData('vault', chainId);
  const registry = await getDeploymentData('registry', chainId);
  if (vault.address === undefined || registry.address === undefined) {
    throw("Vault and/or Registry address(es) not found!")
  }
  console.log(`Using vault(${vault.address}) & registry(${registry.address}) from the artifacts`)

  const EXECUTE_THRESHOLD = 10000;
  const REJECT_THRESHOLD = 10000;
  const ACCEPT_THRESHOLD = 1000000;
  const STAKING_REWARD_POOL = TOTAL_SUPPLY / 10n;
  const INTEREST_RATE = 100000;
  const SIGNERS = [account.address];
  const governance = await deployContract(
    api,
    account,
    governance_data.abi,
    governance_data.wasm,
    'new',
    [
      vault.address,
      registry.address,
      gov_token.address,
      gov_council_hash,
      gov_nft_hash,
      gov_staking_hash,
      EXECUTE_THRESHOLD,
      REJECT_THRESHOLD,
      ACCEPT_THRESHOLD,
      STAKING_REWARD_POOL,
      INTEREST_RATE,
      SIGNERS,
    ],
  )
  const governance_instance = new ContractPromise(api, governance_data.abi, governance.address)


  console.log('\n===== Address Lookup =====')

  console.log('\nFetching "Governance Council" contract ...')
  const gov_council_contract_result = await contractQuery(
    api,
    '',
    governance_instance,
    'iGovernance::get_council',
  )
  const gov_council = {
    address: decodeOutput(gov_council_contract_result, governance_instance, 'iGovernance::get_council').output,
    hash: gov_council_hash,
    block: governance.block,
    blockNumber: governance.blockNumber,
  } as DeployedContract
  const gov_council_instance = new ContractPromise(api, gov_council_data.abi, gov_council.address)
  console.log(`Governance Council address: ${gov_council.address}`)

  console.log('\nFetching "Governance Staking" contract ...')
  const gov_staking_contract_result = await contractQuery(
    api,
    '',
    governance_instance,
    'iGovernance::get_staking',
  )
  const gov_staking = {
    address: decodeOutput(gov_staking_contract_result, governance_instance, 'iGovernance::get_staking').output,
    hash: gov_staking_hash,
    block: governance.block,
    blockNumber: governance.blockNumber,
  } as DeployedContract
  const gov_staking_instance = new ContractPromise(api, gov_staking_data.abi, gov_staking.address)
  console.log(`Governance Staking address: ${gov_staking.address}`)

  console.log('\nFetching "Governance NFT" contract ...')
  const gov_nft_contract_result = await contractQuery(
    api,
    '',
    gov_staking_instance,
    'get_governance_nft',
  )
  const gov_nft = {
    address: decodeOutput(gov_nft_contract_result, gov_staking_instance, 'get_governance_nft').output,
    hash: gov_nft_hash,
    block: governance.block,
    blockNumber: governance.blockNumber,
  } as DeployedContract
  const gov_nft_instance = new ContractPromise(api, gov_nft_data.abi, gov_nft.address)
  console.log(`Governance NFT address: ${gov_nft.address}`)


  console.log(`\n===== Post Deployment Setups =====`)

  // Transfer 10% of gov_token's supply to gov_staking and governance each
  console.log(`\n[Gov Token] Transfer 10% of the supply to gov_staking (${gov_staking.address}) ...`)
  const staking_balance = await transfer_gov_tokens(api, gov_token_instance, account, gov_staking.address, STAKING_REWARD_POOL);
  console.log("Governance staking balance:", staking_balance)

  console.log(`\n[Gov Token] Transfer 10% of the supply to governance (${governance.address}) ...`)
  const governance_balance = await transfer_gov_tokens(api, gov_token_instance, account, governance.address, STAKING_REWARD_POOL);
  console.log("Governance balance:", governance_balance)

  // IVault::transfer_role_adjust_fee
  console.log(`\n[Vault] Transferring 'adjust_role' to the governance (${governance.address}) ...`)
  const vault_instance = new ContractPromise(api, vault.abi, vault.address)
  await contractTx(
    api,
    account,
    vault_instance,
    'iVault::transfer_role_adjust_fee',
    {},
    [governance.address],
  )

  // IRegistry::transfer_role
  const registry_instance = new ContractPromise(api, registry.abi, registry.address)
  console.log(`\n[Registry] Transfer 'AddAgent' role to the staking contract (${gov_staking.address})`)
  await registry_transfer_role(api, registry_instance, account, 'AddAgent', gov_staking.address)

  console.log(`[Registry] Transfer 'UpdateAgents' role to the staking contract (${gov_staking.address})`)
  await registry_transfer_role(api, registry_instance, account, 'UpdateAgents', gov_staking.address)

  console.log(`[Registry] Transfer 'DisableAgent' role to the staking contract (${gov_staking.address})`)
  await registry_transfer_role(api, registry_instance, account, 'DisableAgent', gov_staking.address)

  console.log(`[Registry] Transfer 'RemoveAgent' role to the staking contract (${gov_staking.address})`)
  await registry_transfer_role(api, registry_instance, account, 'RemoveAgent', gov_staking.address)


  console.log('\n===== Contract Locations =====')
  
  console.log({
    gov_token: gov_token.address,
    gov_nft: gov_nft.address,
    vesting: vesting.address,
    gov_staking: gov_staking.address,
    gov_council: gov_council.address,
    governance: governance.address,
  })

  console.log()

  // Write deployment artifacts into associated chainId subdirectory
  await copyArtifacts('governance_token', chainId)
  await copyArtifacts('governance_nft', chainId)
  await copyArtifacts('vesting', chainId)
  await copyArtifacts('governance_staking', chainId)
  await copyArtifacts('governance_council', chainId)
  await copyArtifacts('governance', chainId)

  console.log()

  // Write deployment metadata into associated chainId subdirectory
  await writeContractAddresses(chain.network, {
    "governance_token": gov_token,
    "governance_nft": gov_nft,
    vesting,
    "governance_staking": gov_staking,
    "governance_council": gov_council,
    governance,
  })
}

main()
  .catch((error) => {
    console.error(error);
    process.exit(1);
  })
  .finally(() => process.exit(0));
