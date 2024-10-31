import {ApiPromise} from "@polkadot/api"
import {ContractPromise} from "@polkadot/api-contract"
import {ContractCallOutcome} from "@polkadot/api-contract/types"
import {contractQuery, decodeOutput} from "@scio-labs/use-inkathon"
import dotenv from "dotenv"
import {getDeploymentData} from "./utils/getDeploymentData"
import {initPolkadotJs} from "./utils/initPolkadotJs"

const chainId = process.env.CHAIN || 'development'
dotenv.config({
  path: `.env.${chainId}`,
})

async function main() {
  // Initialization
  const initParams = await initPolkadotJs()
  const {api, chain, account} = initParams

  const nomination_agent_data = await getDeploymentData('nomination_agent', chainId)
  const registry_data = await getDeploymentData('registry', chainId)
  const registry = new ContractPromise(api, registry_data.abi, registry_data.address)

  const [totalWeight, agents] = await contractQueryAndDecode(api, registry, 'iRegistry::getAgents')

  const table = []
  for (const agent of agents) {
    const agent_contract = new ContractPromise(api, nomination_agent_data.abi, agent.address)

    const stakedValue = await contractQueryAndDecode(api, agent_contract, 'iNominationAgent::getStakedValue')
    const unbondingValue = await contractQueryAndDecode(api, agent_contract, 'iNominationAgent::getUnbondingValue')
    const validatorAddress = await contractQueryAndDecode(api, agent_contract, 'iNominationAgent::getValidator')
    const validatorCommission = (await api.query.staking.validators(validatorAddress)).toHuman()['commission']

    table.push({
      agent: agent.address,
      validator: validatorAddress,
      commission: validatorCommission,
      weight: agent.weight,
      stakedValue,
      unbondingValue,
    })
  }
  console.table(table)

  await api.disconnect()
}

async function contractQueryAndDecode(
  api: ApiPromise,
  instance: ContractPromise,
  methodName: string,
) {
  const result: ContractCallOutcome = await contractQuery(
    api,
    '',
    instance,
    methodName,
  )
  return decodeOutput(result, instance, methodName).output
}

main()
  .then(() => process.exit(0))
  .catch((e) => {
    console.error(e)
    process.exit(1)
  })
