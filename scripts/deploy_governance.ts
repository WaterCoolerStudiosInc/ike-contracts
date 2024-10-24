import { ContractPromise } from "@polkadot/api-contract";
import {
  deployContract,
  contractTx,
  decodeOutput,
  contractQuery,
} from "@scio-labs/use-inkathon";
import * as dotenv from "dotenv";
import { getDeploymentData } from "./utils/getDeploymentData.js";
import { initPolkadotJs } from "./utils/initPolkadotJs.js";
import { writeContractAddresses } from "./utils/writeContractAddresses.js";
import { ApiPromise } from "@polkadot/api";
import { IKeyringPair } from "@polkadot/types/types";
// Dynamic environment variables
const chainId = process.env.CHAIN || "development";
dotenv.config({
  path: `.env.${chainId}`,
});
export const uploadCode = async (
  api: ApiPromise,
  account: IKeyringPair | string,
  abi: any,
  wasm: any
): Promise<string> => {
  await new Promise(async (resolve, reject) => {
    const unsub = await api.tx.contracts
      .uploadCode(wasm, null, 0)
      .signAndSend(account, (result) => {
        if (result.isFinalized) {
          unsub();
          resolve(result.txHash);
        }
        if (result.isError) {
          unsub();
          reject(result);
        }
      });
  });

  return abi.source.hash;
};
async function main() {
  // Initialization
  const initParams = await initPolkadotJs();
  const { api, chain, account } = initParams;
  const gtoken_data = await getDeploymentData("governance_token");
  const governance = await getDeploymentData("governance");
  console.log("===== Contract Deployment =====");
  const gtoken = await deployContract(
    api,
    account,
    gtoken_data.abi,
    gtoken_data.wasm,
    "new",
    []
  );
  const multisig = await getDeploymentData("multisig");
  const gov_staking = await getDeploymentData("governance_staking");
  const gov_nft = await getDeploymentData("governance_nft");
  console.log("===== MultiSig Deploy Hash =====");
  let sig_hash = await uploadCode(api, account, multisig.abi, multisig.wasm);
  console.log("===== Staking Deploy Hash =====");
  let gov_hash = await uploadCode(
    api,
    account,
    gov_staking.abi,
    gov_staking.wasm
  );
  console.log("===== NFT Deploy Hash =====");
  let nft_hash = await uploadCode(api, account, gov_nft.abi, gov_nft.wasm);
  const vault = await getDeploymentData("vault");
  const registry = await getDeploymentData("registry");
  console.log(vault.address);
  console.log(registry.address)
  const exec_threshold = 10000;
  const reject_threshold = 10000;
  const acc_threshold = 1000000;
  const REWARDS_PER_SECOND = 100000;
  console.log("===== GOVERNANCE CONTRACT DEPLOY =====");
  let result = await deployContract(
    api,
    account,
    governance.abi,
    governance.wasm,
    "new",
    [
      vault.address,
      registry.address,
      gtoken.address,
      multisig.abi.source.hash,
      gov_nft.abi.source.hash,
      gov_staking.abi.source.hash,
      exec_threshold,
      reject_threshold,
      acc_threshold,
      REWARDS_PER_SECOND,
    ]
  );
  console.log(result);
  
}

main()
  .catch((error) => {
    console.error(error);
    process.exit(1);
  })
  .finally(() => process.exit(0));
