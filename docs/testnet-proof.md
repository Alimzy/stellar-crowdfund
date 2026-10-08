# Testnet proof

Produced by `scripts/demo-testnet.sh` on Stellar testnet with native XLM, on 2026-10-08.

| Item | Value |
|---|---|
| Network | Testnet |
| Contract ID | [`CAI2NXF6LH7TYONZ6QRCMZYKSWQYTFA56OUQU6TDODHXNCQJ3QO5CQKQ`](https://stellar.expert/explorer/testnet/contract/CAI2NXF6LH7TYONZ6QRCMZYKSWQYTFA56OUQU6TDODHXNCQJ3QO5CQKQ) |
| WASM sha256 | `8321b009b2faed2ab103cea5862da69a4dce36252f4d2b12c52f1db83e7c4b84` |
| Token | native XLM |

| Step | Result | Transaction |
|---|---|---|
| deploy | contract created | [`91689704d966...`](https://stellar.expert/explorer/testnet/tx/91689704d9669e7737d7b4ab686a16bde1608a8a57ef66e62934102453f01d30) |
| create_campaign 0 | goal 100 XLM | [`21dcf9fd017c...`](https://stellar.expert/explorer/testnet/tx/21dcf9fd017cbf3301e39e620f9c473efca229bf82639205ce26f8e3fb1adb52) |
| create_campaign 1 | goal 10,000 XLM | [`fee84571a85e...`](https://stellar.expert/explorer/testnet/tx/fee84571a85ef899189a985d88eb8090799beff35ec2fa1329d42cc366ef96ac) |
| contribute (Alice, campaign 0) | 60 XLM | [`2b3088ccb11c...`](https://stellar.expert/explorer/testnet/tx/2b3088ccb11c6f81c99f76cb84506842f178d538e0878858d6e6d4337cb67623) |
| contribute (Bob, campaign 0) | 70 XLM, total raised 130 XLM | [`f89220be5ee9...`](https://stellar.expert/explorer/testnet/tx/f89220be5ee92be8b536b323b3cb4729227cf04831ebbc188787a2570368fad2) |
| contribute (Bob, campaign 1) | 50 XLM | [`9507b51a3db9...`](https://stellar.expert/explorer/testnet/tx/9507b51a3db9fd93dba5058f30039b83bc7c70f17d231fee531e7720dc139ea2) |
| withdraw (creator, campaign 0) | 130 XLM paid to the creator | [`870910a6f20c...`](https://stellar.expert/explorer/testnet/tx/870910a6f20c750027b04057cab31f8911cc4ad144c523f7c147458366cfbf89) |
| refund (Bob, campaign 1) | 50 XLM returned, goal was missed | [`3749a159a896...`](https://stellar.expert/explorer/testnet/tx/3749a159a89639b80db4ac67cb3aaadea6f67054c8ec36d67ae0fcc0c776e041) |
