# Kintsu Contracts

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

At least 2 nomination pools must exist on the target network prior to deployment.
Contract deployment is configured with the following environment variables.

- `CHAIN` - target chain name
- `ACCOUNT_URI` - deployer account

```bash
pnpm run deploy
```

### Note

The primary license for Kintsu sAZERO core is the Business Source License 1.1 (BUSL-1.1), see LICENSE. However, some files are dual licensed under GPL-2.0-or-later:

All files in contracts/interfaces/ may also be licensed under GPL-2.0-or-later (as indicated in their SPDX headers), see contracts/interfaces/LICENSE
