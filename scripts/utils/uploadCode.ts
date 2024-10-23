import { ApiPromise } from '@polkadot/api'
import { Abi } from '@polkadot/api-contract'
import { IKeyringPair } from '@polkadot/types/types'

export const uploadCode = async (
  api: ApiPromise,
  account: IKeyringPair | string,
  contractData: any
) => {
  return new Promise(async (resolve, reject) => {
    const abi = new Abi(contractData)
    await api.tx.contracts
      .uploadCode(abi.info.source.wasm, null, 0)
      .signAndSend(account, (result) => {
        if (result.isFinalized) {
          resolve(abi.info.source.wasmHash.toHex());
        }
        if (result.isError) {
          console.error(result.toHuman())
          reject(new Error(`Error uploading code`));
        }
      });
  });
}
