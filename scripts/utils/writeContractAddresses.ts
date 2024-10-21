import { writeFile } from 'fs/promises'
import path from 'path'
import { deployContract } from "@scio-labs/use-inkathon";

/**
 * Writes each given contract address & blockNumber to a `{baseDir}/{contract}/{network}.ts` file.
 * NOTE: Base directory can be configured via the `DIR` environment variable
 */
export const writeContractAddresses = async (
  networkId: string,
  contractDeployments: Record<string, Awaited<ReturnType<typeof deployContract>>>,
  metadata?: { [key: string]: string | number },
) => {
  for (const [contractName, deployment] of Object.entries(contractDeployments)) {
    const contractNamePath = path.join(networkId, contractName, `deployment.ts`)
    const relativePath = path.join('deployments', contractNamePath)
    const absolutePath = path.join(path.resolve(), relativePath)

    let fileContents = ''

    if (deployment?.address) {
      fileContents += `export const address = '${deployment.address}'\n`
    }

    if (deployment?.blockNumber) {
      fileContents += `export const blockNumber = ${deployment.blockNumber}\n`
    }

    // Iterate over metadata keys and write them to the file
    if (metadata) {
      for (const [key, value] of Object.entries(metadata)) {
        const valueFormatted = typeof value === 'string' ? `'${value}'` : value
        fileContents += `export const ${key} = ${valueFormatted}\n`
      }
    }

    await writeFile(absolutePath, fileContents)
    console.log(`Exported deployment info to file: ${relativePath}`)
  }
}
