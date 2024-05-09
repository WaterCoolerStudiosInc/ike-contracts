import { ApiPromise } from '@polkadot/api'
import { ContractPromise } from '@polkadot/api-contract'
import { ContractCallOutcome } from '@polkadot/api-contract/types'
import { contractQuery, decodeOutput } from '@scio-labs/use-inkathon'
import dotenv from 'dotenv'
import { getDeploymentData } from './utils/getDeploymentData'
import { initPolkadotJs } from './utils/initPolkadotJs'

const chainId = process.env.CHAIN || 'development'
dotenv.config({
  path: `.env.${chainId}`,
})

let debounceTimeoutID: any

async function main() {
  // Initialization
  const initParams = await initPolkadotJs()
  const { api, chain, account } = initParams

  const registry_data = await getDeploymentData('registry', chainId)
  const registry = new ContractPromise(api, registry_data.abi, registry_data.address)

  const poolMembersResult = await api.query.nominationPools.poolMembers.entries()

  const [totalWeight, agents] = await contractQueryAndDecode(api, registry, 'getAgents')

  //   TODO: we could also just initiliaze nomination agents and call getPoolId, determine whats better
  const table = []
  for (const agent of agents) {
    const poolMember = poolMembersResult.find((r) => r[0].toHuman()[0] === agent.address)
    const poolId = poolMember?.[1]?.toJSON()?.['poolId']

    const nominationPoolAddress = (
      await subscanRequest({ pool_id: poolId }, 'nomination_pool/pool')
    ).data.pool_account.address
    const nominationPoolVotedValidator = (
      await subscanRequest({ address: nominationPoolAddress }, 'staking/voted')
    ).data.list[0]
    const validatorAddress = nominationPoolVotedValidator.stash_account_display.address

    // if a display name is present use that. If there is a sub display name such as AZF/Shannon, use that. Else fall back to just the address
    const validatorName = nominationPoolVotedValidator.stash_account_display.parent?.display
      ? `${nominationPoolVotedValidator.stash_account_display.parent.display}${
          nominationPoolVotedValidator.stash_account_display.parent.sub_symbol
            ? `/${nominationPoolVotedValidator.stash_account_display.parent.sub_symbol}`
            : ''
        }`
      : validatorAddress

    const validatorCommission = (await api.query.staking.validators(validatorAddress)).toHuman()[
      'commission'
    ]
    table.push({
      validator: validatorName,
      commission: validatorCommission,
    })
  }

  console.log(table)
  await api.disconnect()
}

async function subscanRequest(body: any, endpoint: string) {
  const requestOptions = {
    method: 'POST',
    body: JSON.stringify(body),
  }
  const response = await fetch(
    `https://${chainId}.api.subscan.io/api/scan/${endpoint}`,
    requestOptions,
  )
  clearTimeout(debounceTimeoutID)
  await new Promise((resolve) => setTimeout(resolve, 400))
  return await response.json()
}

async function contractQueryAndDecode(
  api: ApiPromise,
  instance: ContractPromise,
  methodName: string,
) {
  const result: ContractCallOutcome = await contractQuery(api, '', instance, methodName)
  return decodeOutput(result, instance, methodName).output
}

main()
  .then(() => process.exit(0))
  .catch((e) => {
    console.error(e)
    process.exit(1)
  })
