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

  const registry_data = await getDeploymentData('registry', chainId)
  const registry = new ContractPromise(api, registry_data.abi, registry_data.address)

  const poolMembersResult = await api.query.nominationPools.poolMembers.entries()

  const [totalWeight, agents] = await contractQueryAndDecode(api, registry, 'getAgents')

  const table = []
  for (const agent of agents) {
    const poolMember = poolMembersResult.find(r => r[0].toHuman()[0] === agent.address)
    const poolId = poolMember?.[1]?.toJSON()?.["poolId"]
    table.push({
      agent: agent.address,
      weight: agent.weight,
      pid: poolId,
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
