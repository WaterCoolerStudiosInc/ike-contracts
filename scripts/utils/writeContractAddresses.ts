import { deployContract } from '@scio-labs/use-inkathon'
import { mkdir, writeFile } from 'fs/promises'
import path from 'path'

/**
 * Writes each given contract address & blockNumber to a `deployments/{network}/{contract}/deployment.js` file.
 */
export const writeContractAddresses = async (
  networkId: string,
  contractDeployments: Record<string, Awaited<ReturnType<typeof deployContract>>>,
  metadata?: { [key: string]: string | number },
) => {
  for (const [contractName, deployment] of Object.entries(contractDeployments)) {
    const relativePath = path.join('deployments', networkId, contractName, `deployment.js`)
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

    // Ensure the destination directory exists
    await mkdir(path.dirname(absolutePath), { recursive: true });

    await writeFile(absolutePath, fileContents)
    console.log(`Exported deployment info to file: ${relativePath}`)
  }
}
