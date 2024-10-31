import { copyFile, mkdir } from 'fs/promises'
import path from 'path'

export const copyArtifacts = async (contractName: string, chainId: string) => {
  if (chainId === 'development') return

  const sourceDir = path.join('deployments', 'development', contractName)
  const destinationDir = path.join('deployments', chainId, contractName)

  // Ensure the destination directory exists
  await mkdir(destinationDir, { recursive: true });

  await copyFile(
    path.join(sourceDir, `${contractName}.contract`),
    path.join(destinationDir, `${contractName}.contract`),
  )
  await copyFile(
    path.join(sourceDir, `${contractName}.json`),
    path.join(destinationDir, `${contractName}.json`),
  )
  await copyFile(
    path.join(sourceDir, `${contractName}.wasm`),
    path.join(destinationDir, `${contractName}.wasm`),
  )

  console.log(`Copied ${contractName} artifacts into ${destinationDir}${path.sep}`)
}
