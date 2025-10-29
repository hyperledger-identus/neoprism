# Wait until wallet is up
GET {{ WALLET_BASE_URL }}/wallets
[Options]
delay: 5
retry: 10
HTTP 200

POST {{ WALLET_BASE_URL }}/wallets
{
  "name": "Test wallet",
  "mnemonic_sentence": [
    "mimic",
    "candy",
    "diamond",
    "virus",
    "hospital",
    "dragon",
    "culture",
    "price",
    "emotion",
    "tell",
    "update",
    "give",
    "faint",
    "resist",
    "faculty",
    "soup",
    "demand",
    "window",
    "dignity",
    "capital",
    "bullet",
    "purity",
    "practice",
    "fossil"
  ],
  "passphrase": "{{ WALLET_PASSPHRASE }}"
}
HTTP 201
[Captures]
wallet_id: jsonpath "$.id"

GET {{ WALLET_BASE_URL }}/wallets/{{ wallet_id }}/addresses
HTTP 200
[Captures]
address: jsonpath "$[0].id"
