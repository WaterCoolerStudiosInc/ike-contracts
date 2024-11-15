import { ContractPromise } from '@polkadot/api-contract'
import { deployContract, contractTx, decodeOutput, contractQuery, DeployedContract } from '@scio-labs/use-inkathon'
import * as dotenv from 'dotenv'
import { copyArtifacts } from './utils/copyArtifacts.js'
import { getDeploymentData } from './utils/getDeploymentData.js'
import { initPolkadotJs } from './utils/initPolkadotJs.js'
import { uploadCode } from './utils/uploadCode.js'
import { writeContractAddresses } from './utils/writeContractAddresses.js'

// Dynamic environment variables
const chainId = process.env.CHAIN || 'development'
dotenv.config({
  path: `.env.${chainId}`,
})

/**
 * Deploys and configures contracts
 */
const main = async (validators: string[]) => {
  if (!validators || validators.length === 0) {
    throw new Error(`Must specify validator addresses`)
  }
  if (new Set(validators).size !== validators.length) {
    throw new Error(`Duplicate validator addresses`)
  }

  // Initialization
  const initParams = await initPolkadotJs()
  const { api, chain, account } = initParams

  console.log('===== Network Queries =====')

  const minNominatorBondCodec = await api.query.staking.minNominatorBond()
  const minNominatorBond = BigInt(minNominatorBondCodec.toString())
  console.log(`Minimum nomination bond: ${minNominatorBond}`)

  const sessionPeriod = api.consts.committeeManagement.sessionPeriod.toString().replace(/,/g, '')
  const sessionsPerEra = api.consts.staking.sessionsPerEra.toString().replace(/,/g, '')
  const eraDurationMs = 1000n * BigInt(sessionPeriod) * BigInt(sessionsPerEra)
  console.log(`Era duration: ${eraDurationMs.toLocaleString()} ms`)

  const blockWeights = api.consts.system.blockWeights.toHuman()
  const maxBlock = blockWeights['maxBlock']
  const maxExtrinsic = blockWeights['perClass']['normal']['maxExtrinsic']
  console.log(`Normal refTime: ${maxExtrinsic.refTime} / ${maxBlock.refTime}`)

  console.log('===== Code Hash Deployment =====')

  console.log(`Deploying code hash: 'registry' ...`)
  const registry_data = await getDeploymentData('registry')
  const registry_hash = await uploadCode(api, account, registry_data.contract)
  console.log(`Registry hash: ${registry_hash}`)

  console.log(`Deploying code hash: 'share_token' ...`)
  const share_token_data = await getDeploymentData('share_token')
  const share_token_hash = await uploadCode(api, account, share_token_data.contract)
  console.log(`Share token hash: ${share_token_hash}`)

  console.log(`Deploying code hash: 'nomination_agent' ...`)
  const nomination_agent_data = await getDeploymentData('nomination_agent')
  const nomination_agent_hash = await uploadCode(api, account, nomination_agent_data.contract)
  console.log(`Hash: ${nomination_agent_hash}`)

  console.log('===== Contract Deployment =====')

  console.log(`Deploying contract: 'vault' ...`)
  const vault_data = await getDeploymentData('vault')
  const vault = await deployContract(
    api,
    account,
    vault_data.abi,
    vault_data.wasm,
    'new',
    [share_token_hash, registry_hash, nomination_agent_hash, eraDurationMs],
  )

  const vault_instance = new ContractPromise(api, vault_data.abi, vault.address)

  console.log('===== Address Lookup =====')

  console.log('Fetching registry contract ...')
  const registry_contract_result = await contractQuery(
    api,
    '',
    vault_instance,
    'iVault::get_registry_contract',
  )
  const registry = {
    address: decodeOutput(registry_contract_result, vault_instance, 'iVault::get_registry_contract').output,
    hash: registry_hash,
    block: vault.block,
    blockNumber: vault.blockNumber,
  } as DeployedContract
  const registry_instance = new ContractPromise(api, registry_data.abi, registry.address)
  console.log(`Registry Address: ${registry.address}`)

  console.log('Fetching share token contract ...')
  const share_token_contract_result = await contractQuery(
    api,
    '',
    vault_instance,
    'iVault::get_share_token_contract',
  )
  const share_token = {
    address: decodeOutput(share_token_contract_result, vault_instance, 'iVault::get_share_token_contract').output,
    hash: share_token_hash,
    block: vault.block,
    blockNumber: vault.blockNumber,
  } as DeployedContract
  console.log(`Share Token Address: ${share_token.address}`)

  console.log('===== Agent Configuration =====')

  for (const validator of validators) {
    console.log(`Adding nomination agent (validator: ${validator} ...`)
    await contractTx(
      api,
      account,
      registry_instance,
      'iRegistry::add_agent',
      {
        value: minNominatorBond,
      },
      [account.address, validator],
    )
  }

  console.log('Fetching agents ...')
  const get_agents_result = await contractQuery(
    api,
    '',
    registry_instance,
    'iRegistry::get_agents',
  )
  const [total_weight, agents] = decodeOutput(get_agents_result, registry_instance, 'iRegistry::get_agents').output

  console.log('Equally weighting agents ...')
  await contractTx(
    api,
    account,
    registry_instance,
    'iRegistry::update_agents',
    {},
    [agents.map((a) => ({ agent: a.address, weight: 10000, increase: true }))],
  )

  console.log('===== Contract Locations =====')

  console.log({
    vault: vault.address,
    registry: registry.address,
    share_token: share_token.address,
    ...agents.reduce((obj, a, i) => ({ ...obj, [`agent[${i}]`]: a.address }), {}),
  })

  console.log()

  // Write deployment artifacts into associated chainId subdirectory
  await copyArtifacts('nomination_agent', chainId)
  await copyArtifacts('registry', chainId)
  await copyArtifacts('share_token', chainId)
  await copyArtifacts('vault', chainId)

  // Write deployment metadata into associated chainId subdirectory
  await writeContractAddresses(chain.network, {
    vault,
    share_token,
    registry,
  })
}

main(process.env.VALIDATOR_ADDRESSES?.split(',') ?? [])
  .catch((error) => {
    console.error(error)
    process.exit(1)
  })
  .finally(() => process.exit(0))
