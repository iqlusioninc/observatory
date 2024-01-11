# observatory
[CometBFT](https://github.com/cometbft/cometbft) node monitoring that polls RPC endpoints for chainstate. 
## Run
First setup `observatory.toml`, check out [this full config example](https://github.com/iqlusioninc/observatory/blob/main/observatory.toml.example).
Fill in the config with your validator's consensus key address and the chains you want to monitor. There's an [optional Datadog config](https://github.com/iqlusioninc/observatory/blob/main/observatory.toml.example#L49) that integrates with Pagerduty.

```toml
[[chain]]
id = "cosmoshub-4"
validator_addr = "95E060D07713070FE9822F6C50BD76BCCBF9F17A"
rpc_urls = [
"https://cosmos-rpc.polkachu.com/",
"https://cosmoshub.validator.network/",
]
```

After you setup the config, run the following cmd `cargo run -- start`

## License

Copyright © 2023-2024 iqlusion

Licensed under the Apache License, Version 2.0 (the "License");
you may not use this file except in compliance with the License.
You may obtain a copy of the License at

    https://www.apache.org/licenses/LICENSE-2.0

Unless required by applicable law or agreed to in writing, software
distributed under the License is distributed on an "AS IS" BASIS,
WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
See the License for the specific language governing permissions and
limitations under the License.