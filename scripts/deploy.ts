import { ContractPromise } from '@polkadot/api-contract'
import { deployContract, contractTx, decodeOutput, contractQuery } from '@scio-labs/use-inkathon'
import * as dotenv from 'dotenv'
import { getDeploymentData } from './utils/getDeploymentData.js'
import { initPolkadotJs } from './utils/initPolkadotJs.js'
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

  console.log('===== Code Hash Deployment =====')

  console.log(`Deploying code hash: 'registry' ...`)
  const registry_data = await getDeploymentData('registry')
  const registry = await deployContract(
    api,
    account,
    registry_data.abi,
    registry_data.wasm,
    'deploy_hash',
  )

  console.log(`Deploying code hash: 'share_token' ...`)
  const token_data = await getDeploymentData('share_token')
  const share_token = await deployContract(
    api,
    account,
    token_data.abi,
    token_data.wasm,
    'deploy_hash',
  )

  console.log(`Deploying code hash: 'nomination_agent' ...`)
  const nomination_agent_data = await getDeploymentData('nomination_agent')
  console.log(`Data hash: ${nomination_agent_data.abi.source.hash}`)
  const nomination_agent = await deployContract(
    api,
    account,
    nomination_agent_data.abi,
    nomination_agent_data.wasm,
    'deploy_hash',
  )
  console.log(`Hash: ${nomination_agent.hash}`)

  console.log('===== Contract Deployment =====')

  console.log(`Deploying contract: 'vault' ...`)
  const vault_data = await getDeploymentData('vault')
  const vault = await deployContract(
    api,
    account,
    vault_data.abi,
    vault_data.wasm,
    'new',
    [token_data.abi.source.hash, registry_data.abi.source.hash, nomination_agent_data.abi.source.hash, eraDurationMs],
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
  registry.address = decodeOutput(registry_contract_result, vault_instance, 'iVault::get_registry_contract').output
  const registry_instance = new ContractPromise(api, registry_data.abi, registry.address)
  console.log(`Registry Address: ${registry.address}`)

  console.log('Fetching share token contract ...')
  const share_token_contract_result = await contractQuery(
    api,
    '',
    vault_instance,
    'iVault::get_share_token_contract',
  )
  share_token.address = decodeOutput(share_token_contract_result, vault_instance, 'iVault::get_share_token_contract').output
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
    [agents.map((a) => a.address), [1000, 1000]],
  )

  console.log('===== Contract Locations =====')

  console.log({
    vault: vault.address,
    registry: registry.address,
    share_token: share_token.address,
    ...agents.reduce((obj, a, i) => ({...obj, [`agent[${i}]`]: a.address}), {}),
  })

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
