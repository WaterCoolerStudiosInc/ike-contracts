import { readFile } from 'fs/promises'
import path from 'path'

/**
 * Reads the contract deployment files (wasm & abi).
 * NOTE: Base directory can be configured via the `DIR` environment variable
 */
export const getDeploymentData = async (contractName: string) => {
  const chainId = process.env.CHAIN || 'development'
  const contractPath = path.join(path.resolve(), 'deployments', chainId, contractName)

  let abi, wasm, contract
  try {
    abi = JSON.parse(await readFile(path.join(contractPath, `${contractName}.json`), 'utf-8'))
    contract = JSON.parse(await readFile(path.join(contractPath, `${contractName}.contract`), 'utf-8'))
    wasm = await readFile(path.join(contractPath, `${contractName}.wasm`))
  } catch (e) {
    console.error(e)
    throw new Error("Couldn't find contract deployment files. Did you build it via `pnpm build`?")
  }

  let address: string
  let blockNumber: number

  try {
    ({ address, blockNumber } = await import(path.join(contractPath, `deployment.ts`)))
  } catch (e) {}

  return {
    contractPath,
    abi,
    wasm,
    contract,
    address,
    blockNumber,
  }
}