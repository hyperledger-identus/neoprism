GET {{ WALLET_BASE_URL }}/wallets
HTTP 200
[Captures]
wallet_id: jsonpath "$[0].id"

# Wait until wallet receives balance from genesis transaction
GET {{ WALLET_BASE_URL }}/wallets/{{ wallet_id }}
[Options]
retry: 10
HTTP 200
[Asserts]
jsonpath "$.balance.available.quantity" > 0

GET {{ WALLET_BASE_URL }}/wallets/{{ wallet_id }}/addresses
HTTP 200
[Captures]
address: jsonpath "$[0].id"

POST {{ WALLET_BASE_URL }}/wallets/{{ wallet_id }}/transactions
{
  "passphrase": "{{ WALLET_PASSPHRASE }}",
  "payments": [
    {
      "address": "{{ address }}",
      "amount": {
        "quantity": 1000000,
        "unit": "lovelace"
      }
    }
  ],
  "metadata": {
    "21325": {
      "map": [
        { "k": { "string": "v" }, "v": { "int": 1 } },
        {
          "k": { "string": "c" },
          "v": {
            "list": [
              { "bytes": "22d50112d2010a086d61737465722d3012473045022100c921083f391f179c947a4e95f8ed226870c32557565f8adba52daebcf47ce5b3022019f8632237331c" },
              { "bytes": "5183d5ee6d192b617637848e32ce5d26c12dd2a86890b8bd041a7d0a7b0a79123c0a086d61737465722d3010014a2e0a09736563703235366b31122103b20404" },
              { "bytes": "f350d87eec98982131c176acfea520f26f8901fe08b619a56a0dd9e41712390a057664722d3010084a2e0a09736563703235366b31122102647aff70cfd5d510" },
              { "bytes": "ec369c512da85faef95803db30bb47499a28c08a590186ac" }
            ]
          }
        }
      ]
    }
  }
}
HTTP 202
