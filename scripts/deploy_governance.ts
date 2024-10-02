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

// Dynamic environment variables
const chainId = process.env.CHAIN || "development";
dotenv.config({
  path: `.env.${chainId}`,
});

async function main() {
  // Initialization
  const initParams = await initPolkadotJs();
  const { api, chain, account } = initParams;
  const gtoken_data = await getDeploymentData("governance_token");
  const governance = await getDeploymentData("governance");

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
  await deployContract(
    api,
    account,
    multisig.abi,
    multisig.wasm,
    "deploy_hash",
    []
  );
  await deployContract(
    api,
    account,
    gov_staking.abi,
    gov_staking.wasm,
    "governance_staking",
    []
  );
  await deployContract(
    api,
    account,
    gov_nft.abi,
    gov_nft.wasm,
    "deploy_hash",
    []
  );
  const vault = await getDeploymentData("vault");
  const registry = await getDeploymentData("registry");
  const exec_threshold = 10000;
  const reject_threshold = 10000;
  const acc_threshold = 1000000;
  const REWARDS_PER_SECOND = 100000;
  await deployContract(api, account, governance.abi, governance.wasm, "new", [
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
  ]);
}
