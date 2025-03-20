# Ike Contracts


### Installing Environment Pre-reqs
```bash
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
source $HOME/.cargo/env

cargo install --version 3.2.0 --force --locked cargo-contract

rustup toolchain install stable-2024-09-05
rustup component add rust-src --toolchain stable-2024-09-05
```


### Building

Install the package:
```bash
pnpm install
```

After installing the pre-reqs above (especially the required stable toolchain), contracts must be compiled.
This is done with the [build-all.sh](./build-all.sh) script which can be run simply via the following command.
All contract artifacts will be saved in the [deployments](./deployments/) directory.
```bash
pnpm run build
```


### Testing (Integration Tests)
The integration tests for the core protocol are located in [contract_tests](drink_tests) and can be run simply via the following command.
```bash
pnpm test
```

And to run the integration tests for the governance protocol (located in [governance_tests](governance_tests)), execute the following command.

```bash
pnpm test_governance
```


### Deploying

1. Set the target chain (default: development). Example:

```bash
export CHAIN=alephzero-testnet;
```

2. Setup the environment variables by creating a `.env.${CHAIN}` file like:

```bash
touch .env.alephzero-testnet
```

Contract deployment is configured with the following environment variables.
* `ACCOUNT_URI` - deployer account
* `VALIDATOR_ADDRESSES` - comma separated list of validators used for nomination

3. Deploy the core protocol

```bash
pnpm run deploy
```

4. Deploy the governance protocol

```bash
pnpm run deploy_governance
```
Note: ensure `pnpm run deploy` is invoked first.
