# Ike Contracts


### Installing Environment Pre-reqs
```bash
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
source $HOME/.cargo/env

cargo install --force --locked cargo-contract

rustup toolchain install stable-2023-12-28
rustup component add rust-src --toolchain stable-2023-12-28
```


### Building
After installing the pre-reqs above (especially the required stable toolchain), contracts must be compiled.
This is done with the [build-all.sh](./build-all.sh) script which can be run simply via the following command.
All contract artifacts will be saved in the [deployments](./deployments/) directory.
```bash
pnpm run build
```


### Testing (Integration Tests)
The integration tests are located in [contract_tests](drink_tests) and can be run simply via the following command.
```bash
pnpm test
```


### Deploying
Contract deployment is configured with the following environment variables.
* `ACCOUNT_URI` - deployer account
* `VALIDATOR_ADDRESSES` - comma separated list of validators used for nomination

```bash
pnpm run deploy
```
